#[macro_use]
extern crate log;

use std::{
    env::current_exe,
    fs::{read_to_string, write},
    io::stdin,
};

use anyhow::{Context, Error};
use fehler::throws;
use futures::TryFutureExt;

mod buttplug;
mod config;
mod csgo;
mod script;
mod version;

fn wait_for_enter() {
    let mut ignored = String::new();
    let _ = stdin().read_line(&mut ignored);
}

#[throws]
fn get_config() -> config::Config {
    let exe_path = current_exe().context("couldn't get path of Crotch-Stim: Get Off executable")?;
    let config_path = exe_path.with_extension("toml");
    if !config_path.exists() {
        println!("Creating config file {} with default settings, go look over those settings and then come back and press Enter", config_path.display());
        let default_config = config::Config::default();
        let default_config = toml::to_string_pretty(&default_config).context("couldn't build default config file")?;
        write(&config_path, default_config).context("couldn't save default config file")?;
        wait_for_enter();
    }

    let config_text = read_to_string(&config_path).context("couldn't read config file")?;
    toml::from_str(&config_text).context("couldn't parse config file")?
}

#[throws]
fn inner_main() {
    println!("This is Crotch-Stim: Get Off, v{}, by hornycactus (https://cactus.sexy)", env!("CARGO_PKG_VERSION"));

    version::check_for_updates();

    let config = get_config()?;

    let (buttplug_send, buttplug_thread) = buttplug::spawn_run_thread(&config.buttplug_server_url).context("couldn't start buttplug client")?;

    let mut tokio_runtime = tokio::runtime::Runtime::new().unwrap();

    let mut gsi_server = csgo::build_server(config.csgo_integration_port)
        .map_err(|err| anyhow::anyhow!("{}", err))
        .context("couldn't set up CS:GO integration server")?;

    let mut script_host = script::ScriptHost::new(buttplug_send)
        .context("couldn't start script host")?;
    gsi_server.add_listener(move |update| script_host.handle_update(update));

    info!("probably started threads?");

    let result = tokio_runtime.block_on(async {
        tokio::try_join!(
            gsi_server.run().map_err(|err| anyhow::anyhow!("{}", err)),
            buttplug_thread,
        )
    })?;

    dbg!(&result);
}

fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Warn)
        .filter(Some("crotch_stim_get_off"), log::LevelFilter::max())
        .init();

    if let Err(error) = inner_main() {
        eprintln!("{}", error);
        println!("\nPress Enter to exit...");
        wait_for_enter();
    }
}
