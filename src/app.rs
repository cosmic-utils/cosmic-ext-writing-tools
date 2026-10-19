// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::fl;
use crate::provider;
use crate::style::editor_style;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::cosmic_theme::Spacing;
use cosmic::iced::Alignment;
use cosmic::iced::Length;
use cosmic::prelude::*;
use cosmic::theme::spacing;
use cosmic::widget::{self, about::About, menu, text_editor};
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

pub struct AppModel {
    core: cosmic::Core,
    context_page: ContextPage,
    about: About,
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    config: Config,
    source: text_editor::Content<cosmic::Renderer>,
    selected_tool: WritingTool,
    tone: Tone,
    tone_labels: [String; 2],
    tool_model: widget::segmented_button::SingleSelectModel,
    previous_draft: Option<String>,
    status: String,
    is_processing: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    SourceEdited(text_editor::Action),
    SelectProvider(provider::Provider),
    UpdateEndpoint(String),
    UpdateModel(String),
    SelectTool(widget::segmented_button::Entity),
    SelectTone(Tone),
    ClearDraft,
    RunTool,
    AcceptSuggestion,
    RejectSuggestion,
    GenerationFinished(Result<String, String>),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.edfloreshz.WritingTools";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| Config::get_entry(&context).unwrap_or_else(|(_, config)| config))
            .unwrap_or_default();
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        let mut app = Self {
            core,
            context_page: ContextPage::default(),
            about,
            key_binds: HashMap::new(),
            config,
            source: text_editor::Content::new(),
            selected_tool: WritingTool::Rewrite,
            tone: Tone::Professional,
            tone_labels: Tone::labels(),
            tool_model: widget::segmented_button::Model::builder()
                .insert(|b| b.text(fl!("rewrite")).data(WritingTool::Rewrite).activate())
                .insert(|b| b.text(fl!("proofread")).data(WritingTool::Proofread))
                .insert(|b| b.text(fl!("summarize")).data(WritingTool::Summarize))
                .build(),
            previous_draft: None,
            status: fl!("ready"),
            is_processing: false,
        };
        let task = app.update_title();
        (app, task)
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![
            menu::bar(vec![menu::Tree::with_children(
                menu::root(fl!("view")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("settings"), None, MenuAction::Settings),
                        menu::Item::Button(fl!("about"), None, MenuAction::About),
                    ],
                ),
            )])
            .into(),
        ]
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
            ContextPage::Settings => context_drawer::context_drawer(
                self.settings_view(),
                Message::ToggleContextPage(ContextPage::Settings),
            )
            .title(fl!("settings")),
        })
    }

    fn footer(&self) -> Option<Element<'_, Self::Message>> {
        let Spacing {
            space_s, space_xxs, ..
        } = spacing();

        let can_run = !self.is_processing && self.previous_draft.is_none();

        let reject = if self.previous_draft.is_some() {
            widget::button::text(fl!("reject")).on_press(Message::RejectSuggestion)
        } else {
            widget::button::text(fl!("clear")).on_press(Message::ClearDraft)
        };
        let accept = if self.previous_draft.is_some() {
            widget::button::suggested(fl!("accept")).on_press(Message::AcceptSuggestion)
        } else {
            let label = if self.is_processing {
                fl!("working")
            } else {
                fl!("generate")
            };

            widget::button::suggested(label).on_press_maybe(can_run.then_some(Message::RunTool))
        };

        Some(
            widget::row::with_capacity(4)
                .push(widget::text::caption(&self.status))
                .push(widget::space::horizontal())
                .push(reject)
                .push(accept)
                .spacing(space_s)
                .padding([space_xxs, space_s])
                .align_y(Alignment::Center)
                .into(),
        )
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let Spacing {
            space_s, space_xxs, ..
        } = spacing();

        let header = widget::row::with_capacity(2)
            .push(widget::icon::from_name("edit-paste-symbolic").size(18))
            .push(widget::text::title3(fl!("writing-tools")))
            .push(widget::space::horizontal())
            .spacing(space_s)
            .align_y(Alignment::Center);

        let task_selector = widget::segmented_button::horizontal(&self.tool_model)
            .style(cosmic::theme::SegmentedButton::Control)
            .button_alignment(Alignment::Center)
            .button_height(36)
            .button_padding([12, 8, 12, 8])
            .button_spacing(space_xxs)
            .on_activate(Message::SelectTool);

        let tone_row = widget::row::with_capacity(3)
            .push(widget::text::body(fl!("tone")))
            .push(widget::dropdown(
                &self.tone_labels,
                Some(self.tone.index()),
                |index| Message::SelectTone(Tone::ALL[index]),
            ))
            .spacing(space_xxs)
            .align_y(Alignment::Center);

        let options_row = widget::row::with_capacity(2)
            .push(task_selector)
            .push(tone_row)
            .spacing(space_s)
            .align_y(Alignment::Center);

        let editor = text_editor::text_editor(&self.source)
            .placeholder(fl!("source-placeholder"))
            .style(editor_style)
            .padding(space_s)
            .height(Length::Fill)
            .on_action(Message::SourceEdited);

        let content = widget::column([])
            .push(header)
            .push(options_row)
            .push(editor)
            .spacing(space_s)
            .padding(space_s)
            .width(Length::Fill)
            .height(Length::Fill);

        content.into()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::LaunchUrl(url) => {
                if let Err(error) = open::that_detached(&url) {
                    self.status = format!("Could not open link: {error}");
                }
            }
            Message::ToggleContextPage(page) => {
                if self.context_page == page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = page;
                    self.core.window.show_context = true;
                }
            }
            Message::SourceEdited(action) => {
                if self.is_processing {
                    return Task::none();
                }
                if action.is_edit() && self.previous_draft.take().is_some() {
                    self.status = fl!("accepted");
                }
                self.source.perform(action);
            }
            Message::SelectProvider(provider) => self.save_config(Config {
                provider: match provider {
                    provider::Provider::Ollama => "ollama",
                    provider::Provider::LmStudio => "lm-studio",
                }
                .to_string(),
                endpoint: self.config.endpoint.clone(),
                model: self.config.model.clone(),
            }),
            Message::UpdateEndpoint(endpoint) => self.save_config(Config {
                provider: self.config.provider.clone(),
                endpoint,
                model: self.config.model.clone(),
            }),
            Message::UpdateModel(model) => self.save_config(Config {
                provider: self.config.provider.clone(),
                endpoint: self.config.endpoint.clone(),
                model,
            }),
            Message::SelectTool(id) => {
                self.tool_model.activate(id);
                if let Some(tool) = self.tool_model.data::<WritingTool>(id) {
                    self.selected_tool = *tool;
                }
                self.previous_draft = None;
            }
            Message::SelectTone(tone) => {
                self.tone = tone;
                self.previous_draft = None;
            }
            Message::ClearDraft => {
                self.source = text_editor::Content::new();
                self.previous_draft = None;
                self.status = fl!("ready");
            }
            Message::RunTool => {
                if self.previous_draft.is_some() || self.is_processing {
                    return Task::none();
                }
                let text = self.source.text();
                if text.trim().is_empty() {
                    self.status = fl!("enter-text");
                    return Task::none();
                }
                return self.generate(self.selected_tool.prompt(&text, self.tone));
            }
            Message::AcceptSuggestion => {
                if self.previous_draft.take().is_some() {
                    self.status = fl!("accepted");
                }
            }
            Message::RejectSuggestion => {
                if let Some(previous) = self.previous_draft.take() {
                    self.source = text_editor::Content::with_text(&previous);
                    self.status = fl!("rejected");
                }
            }
            Message::GenerationFinished(result) => {
                self.is_processing = false;
                match result {
                    Ok(text) if text.is_empty() => self.status = fl!("empty-response"),
                    Ok(text) => {
                        self.previous_draft = Some(self.source.text());
                        self.source = text_editor::Content::with_text(&text);
                        self.status = fl!("review-suggestion");
                    }
                    Err(error) => self.status = error,
                }
            }
        }
        Task::none()
    }
}

impl AppModel {
    fn settings_view(&self) -> Element<'_, Message> {
        let Spacing { space_s, .. } = spacing();
        let provider = provider::Provider::from_config(&self.config.provider);
        let provider_buttons = widget::row::with_capacity(2)
            .push(
                widget::button::text(fl!("ollama")).on_press_maybe(
                    (provider != provider::Provider::Ollama)
                        .then_some(Message::SelectProvider(provider::Provider::Ollama)),
                ),
            )
            .push(
                widget::button::text(fl!("lm-studio")).on_press_maybe(
                    (provider != provider::Provider::LmStudio)
                        .then_some(Message::SelectProvider(provider::Provider::LmStudio)),
                ),
            )
            .spacing(space_s);
        let help = match provider {
            provider::Provider::Ollama => fl!("ollama-help"),
            provider::Provider::LmStudio => fl!("lm-studio-help"),
        };

        widget::column::with_capacity(7)
            .push(widget::text::body(fl!("local-only-description")))
            .push(widget::text::heading(fl!("provider")))
            .push(provider_buttons)
            .push(widget::text::heading(fl!("model-endpoint")))
            .push(
                widget::text_input::text_input("", &self.config.endpoint)
                    .on_input(Message::UpdateEndpoint),
            )
            .push(widget::text::heading(fl!("model-name")))
            .push(
                widget::text_input::text_input("", &self.config.model)
                    .on_input(Message::UpdateModel),
            )
            .push(widget::text::caption(help))
            .spacing(space_s)
            .into()
    }

    fn save_config(&mut self, config: Config) {
        self.config = config;
        match cosmic_config::Config::new(<Self as cosmic::Application>::APP_ID, Config::VERSION) {
            Ok(context) => {
                if let Err(error) = self.config.write_entry(&context) {
                    self.status = format!("Could not save model settings: {error}");
                }
            }
            Err(error) => self.status = format!("Could not open model settings: {error}"),
        }
    }

    fn generate(&mut self, prompt: String) -> Task<cosmic::Action<Message>> {
        self.is_processing = true;
        self.status = fl!("working");
        Task::perform(
            provider::generate(
                provider::Provider::from_config(&self.config.provider),
                self.config.endpoint.clone(),
                self.config.model.clone(),
                prompt,
            ),
            Message::GenerationFinished,
        )
        .map(cosmic::Action::App)
    }

    fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        self.core
            .main_window_id()
            .map_or_else(Task::none, |id| self.set_window_title(fl!("app-title"), id))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WritingTool {
    Rewrite,
    Proofread,
    Summarize,
}

impl WritingTool {
    fn prompt(self, text: &str, tone: Tone) -> String {
        let instruction = match self {
            Self::Rewrite => "Rewrite the draft for clarity and flow while preserving its meaning.",
            Self::Proofread => {
                "Proofread the draft, correcting grammar, spelling, and punctuation while preserving the author's voice."
            }
            Self::Summarize => "Summarize the draft concisely while preserving its key points.",
        };
        let tone_instruction = match (self, tone) {
            (Self::Summarize, _) | (Self::Proofread, _) => "",
            (_, Tone::Friendly) => " Use a warm, friendly tone.",
            (_, Tone::Professional) => " Use a clear, professional tone.",
        };

        format!(
            "{instruction}{tone_instruction}\n\nReturn only the requested replacement text, without commentary or quotation marks.\n\nDraft:\n{text}"
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tone {
    Friendly,
    Professional,
}

impl Tone {
    const ALL: [Tone; 2] = [Tone::Friendly, Tone::Professional];

    fn labels() -> [String; 2] {
        [fl!("friendly"), fl!("professional")]
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
    Settings,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Self::About => Message::ToggleContextPage(ContextPage::About),
            Self::Settings => Message::ToggleContextPage(ContextPage::Settings),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WritingTool;

    #[test]
    fn writing_prompt_preserves_the_source_text() {
        let source = "A short draft.";
        let prompt = WritingTool::Rewrite.prompt(source, super::Tone::Friendly);

        assert!(prompt.contains("friendly tone"));
        assert!(prompt.ends_with(source));
    }
}
