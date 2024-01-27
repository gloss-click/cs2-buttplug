#[macro_use]
extern crate log;

use std::{env::current_exe, fs::{read_to_string, write}, io::stdin};

use anyhow::{Context, Error};
use fehler::throws;

use cs2_buttplug::{async_main, config::Config};

pub fn wait_for_enter() {
    let mut ignored = String::new();
    let _ = stdin().read_line(&mut ignored);
}

#[throws]
pub fn get_config() -> Config {
    let exe_path = current_exe().context("couldn't get path of cs2-buttplug.exe")?;
    let config_path = exe_path.with_extension("toml");
    if !config_path.exists() {
        info!("Creating config file {} with default settings, go look over those settings and then come back and press Enter", config_path.display());
        let default_config = Config::default();
        let default_config = toml::to_string_pretty(&default_config).context("couldn't build default config file")?;
        write(&config_path, default_config).context("couldn't save default config file")?;
    }

    let config_text = read_to_string(&config_path).context("couldn't read config file")?;
    toml::from_str(&config_text).context("couldn't parse config file")?
}

#[throws]
fn inner_main() {
    info!("This is cs2-buttplug (cli), v{}, original author hornycactus (https://cactus.sexy)", env!("CARGO_PKG_VERSION"));

    let config = get_config()?;

    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    let handle = tokio_runtime.handle().clone();
    let (close_send, _close_receive) = tokio::sync::broadcast::channel(64);
    let _result: core::result::Result<(), Error> = tokio_runtime.block_on(async {
        async_main(config, handle, close_send).await
    });

}

fn main() { 
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter(Some("cs2_buttplug"), log::LevelFilter::max())
        .filter(Some("cs2_buttplug_cli"), log::LevelFilter::max())
        .filter(Some("csgo_gsi"), log::LevelFilter::max())
        .init();

    if let Err(error) = inner_main() {
        error!("{}", error);
        println!("\nPress Enter to exit...");
        wait_for_enter();
    }
}
