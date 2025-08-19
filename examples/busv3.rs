use an_daghdha::messaging::{Message, MessageBody, MessageBroker};
use tokio::signal;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Create broker and handler
    let (broker, mut handler) = MessageBroker::new();
    
    // Subscribe to messages before starting the handler
    let mut subscriber_foo = broker.subscribe("example").await.unwrap();
    
    let task_handler = tokio::spawn(async move {
        handler.start().await;
    });

    tracing::info!("Messaging system started");
    let now = chrono::Utc::now().timestamp_millis() as u64;

    // Send some messages
    broker.send(Message {
        id: Uuid::new_v4(),
        body: MessageBody::Example { foo: "bar".into() },
        topic: Some("example".into()),
        timestamp: now,
    }).await.unwrap();

    // Start a task to handle subscription messages
    let _subscription_handler = tokio::spawn(async move {
        while let Some(msg) = subscriber_foo.recv().await {
            tracing::info!("Received subscribed message: {:?}", msg);
        }
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown...");
        }
        Err(err) => {
            tracing::error!("Unable to listen for shutdown signal: {}", err);
            // Continue anyway to allow manual shutdown
        }
    }

    // Send a stop message
    broker.send(Message {
        id: Uuid::new_v4(),
        body: MessageBody::Stop,
        topic: None,
        timestamp: now,
    }).await.unwrap();

    task_handler.await.unwrap();
    
    // Note: subscription_handler will end when the subscription channel is closed
}