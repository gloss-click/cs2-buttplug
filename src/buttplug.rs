use std::collections::{HashMap, HashSet};

use anyhow::{Context, Error};
use buttplug::{
    client::{
        ButtplugClient,
        ButtplugClientEvent, device::{LinearCommand, ScalarValueCommand}
    },
    core::{connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport}, message::serializer::ButtplugClientJSONSerializer},
    util::{async_manager, in_process_client}
};
use futures::{future::RemoteHandle, StreamExt};
use tokio::sync::broadcast;

use crate::CloseEvent;

#[derive(Clone)]
pub enum ClientEvent {
    DeviceFound(String, u32),
    DeviceLost(String, u32)
}

#[derive(Clone)]
pub enum GuiEvent {
    DeviceToggle(u32, bool),
    None,
}

#[derive(Copy, Clone)]
pub enum BPCommand {
    Vibrate(f64),
    VibrateIndex(f64, u32),
    Linear(u32, f64),
    Stop
}


async fn run_buttplug_catch(
    client_send: Option<tokio::sync::broadcast::Sender<ClientEvent>>, 
    gui_receive: Option<broadcast::Receiver<GuiEvent>>,
    close_receive: broadcast::Receiver<CloseEvent>, 
    client_name: &str,
    transport: Option<ButtplugWebsocketClientTransport>,
    rx: broadcast::Receiver<BPCommand>,
) {
    let err = run_buttplug(client_send, gui_receive, close_receive, client_name, transport, rx).await;
    if let Err(err) = err {
        error!("Buttplug thread error: {}", err);
    }
}

async fn run_buttplug(
    client_send: Option<tokio::sync::broadcast::Sender<ClientEvent>>,
    gui_receive: Option<broadcast::Receiver<GuiEvent>>,
    close_receive: broadcast::Receiver<CloseEvent>, 
    //client: ButtplugClient,
    client_name: &str,
    transport: Option<ButtplugWebsocketClientTransport>,
    rx: broadcast::Receiver<BPCommand>,
) -> Result<(), Error> {
    info!("Launched buttplug.io thread");

    let client = match transport {
        Some(_) => ButtplugClient::new(client_name),
        _ => in_process_client(client_name, false).await,
    };

    let recv = tokio_stream::wrappers::BroadcastStream::new(rx);
    let close_recv = tokio_stream::wrappers::BroadcastStream::new(close_receive);


    info!("Starting buttplug.io client");

    enum Event {
        Buttplug(ButtplugClientEvent),
        Command(BPCommand),
        GuiCommand(GuiEvent),
        CloseCommand,
    }

    let merge_bp_and_commands = tokio_stream::StreamExt::merge(
        client.event_stream().map(Event::Buttplug),
        recv.map(|ev| {
            Event::Command(match ev {
                Ok(ev) => ev,
                Err(_) => BPCommand::Stop, // stop on error
            })
        }),
    );

    let merge_bp_and_commands_and_close = tokio_stream::StreamExt::merge(
        merge_bp_and_commands,
        close_recv.map(|_| Event::CloseCommand),
    );

    // couldn't find a more elegant way of doing this
    let (_, dummy_recv) = tokio::sync::broadcast::channel(1);
    let gui_receive_stream = tokio_stream::wrappers::BroadcastStream::new(gui_receive.unwrap_or(dummy_recv));
    
    let mut merge_bp_and_commands_and_close_and_gui = tokio_stream::StreamExt::merge(
        merge_bp_and_commands_and_close,
        gui_receive_stream.map(|ev| {
            Event::GuiCommand(match ev {
                Ok(ev) => ev,
                Err(_) => GuiEvent::None, // stop on error
            })
        }),
    );

    if let Some(transport) = transport {
        let connector = ButtplugRemoteClientConnector::<ButtplugWebsocketClientTransport, ButtplugClientJSONSerializer>::new(transport);
        client.connect(connector).await?;
    }
    client.start_scanning().await.context("Couldn't start buttplug.io device scan")?;

    let mut enabled_devices = HashSet::new();

    while let Some(event) = merge_bp_and_commands_and_close_and_gui.next().await {
        match event {
            Event::Buttplug(ButtplugClientEvent::DeviceAdded(dev)) => {
                if let Some(ref client_send) = client_send {
                    match client_send.send(ClientEvent::DeviceFound(dev.name().clone(), dev.index())) {
                        Ok(_) => {},
                        Err(e) => error!("Error sending client event: {}", e),
                    }
                }
                info!("Intiface: Device added: {}", dev.name());
                debug!("Device {} capabilities: vibrate: {}, linear: {}", dev.name(),
                    !dev.vibrate_attributes().is_empty(),
                    !dev.linear_attributes().is_empty());
                enabled_devices.insert(dev.index());
            }
            Event::Buttplug(ButtplugClientEvent::DeviceRemoved(dev)) => {
                if let Some(ref client_send) = client_send {
                    match client_send.send(ClientEvent::DeviceLost(dev.name().clone(), dev.index())) {
                        Ok(_) => {},
                        Err(e) => error!("Error sending client event: {}", e),
                    }
                }
                info!("Intiface: Device removed: {}", dev.name());
                enabled_devices.remove(&dev.index());
            }
            Event::Buttplug(ButtplugClientEvent::ServerDisconnect) => {
                info!("Intiface: server disconnected, shutting down.");
                break; // we're done
            }
            Event::Buttplug(ButtplugClientEvent::ServerConnect) => {
                info!("Intiface: connected to server.");
            }
            Event::Buttplug(ButtplugClientEvent::Error(e)) => {
                info!("Intiface: error {}", e);
            }
            Event::Buttplug(ButtplugClientEvent::PingTimeout) => {
                info!("Intiface: server not responding to ping.");
            }
            Event::Buttplug(ButtplugClientEvent::ScanningFinished) => {
                info!("Intiface: scanning complete.");
            }
            Event::Command(command) => {
                match command {
                    BPCommand::Vibrate(speed) => {
                        for device in client.devices() {
                            if enabled_devices.contains(&device.index()) {
                                if device.vibrate_attributes().is_empty() {
                                    debug!("Device {} does not support vibration attributes, skipping", device.name());
                                    continue;
                                }
                                device.vibrate(&ScalarValueCommand::ScalarValue(speed.min(1.0)))
                                    .await
                                    .context("Couldn't send Vibrate command")?;
                            }
                        }
                    },
                    BPCommand::VibrateIndex(speed, index) => {
                        for device in client.devices() {
                            if enabled_devices.contains(&device.index()) {
                                let vibrate_len = device.vibrate_attributes().len() as u32;

                                if vibrate_len == 0 { // Prevent underflow: Check if the device has any vibration attributes before performing subtraction
                                    debug!("Device {} does not support vibration attributes, skipping", device.name());
                                    continue;
                                }
                                
                                let nindex = index.min(vibrate_len - 1); // Ensure index is within bounds
                                info!("Setting speed {} on index {} on device {}", speed, nindex, &device.name());
                                
                                let map = HashMap::from([(nindex, speed.min(1.0))]);
                                device.vibrate(&ScalarValueCommand::ScalarValueMap(map)).await.context("Couldn't send VibrateIndex command")?;
                            }
                        }
                    },                                      
                    BPCommand::Linear(duration, position) => {
                        for device in client.devices() {
                            if enabled_devices.contains(&device.index()) {
                                if device.linear_attributes().is_empty() {  // Ensure the device has linear attributes before proceeding
                                    debug!("Device {} does not support linear movement, skipping", device.name());
                                    continue;
                                }

                                let max_index = device.linear_attributes().len() as u32;    // Safe handling: check if the device has at least one linear attribute
                                
                                if max_index > 0 {  // Prevent underflow: Making sure we don't subtract from zero
                                    let nindex = position.min((max_index - 1) as f64);
                                    info!("Setting linear position {} on device {}", position, device.name());
                                    let map = HashMap::from([(nindex as u32, (duration as u32, position.min(1.0)))]);
                                    device.linear(&LinearCommand::LinearMap(map))
                                        .await
                                        .context("Couldn't send Linear command")?;
                                } else {
                                    debug!("Device {} does not support linear movement, skipping", device.name());
                                }
                            }
                        }
                    },                                      
                    BPCommand::Stop => {
                        for device in client.devices() {
                            if enabled_devices.contains(&device.index()) {
                                if !device.vibrate_attributes().is_empty() {
                                    device.vibrate(&ScalarValueCommand::ScalarValue(0.0))
                                        .await
                                        .context("Couldn't send Stop command for vibration")?;
                                }
                                if !device.linear_attributes().is_empty() {
                                    let map = HashMap::from([(0, (0, 0.0))]);
                                    device.linear(&LinearCommand::LinearMap(map))
                                        .await
                                        .context("Couldn't send Stop command for linear movement")?;
                                }
                            }
                        }
                    }
                }                
            },
            Event::CloseCommand => {
                info!("Buttplug thread asked to quit");
                if client.connected() {
                    client.disconnect().await.expect("Failed to disconnect from buttplug");
                }
            },
            Event::GuiCommand(GuiEvent::DeviceToggle(index, checked)) => {
                info!("Device {} enabled {}", index, checked);
                match checked {
                    true => {
                        enabled_devices.insert(index);
                    },
                    false => {
                        enabled_devices.remove(&index);
                    }
                }
            }
            Event::GuiCommand(GuiEvent::None) => {
                info!("GUI command");
            }
        }
    }

    // And now we're done!
    info!("Exiting buttplug thread");
    Ok(())
}

pub fn spawn_run_thread(close_receive: broadcast::Receiver<CloseEvent>, connect_url: &String, client_send: Option<tokio::sync::broadcast::Sender<ClientEvent>>, gui_receive: Option<broadcast::Receiver<GuiEvent>>) -> Result<(broadcast::Sender<BPCommand>, RemoteHandle<()>), Error> {
    info!("Spawning buttplug thread");
    let client_name = "CS2 integration";
    let (send, recv) = broadcast::channel(5);

    let transport = match connect_url {
        connect_url if connect_url.starts_with("wss://") => Some(ButtplugWebsocketClientTransport::new_secure_connector(&connect_url, false)),
        connect_url if connect_url.starts_with("ws://") => Some(ButtplugWebsocketClientTransport::new_insecure_connector(&connect_url)),
        _ => {
            warn!("Invalid server URL, starting local buttplug.io server.");
            None
        }
    };

    let handle = async_manager::spawn_with_handle(run_buttplug_catch(
                client_send,
                gui_receive,
                close_receive,
                client_name,
                transport,
                recv,
            ))?;

    Ok((send, handle))
}
