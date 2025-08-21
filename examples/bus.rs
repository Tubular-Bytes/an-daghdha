use an_daghdha::messaging::{
    broker::MessageBroker,
    model::{Message, MessageBody},
};
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
    broker
        .send(Message {
            id,
            body: MessageBody::Example { foo: "bar".into() },
            topic: Some("example".into()),
            is_request: false,
            timestamp: now,
        })
        .await
        .unwrap();

    let subbroker = broker.clone();
    // Start a task to handle subscription messages
    let _subscription_handler = tokio::spawn(async move {
        while let Some(msg) = subscriber_foo.recv().await {
            tracing::info!("Received subscribed message: {:?}", msg);

            if !msg.is_request {
                continue;
            }

            let reply_topic = msg.reply_topic();
            tracing::info!("Reply topic: {}", reply_topic);

            subbroker
                .send(Message {
                    id: Uuid::new_v4(),
                    body: MessageBody::ExampleResponse {
                        foo: "response".into(),
                    },
                    topic: Some(reply_topic.clone()),
                    is_request: false,
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                })
                .await
                .unwrap();
        }
    });

    let reply = broker
        .request(Message {
            id: Uuid::new_v4(),
            body: MessageBody::Example { foo: "baz".into() },
            topic: Some("example".into()),
            is_request: true,
            timestamp: now,
        })
        .await;

    tracing::info!("reply to 'baz': {reply:?}");

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
    broker
        .send(Message {
            id: Uuid::new_v4(),
            body: MessageBody::Stop,
            topic: None,
            is_request: false,
            timestamp: now,
        })
        .await
        .unwrap();

    task_handler.await.unwrap();

    // Note: subscription_handler will end when the subscription channel is closed
}
