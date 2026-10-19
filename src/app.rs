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
use cosmic::iced::Size;
use cosmic::iced::window;
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
    app_icon: widget::icon::Handle,
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
    setup_step: Option<SetupStep>,
    connection: ConnectionState,
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
    CopyDraft,
    AcceptSuggestion,
    RejectSuggestion,
    GenerationFinished(Result<String, String>),
    SetupNext,
    SetupBack,
    SetupSkip,
    SetupFinish,
    TestConnection,
    ConnectionTested(Result<(), String>),
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
        let app_icon = widget::icon::from_svg_bytes(APP_ICON);
        let about = About::default()
            .name(fl!("app-title"))
            .author("Eduardo Flores")
            .icon(app_icon.clone())
            .version(env!("CARGO_PKG_VERSION"))
            .license(env!("CARGO_PKG_LICENSE"))
            .links([
                (fl!("repository"), REPOSITORY.to_string()),
                (fl!("support"), format!("{}/issues", REPOSITORY)),
            ])
            .developers([("Eduardo Flores", "edfloreshz@gmail.com")]);

        let setup_step = (!config.setup_complete).then_some(SetupStep::Welcome);

        let mut app = Self {
            core,
            context_page: ContextPage::default(),
            about,
            app_icon,
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
            setup_step,
            connection: ConnectionState::Untested,
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
        if let Some(step) = self.setup_step {
            return Some(self.setup_footer(step));
        }

        let Spacing {
            space_s, space_xxs, ..
        } = spacing();

        let can_run = !self.is_processing && self.previous_draft.is_none();

        let reject = self
            .previous_draft
            .is_some()
            .then_some(widget::button::text(fl!("reject")).on_press(Message::RejectSuggestion));
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
                .push_maybe(reject)
                .push(accept)
                .spacing(space_s)
                .padding([space_xxs, space_s])
                .align_y(Alignment::Center)
                .into(),
        )
    }

    fn view(&self) -> Element<'_, Self::Message> {
        if let Some(step) = self.setup_step {
            return self.setup_view(step);
        }

        let Spacing {
            space_s,
            space_xxs,
            space_xl,
            ..
        } = spacing();

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
            .padding([space_xxs, space_xl, space_xxs, space_xxs])
            .height(Length::Fill)
            .on_action(Message::SourceEdited);

        let actions = widget::container(
            widget::row([]).push(widget::space::horizontal()).push(
                widget::column([])
                    .push(widget::tooltip(
                        widget::button::icon(widget::icon::from_name("edit-clear-symbolic"))
                            .on_press(Message::ClearDraft),
                        widget::text(fl!("clear")),
                        widget::tooltip::Position::Left,
                    ))
                    .push(widget::space::vertical())
                    .push(widget::tooltip(
                        widget::button::icon(widget::icon::from_name("edit-copy-symbolic"))
                            .on_press(Message::CopyDraft),
                        widget::text(fl!("copy")),
                        widget::tooltip::Position::Left,
                    )),
            ),
        )
        .padding(space_s);

        let stack = cosmic::iced::widget::stack![editor, actions];

        let content = widget::column([])
            .push(options_row)
            .push(stack)
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
            Message::SelectProvider(new) => {
                let old = provider::Provider::from_config(&self.config.provider);
                if old != new {
                    self.update_config(|config| {
                        if config.endpoint == default_endpoint(&old) {
                            config.endpoint = default_endpoint(&new).to_string();
                        }
                        if config.model == default_model(&old) {
                            config.model = default_model(&new).to_string();
                        }
                        config.provider = provider_key(&new).to_string();
                    });
                    self.connection = ConnectionState::Untested;
                }
            }
            Message::UpdateEndpoint(endpoint) => {
                self.update_config(|config| config.endpoint = endpoint);
                self.connection = ConnectionState::Untested;
            }
            Message::UpdateModel(model) => {
                self.update_config(|config| config.model = model);
                self.connection = ConnectionState::Untested;
            }
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
            Message::SetupNext => {
                if let Some(step) = self.setup_step {
                    if self.can_advance(step) {
                        if let Some(next) = step.next() {
                            self.setup_step = Some(next);
                        }
                    }
                }
            }
            Message::SetupBack => {
                if let Some(previous) = self.setup_step.and_then(SetupStep::previous) {
                    self.setup_step = Some(previous);
                }
            }
            Message::SetupSkip | Message::SetupFinish => {
                self.complete_setup();
                if let Some(id) = self.core.main_window_id() {
                    return window::resize(id, Size::new(600.0, 450.0));
                }
            }
            Message::TestConnection => {
                if self.connection == ConnectionState::Testing {
                    return Task::none();
                }
                self.connection = ConnectionState::Testing;

                let provider = provider::Provider::from_config(&self.config.provider);
                let endpoint = self.config.endpoint.clone();
                let model = self.config.model.clone();
                return Task::perform(
                    async move {
                        provider::generate(
                            provider,
                            endpoint,
                            model,
                            "Reply with the single word OK.".to_string(),
                        )
                        .await
                        .map(|_| ())
                    },
                    Message::ConnectionTested,
                )
                .map(cosmic::Action::App);
            }
            Message::ConnectionTested(result) => {
                if self.connection == ConnectionState::Testing {
                    self.connection = match result {
                        Ok(()) => ConnectionState::Connected,
                        Err(error) => ConnectionState::Failed(error),
                    };
                }
            }
            Message::CopyDraft => {
                let text = self.source.text();

                if !text.is_empty() {
                    return cosmic::iced::clipboard::write(text).map(cosmic::Action::App);
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

    /// Applies `apply` to a copy of the current config and persists the result.
    fn update_config(&mut self, apply: impl FnOnce(&mut Config)) {
        let mut config = self.config.clone();
        apply(&mut config);
        self.save_config(config);
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

    fn can_advance(&self, step: SetupStep) -> bool {
        match step {
            SetupStep::Model => self.connection == ConnectionState::Connected,
            _ => true,
        }
    }

    fn complete_setup(&mut self) {
        self.setup_step = None;
        self.status = fl!("ready");
        self.update_config(|config| config.setup_complete = true);
    }

    fn setup_footer(&self, step: SetupStep) -> Element<'_, Message> {
        let Spacing {
            space_s, space_xxs, ..
        } = spacing();

        let is_last = step.next().is_none();

        let current = step.index() + 1;
        let mut row = widget::row::with_capacity(5)
            .push(widget::text::caption(fl!(
                "setup-step",
                current = current,
                total = SetupStep::ALL.len()
            )))
            .push(widget::space::horizontal());

        if !is_last {
            row = row.push(widget::button::text(fl!("setup-skip")).on_press(Message::SetupSkip));
        }
        if step.previous().is_some() {
            row =
                row.push(widget::button::standard(fl!("setup-back")).on_press(Message::SetupBack));
        }

        let forward = if is_last {
            widget::button::suggested(fl!("setup-finish")).on_press(Message::SetupFinish)
        } else {
            let label = match step {
                SetupStep::Welcome => fl!("setup-get-started"),
                _ => fl!("setup-next"),
            };
            widget::button::suggested(label)
                .on_press_maybe(self.can_advance(step).then_some(Message::SetupNext))
        };

        row.push(forward)
            .spacing(space_s)
            .padding([space_xxs, space_s])
            .align_y(Alignment::Center)
            .into()
    }

    fn setup_view(&self, step: SetupStep) -> Element<'_, Message> {
        let Spacing {
            space_l, space_xxs, ..
        } = spacing();

        let page = match step {
            SetupStep::Welcome => self.welcome_page(),
            SetupStep::Provider => self.provider_page(),
            SetupStep::Model => self.model_page(),
            SetupStep::Done => self.done_page(),
        };

        widget::container(page)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(space_l)
            .apply(widget::scrollable)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(space_xxs)
            .into()
    }

    fn setup_page<'a>(
        &'a self,
        show_icon: bool,
        title: String,
        description: String,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let Spacing {
            space_s, space_m, ..
        } = spacing();

        let mut page = widget::column::with_capacity(4).spacing(space_m);
        if show_icon {
            page = page.push(widget::icon::icon(self.app_icon.clone()).size(96));
        }

        page.push(
            widget::column::with_capacity(2)
                .push(widget::text::title2(title))
                .push(widget::text::body(description))
                .spacing(space_s),
        )
        .push(content)
        .into()
    }

    fn welcome_page(&self) -> Element<'_, Message> {
        let Spacing { space_xs, .. } = spacing();

        let features = widget::column::with_capacity(4)
            .push(feature_row(
                "security-high-symbolic",
                fl!("setup-feature-private"),
            ))
            .push(feature_row(
                "document-edit-symbolic",
                fl!("setup-feature-rewrite"),
            ))
            .push(feature_row(
                "emblem-ok-symbolic",
                fl!("setup-feature-proofread"),
            ))
            .push(feature_row(
                "view-list-symbolic",
                fl!("setup-feature-summarize"),
            ))
            .spacing(space_xs);

        self.setup_page(
            true,
            fl!("setup-welcome-title"),
            fl!("setup-welcome-body"),
            features.into(),
        )
    }

    fn provider_page(&self) -> Element<'_, Message> {
        let Spacing { space_s, .. } = spacing();
        let selected = provider::Provider::from_config(&self.config.provider);

        let choice = |label: String, value: provider::Provider| {
            let button = if selected == value {
                widget::button::suggested(label)
            } else {
                widget::button::standard(label)
            };
            button
                .on_press(Message::SelectProvider(value))
                .width(Length::Fill)
        };

        let help = match selected {
            provider::Provider::Ollama => fl!("ollama-help"),
            provider::Provider::LmStudio => fl!("lm-studio-help"),
        };

        let content = widget::column::with_capacity(2)
            .push(
                widget::row::with_capacity(2)
                    .push(choice(fl!("ollama"), provider::Provider::Ollama))
                    .push(choice(fl!("lm-studio"), provider::Provider::LmStudio))
                    .spacing(space_s),
            )
            .push(widget::text::caption(help))
            .spacing(space_s);

        self.setup_page(
            false,
            fl!("setup-provider-title"),
            fl!("setup-provider-body"),
            content.into(),
        )
    }

    fn model_page(&self) -> Element<'_, Message> {
        let Spacing {
            space_s, space_xxs, ..
        } = spacing();
        let provider = provider::Provider::from_config(&self.config.provider);

        let can_test = !self.config.endpoint.trim().is_empty()
            && !self.config.model.trim().is_empty()
            && self.connection != ConnectionState::Testing;
        let test = widget::button::standard(fl!("setup-test"))
            .on_press_maybe(can_test.then_some(Message::TestConnection));

        let status: Element<'_, Message> = match &self.connection {
            ConnectionState::Untested => widget::text::caption(fl!("setup-test-hint")).into(),
            ConnectionState::Testing => widget::text::caption(fl!("setup-testing")).into(),
            ConnectionState::Connected => widget::text::caption(fl!("setup-connected")).into(),
            ConnectionState::Failed(error) => widget::text::caption(error.as_str()).into(),
        };

        let content = widget::column::with_capacity(6)
            .push(
                widget::column::with_capacity(2)
                    .push(widget::text::heading(fl!("model-endpoint")))
                    .push(
                        widget::text_input::text_input(
                            default_endpoint(&provider),
                            &self.config.endpoint,
                        )
                        .on_input(Message::UpdateEndpoint),
                    )
                    .spacing(space_xxs),
            )
            .push(
                widget::column::with_capacity(2)
                    .push(widget::text::heading(fl!("model-name")))
                    .push(
                        widget::text_input::text_input(
                            default_model(&provider),
                            &self.config.model,
                        )
                        .on_input(Message::UpdateModel),
                    )
                    .spacing(space_xxs),
            )
            .push(
                widget::row::with_capacity(2)
                    .push(test)
                    .push(status)
                    .spacing(space_s)
                    .align_y(Alignment::Center),
            )
            .spacing(space_s);

        self.setup_page(
            false,
            fl!("setup-model-title"),
            fl!("setup-model-body"),
            content.into(),
        )
    }

    fn done_page(&self) -> Element<'_, Message> {
        let Spacing { space_s, .. } = spacing();
        let provider = provider::Provider::from_config(&self.config.provider);
        let provider_name = match provider {
            provider::Provider::Ollama => fl!("ollama"),
            provider::Provider::LmStudio => fl!("lm-studio"),
        };

        let summary = widget::list_column()
            .add(widget::settings::item(
                fl!("provider"),
                widget::text::body(provider_name),
            ))
            .add(widget::settings::item(
                fl!("model-endpoint"),
                widget::text::body(&self.config.endpoint),
            ))
            .add(widget::settings::item(
                fl!("model-name"),
                widget::text::body(&self.config.model),
            ));

        let content = widget::column::with_capacity(2)
            .push(summary)
            .push(widget::text::caption(fl!("setup-done-hint")))
            .spacing(space_s);

        self.setup_page(
            true,
            fl!("setup-done-title"),
            fl!("setup-done-body"),
            content.into(),
        )
    }
}

fn feature_row<'a>(icon_name: &'static str, label: String) -> Element<'a, Message> {
    let Spacing { space_xs, .. } = spacing();

    widget::row::with_capacity(2)
        .push(widget::icon::from_name(icon_name).size(20))
        .push(widget::text::body(label))
        .spacing(space_xs)
        .align_y(Alignment::Center)
        .into()
}

fn provider_key(provider: &provider::Provider) -> &'static str {
    match provider {
        provider::Provider::Ollama => "ollama",
        provider::Provider::LmStudio => "lm-studio",
    }
}

fn default_endpoint(provider: &provider::Provider) -> &'static str {
    match provider {
        provider::Provider::Ollama => "http://127.0.0.1:11434",
        provider::Provider::LmStudio => "http://127.0.0.1:1234",
    }
}

fn default_model(provider: &provider::Provider) -> &'static str {
    match provider {
        provider::Provider::Ollama => "llama3.2",
        provider::Provider::LmStudio => "",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupStep {
    Welcome,
    Provider,
    Model,
    Done,
}

impl SetupStep {
    const ALL: [SetupStep; 4] = [
        SetupStep::Welcome,
        SetupStep::Provider,
        SetupStep::Model,
        SetupStep::Done,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }

    fn next(self) -> Option<Self> {
        Self::ALL.get(self.index() + 1).copied()
    }

    fn previous(self) -> Option<Self> {
        self.index().checked_sub(1).map(|index| Self::ALL[index])
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ConnectionState {
    #[default]
    Untested,
    Testing,
    Connected,
    Failed(String),
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
    use super::{SetupStep, WritingTool};

    #[test]
    fn writing_prompt_preserves_the_source_text() {
        let source = "A short draft.";
        let prompt = WritingTool::Rewrite.prompt(source, super::Tone::Friendly);

        assert!(prompt.contains("friendly tone"));
        assert!(prompt.ends_with(source));
    }

    #[test]
    fn setup_steps_walk_forward_and_back() {
        let mut step = SetupStep::Welcome;
        let mut visited = vec![step];
        while let Some(next) = step.next() {
            step = next;
            visited.push(step);
        }

        assert_eq!(visited, SetupStep::ALL);
        assert_eq!(step, SetupStep::Done);
        assert_eq!(SetupStep::Welcome.previous(), None);
        assert_eq!(SetupStep::Done.previous(), Some(SetupStep::Model));
    }
}
