use std::net::SocketAddr;

use an_daghdha::messaging::{
    broker::MessageBroker, model::Message, model::MessageBody, model::Status,
};
use tokio::net::TcpListener;
use tokio::signal;

use an_daghdha::websocket::api;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (broker, mut handler) = MessageBroker::new();

    let task_handler = tokio::spawn(async move {
        handler.start().await;
    });

    let addr = "127.0.0.1:8080".parse::<SocketAddr>()?;
    let bouncer = api::Bouncer::new(&broker);


    let auth_actor = an_daghdha::actor::auth::AuthActorHandler::load("users.json".into())?;
    let auth_broker = broker.clone();
    tokio::spawn(async move {
        auth_actor.listen(auth_broker).await.unwrap();
    });

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    tracing::info!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(bouncer.clone().handle_connection(addr, stream));
        }
    });

    // Wait for Ctrl+C signal
    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        Err(err) => {
            tracing::error!("Unable to listen for shutdown signal: {}", err);
            // Continue anyway to allow manual shutdown
        }
    }

    // Send stop signal to the bus
    if let Err(e) = broker
        .send(Message::new(MessageBody::Stop, None, false))
        .await
    {
        tracing::error!("Failed to send stop signal to bus: {}", e);
    }

    let mut grace_wait = 5;
    loop {
        let status = broker.status().await;
        if status == Status::Stopped {
            break;
        }

        grace_wait -= 1;

        if grace_wait <= 0 {
            tracing::error!("failed to wait for broker to shut down gracefully, terminating");
            break;
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    task_handler.await?;
    Ok(())
}
