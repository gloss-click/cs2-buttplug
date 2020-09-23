use std::time::Duration;

use csgo_gsi::{Error, GSIConfigBuilder, GSIServer, Subscription};
use fehler::throws;

#[throws]
pub fn build_server(port: u16) -> GSIServer {
    let config = GSIConfigBuilder::new("CrotchStimGetOff")
        .subscribe_multiple(Subscription::UNRESTRICTED)
        .throttle(Duration::from_millis(0))
        .buffer(Duration::from_millis(0))
        .build();

    let mut server = GSIServer::new(config, port);

    server.install()?;

    server
}
