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
    let (_, mut subscriber_foo) = broker.subscribe("example").await.unwrap();
    
    let task_handler = tokio::spawn(async move {
        handler.start().await;
    });

    tracing::info!("Messaging system started");
    let now = chrono::Utc::now().timestamp_millis() as u64;

    let id = Uuid::new_v4();
    // Send some messages
    broker.send(Message {
        id,
        body: MessageBody::Example { foo: "bar".into() },
        topic: Some("example".into()),
        reply: Some(format!("reply-{}", id)),
        timestamp: now,
    }).await.unwrap();

    let bbb = broker.clone();
    let (reply_id, mut subscriber_reply) = broker.subscribe(format!("reply-{}", id).as_str()).await.unwrap();
    let _reply_handler = tokio::spawn(async move {
        while let Some(msg) = subscriber_reply.recv().await {
            tracing::info!("Received reply message: {:?}", msg);
        }
        bbb.unsubscribe(reply_id).await.unwrap();
    });

    // Start a task to handle subscription messages
    let sub_broker = broker.clone();
    let _subscription_handler = tokio::spawn(async move {
        while let Some(msg) = subscriber_foo.recv().await {
            tracing::info!("Received subscribed message: {:?}", msg);
            sub_broker.send(Message {
                id: Uuid::new_v4(),
                body: MessageBody::ExampleResponse { foo: "response".into() },
                topic: msg.reply,
                reply: None,
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
            }).await.unwrap();
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
        reply: None,
        timestamp: now,
    }).await.unwrap();

    task_handler.await.unwrap();
    
    // Note: subscription_handler will end when the subscription channel is closed
}