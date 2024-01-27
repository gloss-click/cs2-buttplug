use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub buttplug_server_url: String,
    pub cs_integration_port: u16,
    pub cs_script_dir: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            buttplug_server_url: "ws://127.0.0.1:12345".to_string(),
            cs_integration_port: 42069,
            cs_script_dir: None,
        }
    }
}