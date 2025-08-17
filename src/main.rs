use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing_subscriber;

use an_daghdha::broker;
use an_daghdha::websocket;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let (bus_tx, bus_rx) = tokio::sync::mpsc::channel(100);

    let addr = "127.0.0.1:8080".parse::<SocketAddr>()?;
    let bouncer = websocket::Bouncer::new(bus_tx);
    let _broker = broker::Broker::spawn(stop_rx, bus_rx).await;

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    tracing::info!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(bouncer.clone().handle_connection(addr, stream));
    }

    stop_tx.send(()).expect("Failed to send stop signal");

    Ok(())
}
