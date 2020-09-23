use std::{
    future::Future,
};

use anyhow::{Context, Error};
use buttplug::{
    client::{
        ButtplugClient, ButtplugClientError,
        ButtplugClientEvent,
        device::VibrateCommand,
    },
    connector::{ButtplugRemoteClientConnector, ButtplugWebsocketClientTransport},
    core::messages::{
        serializer::ButtplugClientJSONSerializer,
    },
    util::async_manager,
};
use fehler::throws;
use futures::{StreamExt, future::RemoteHandle};
use tokio::{
    sync::watch,
};

#[throws]
async fn run_buttplug<
    ClientFuture: Future<
        Output = Result<
            (ButtplugClient, impl StreamExt<Item = ButtplugClientEvent> + Unpin),
            ButtplugClientError,
        >,
    >,
>(
    client: ClientFuture,
    recv: watch::Receiver<f64>,
) {
    info!("in buttplug thread");
    let (client, event_stream) = client
        .await
        .context("couldn't start buttplug client")?;

    info!("started client");

    client.start_scanning()
        .await
        .context("couldn't start device scan")?;

    enum Event {
        Buttplug(ButtplugClientEvent),
        VibrateSpeed(f64),
    }

    let mut events = tokio::stream::StreamExt::merge(
        event_stream.map(Event::Buttplug),
        recv.map(Event::VibrateSpeed)
    );

    while let Some(event) = events.next().await {
        match event {
            Event::Buttplug(ButtplugClientEvent::DeviceAdded(dev)) => {
                println!("We got a device: {}", dev.name);
            }
            Event::Buttplug(ButtplugClientEvent::ServerDisconnect) => {
                // The server disconnected, which means we're done here, so just
                // break up to the top level.
                println!("Server disconnected!");
                break;
            }
            Event::Buttplug(_) => {
                // Something else happened, like scanning finishing, devices
                // getting removed, etc... Might as well say something about it.
                println!("Got some other kind of event we don't care about");
            }
            Event::VibrateSpeed(speed) => {
                info!("got vibrate speed {} from script", speed);
                for device in client.devices() {
                    info!("sending vibrate speed {} to device {}", speed, &device.name);
                    device.vibrate(VibrateCommand::Speed(speed)).await
                        .context("couldn't send vibrate command")?;
                }
            }
        }
    }

    // And now we're done!
    println!("Exiting example");
}

#[throws]
pub fn spawn_run_thread(url: &Option<String>) -> (watch::Sender<f64>, RemoteHandle<Result<(), Error>>) {
    info!("spawning buttplug thread");
    let client_name = "Crotch-Stim: Get Off";
    let (send, recv) = watch::channel(0.0);
    let handle = match url {
        None => async_manager::spawn_with_handle(run_buttplug(
            ButtplugClient::connect_in_process(client_name, 0),
            recv,
        ))?,
        Some(connect_url) => {
            let transport = if connect_url.starts_with("wss://") {
                ButtplugWebsocketClientTransport::new_secure_connector(&connect_url, false)
            } else {
                ButtplugWebsocketClientTransport::new_insecure_connector(&connect_url)
            };
            let connector = ButtplugRemoteClientConnector::<
                ButtplugWebsocketClientTransport,
                ButtplugClientJSONSerializer,
            >::new(transport);
            async_manager::spawn_with_handle(run_buttplug(
                ButtplugClient::connect(client_name, connector),
                recv,
            ))?
        }
    };
    (send, handle)
}
