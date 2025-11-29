use crate::messaging::broker::MessageBroker;
use crate::messaging::model::{Message, MessageBody};
use crate::persistence::{Query, QueryResponse};
use uuid::Uuid;

pub async fn handle_build_request(
    broker: &MessageBroker,
    inventory_id: Uuid,
    blueprint_slug: String,
) -> MessageBody {
    let response = broker
        .request(Message::new_request(
            MessageBody::PersistenceQueryRequest(Query::CreateBuilding {
                inventory_id,
                blueprint_slug: blueprint_slug.clone(),
            }),
            Some("persistence".into()),
        ))
        .await;

    match response {
        Ok(Some(Message {
            body: MessageBody::PersistenceQueryResponse(QueryResponse::CreateBuilding(building_id)),
            ..
        })) => MessageBody::BuildResponse(Ok(building_id)),

        Ok(Some(Message {
            body: MessageBody::PersistenceQueryResponse(QueryResponse::CreateBuildingFailed(e)),
            ..
        })) => MessageBody::BuildResponse(Err(format!("Failed to create building: {}", e))),

        Ok(Some(_)) => MessageBody::BuildResponse(Err("Unexpected response type".into())),

        Ok(None) => MessageBody::BuildResponse(Err("No response received from persistence".into())),

        Err(e) => {
            MessageBody::BuildResponse(Err(format!("Failed to send persistence request: {}", e)))
        }
    }
}
