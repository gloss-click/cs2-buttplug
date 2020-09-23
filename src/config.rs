use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub buttplug_server_url: Option<String>,
    pub csgo_integration_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            buttplug_server_url: None,
            csgo_integration_port: 42069,
        }
    }
}
