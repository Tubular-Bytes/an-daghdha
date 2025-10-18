pub mod actor;
pub mod auth;
pub mod error;
pub mod messaging;
pub mod websocket;

#[derive(Clone)]
pub struct AppState {
    pub bouncer: websocket::api::Bouncer,
    pub broker: messaging::broker::MessageBroker,
}

impl From<(websocket::api::Bouncer, messaging::broker::MessageBroker)> for AppState {
    fn from(value: (websocket::api::Bouncer, messaging::broker::MessageBroker)) -> Self {
        AppState {
            bouncer: value.0,
            broker: value.1,
        }
    }
}
