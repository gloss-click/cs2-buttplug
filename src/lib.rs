#[macro_use]
extern crate log;

use std::{
    path::PathBuf, str::FromStr,
};

use anyhow::{Context, Error};
use csgo_gsi::GSIServer;
use fehler::throws;
use futures::{future::RemoteHandle, FutureExt, TryFutureExt};

mod buttplug;
pub mod config;
mod csgo;
mod script;
mod timer_thread;

use config::Config;
use tokio::{runtime::Handle, sync::broadcast, task::JoinHandle};

use crate::timer_thread::ScriptCommand;

const DEFAULT_GAME_DIR: &str = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Counter-Strike Global Offensive\\game\\csgo\\cfg";

pub type CloseEvent = csgo_gsi::CloseEvent;

#[throws]
fn spawn_tasks(close_receive: broadcast::Receiver<CloseEvent>, config: &Config, tokio_handle: Handle) -> (RemoteHandle<Result<(), Error>>, GSIServer, broadcast::Sender<ScriptCommand>, JoinHandle<()>, script::ScriptHost) {
    let (buttplug_send, buttplug_thread) = buttplug::spawn_run_thread(close_receive, &config.buttplug_server_url).context("couldn't start buttplug client")?;
    let gsi_server = csgo::build_server(config.cs_integration_port, match &config.cs_script_dir { Some(dir) => dir.clone(), None => PathBuf::from_str(DEFAULT_GAME_DIR).unwrap() })
        .map_err(|err| anyhow::anyhow!("{}", err)).context("couldn't set up CS integration server")?;
    let (event_proc_send, event_proc_thread) = timer_thread::spawn_timer_thread(tokio_handle, buttplug_send)?;
    let script_host = script::ScriptHost::new(event_proc_send.clone()).context("couldn't start script host")?;
    (buttplug_thread, gsi_server, event_proc_send, event_proc_thread, script_host)
}

#[throws]
pub async fn async_main(config: Config, tokio_handle: Handle, close_send: tokio::sync::broadcast::Sender<CloseEvent>) {
    match spawn_tasks(close_send.subscribe(), &config, tokio_handle.clone()) {
        Ok((buttplug_thread, mut gsi_server, event_proc_send, event_proc_thread, mut script_host)) => {
            gsi_server.add_listener(move |update| script_host.handle_update(update));
                                        
            let gsi_close_event_receiver = close_send.subscribe();
            
            let gsi_task_handle = gsi_server.run(tokio_handle.clone(), gsi_close_event_receiver).map_err(|err| anyhow::anyhow!("{}", err));

            let gsi_tokio_handle = tokio_handle.clone();
            let gsi_exit_handle = tokio_handle.spawn_blocking(move || gsi_tokio_handle.block_on(gsi_task_handle));
            
            info!("Initialised; waiting for exit");

            buttplug_thread.await.expect("Critical: Crashed stopping buttplug thread.");
            info!("Sending close event");
            close_send.send(CloseEvent{}).expect("Critical: Crashed sending close event.");
            info!("Closing GSI thread");
            gsi_exit_handle.await.unwrap().expect("Critical: Crashed stopping GSI server.");
            info!("Closing event processing thread.");
            event_proc_send.send(ScriptCommand::Close).expect("Critical: Crashed sending close to event processing thread.");

            event_proc_thread.await.expect("Critical: failed to join timer thread");
        },
        Err(e) => info!("Error : {}", e.to_string()),
    };

    Ok::<(), Error>(()).expect("Error ending main task")
}
