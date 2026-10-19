// SPDX-License-Identifier: GPL-3.0-only

mod app;
mod config;
mod i18n;
mod provider;
mod style;

fn main() -> cosmic::iced::Result {
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(600.0)
                .min_height(450.0),
        )
        .size(cosmic::iced::Size::new(600.0, 700.0))
        .debug(false);

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
