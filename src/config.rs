// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 1]
pub struct Config {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub setup_complete: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            endpoint: "http://127.0.0.1:11434".to_string(),
            model: "llama3.2".to_string(),
            setup_complete: false,
        }
    }
}
