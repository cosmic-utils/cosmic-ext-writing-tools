// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    Application,
    cosmic_config::{self, CosmicConfigEntry},
};

use crate::{app::AppModel, config::Config};

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

    let config = cosmic_config::Config::new(AppModel::APP_ID, Config::VERSION)
        .map(|context| Config::get_entry(&context).unwrap_or_else(|(_, config)| config))
        .unwrap_or_default();

    let size = if config.setup_complete {
        cosmic::iced::Size::new(600.0, 450.0)
    } else {
        cosmic::iced::Size::new(600.0, 700.0)
    };

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default()
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(600.0)
                .min_height(450.0),
        )
        .size(size)
        .debug(false);

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}
