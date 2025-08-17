pub struct Broker;

impl Broker {
    pub async fn spawn(
        mut stop_rx: tokio::sync::oneshot::Receiver<()>,
        mut bus_rx: tokio::sync::mpsc::Receiver<String>,
    ) -> Self {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        tracing::info!("Broker stopping");
                        break;
                    }
                    Some(message) = bus_rx.recv() => {
                        tracing::info!("Received message: {}", message);
                        // Handle the message as needed
                    }
                }
            }
        });

        Broker
    }
}
