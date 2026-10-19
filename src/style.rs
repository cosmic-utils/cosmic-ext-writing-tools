use cosmic::iced::{Border, Color};
use cosmic::widget;
use cosmic::widget::text_editor::Status;

pub fn editor_style(theme: &cosmic::Theme, status: Status) -> widget::text_editor::Style {
    let cosmic = theme.cosmic();
    let container = theme.current_container();

    let mut background: Color = container.small_widget.into();
    background.a = 0.25;

    let border_color: Color = match status {
        Status::Active | Status::Disabled => container.component.divider.into(),
        Status::Hovered | Status::Focused { .. } => cosmic.accent.base.into(),
    };

    let mut placeholder: Color = container.on.into();
    placeholder.a = 0.7;

    let mut selection: Color = cosmic.accent_color().into();
    selection.a = 0.4;

    widget::text_editor::Style {
        background: background.into(),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: cosmic.corner_radii.radius_s.into(),
        },
        placeholder,
        value: container.on.into(),
        selection,
    }
}
