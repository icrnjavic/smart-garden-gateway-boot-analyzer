use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Default)]
pub struct Config {
    pub serial_port: String,
}

impl Config {
    /// # Panics
    ///
    /// Panics if something unexpected happens.
    #[must_use]
    pub fn new() -> Config {
        Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(Config::file_path()))
            .extract()
            .expect("Failed to construct Config")
    }

    /// # Panics
    ///
    /// Panics if something unexpected happens.
    pub fn save(&self) {
        let toml = toml::to_string(&self).expect("Failed to serialize config");
        let config_file_path = Config::file_path();
        std::fs::create_dir_all(
            config_file_path
                .parent()
                .expect("Failed to create config directory"),
        )
        .expect("Failed to create config directory");
        let mut f = File::create(config_file_path).expect("Failed to create config file");
        f.write_all(toml.as_bytes())
            .expect("Failed to write config");
    }

    /// # Panics
    ///
    /// Panics if something unexpected happens.
    fn file_path() -> PathBuf {
        dirs::config_dir()
            .expect("Failed to get config dir")
            .join("Husqvarna")
            .join("smart-garden-gateway-boot-analyzer")
            .join("config.toml")
    }
}
