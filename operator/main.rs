mod controllers;

use controllers::{interface, ipam, key, peer};

use tracing::*;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt().init();

    info!("starting ipam controller");
    let ipam_controller = ipam::run();

    info!("starting key controller");
    let key_controller = key::run();

    info!("starting peer controller");
    let peer_controller = peer::run();

    info!("starting interface controller");
    let interface_controller = interface::run();

    let results = tokio::join!(
        ipam_controller,
        key_controller,
        peer_controller,
        interface_controller
    );

    results.0?;
    results.1?;
    results.2?;
    results.3?;

    Ok(())
}
