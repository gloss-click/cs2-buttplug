#[macro_use]
extern crate log;

use std::{
    future::IntoFuture, path::PathBuf, str::FromStr
};

use anyhow::{Context, Error};
use buttplug::BPCommand;
use csgo_gsi::GSIServer;
use futures::{future::RemoteHandle, TryFutureExt};

mod buttplug;
pub use buttplug::{ClientEvent, GuiEvent};
pub mod config;
mod csgo;
mod script;
mod timer_thread;

use config::Config;
use tokio::{runtime::Handle, sync::broadcast, task::JoinHandle};

use crate::timer_thread::ScriptCommand;

const DEFAULT_GAME_DIR: &str = "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Counter-Strike Global Offensive\\game\\csgo\\cfg";

pub type CloseEvent = csgo_gsi::CloseEvent;

pub async fn spawn_buttplug_client(buttplug_server_url: &String, close_receive: broadcast::Receiver<CloseEvent>, client_send: Option<broadcast::Sender<ClientEvent>>, gui_receive: Option<broadcast::Receiver<GuiEvent>>) -> Result<(broadcast::Sender<BPCommand>, RemoteHandle<()>), Error> {
    buttplug::spawn_run_thread(close_receive, &buttplug_server_url, client_send, gui_receive).context("couldn't start buttplug client")
}

fn spawn_tasks(config: &Config, tokio_handle: Handle, buttplug_send: broadcast::Sender<BPCommand>) -> Result<(GSIServer, broadcast::Sender<ScriptCommand>, JoinHandle<()>, script::ScriptHost), Error> {
    let gsi_server = csgo::build_server(config.cs_integration_port, match &config.cs_script_dir { Some(dir) => dir.clone(), None => PathBuf::from_str(DEFAULT_GAME_DIR).unwrap() })
        .map_err(|err| anyhow::anyhow!("{}", err)).context("couldn't set up CS integration server")?;
    let (event_proc_send, event_proc_thread) = timer_thread::spawn_timer_thread(tokio_handle, buttplug_send)?;
    let script_host = script::ScriptHost::new(event_proc_send.clone(), &config.rhai_script_path).context("couldn't start script host")?;
    Ok((gsi_server, event_proc_send, event_proc_thread, script_host))
}

pub async fn async_main_with_buttplug(config: Config, tokio_handle: Handle, close_send: broadcast::Sender<CloseEvent>) -> Result<(), Error> {
    //let tokio_handle2 = tokio_handle.clone();
    let (buttplug_send, buttplug_thread) = spawn_buttplug_client(&config.buttplug_server_url, close_send.subscribe(), None, None).await.unwrap();
    let main_handle = async_main(config, tokio_handle, close_send, buttplug_send, None::<fn(&csgo_gsi::Update)>);
    main_handle.await;
    buttplug_thread.into_future().await;
    Ok(())
}

pub async fn async_main<T: 'static + FnMut(&csgo_gsi::Update) + Send>(config: Config, tokio_handle: Handle, close_send: broadcast::Sender<CloseEvent>, buttplug_send: broadcast::Sender<BPCommand>, gsi_listener: Option<T>) -> Result<(), Error> {
    match spawn_tasks(&config, tokio_handle.clone(), buttplug_send) {
        Ok((mut gsi_server, event_proc_send, event_proc_thread, mut script_host)) => {
            gsi_server.add_listener(move |update| script_host.handle_update(update));

            if let Some(mut listener) = gsi_listener {
                gsi_server.add_listener(move |update| listener(update));
            }
                                        
            let gsi_close_event_receiver = close_send.subscribe();
            let mut main_close_event_receiver = close_send.subscribe();
            
            let gsi_task_handle = gsi_server.run(tokio_handle.clone(), gsi_close_event_receiver).map_err(|err| anyhow::anyhow!("{}", err));

            let gsi_tokio_handle = tokio_handle.clone();
            let gsi_exit_handle = tokio_handle.spawn_blocking(move || gsi_tokio_handle.block_on(gsi_task_handle));
            
            info!("Initialised; waiting for exit");
            main_close_event_receiver.recv().await.expect("Critical: Crashed waiting for close event.");
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

    Ok(())
}
