use crate::messaging::model::{Message, MessageBody};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum RtcRequestBody {
    #[serde(rename = "build")]
    Build { blueprint_id: String },
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RtcRequest {
    pub body: RtcRequestBody,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RtcResponse {
    pub id: uuid::Uuid,
    pub success: bool,
    pub message: Option<String>,
}

impl RtcResponse {
    pub fn from_message(msg: Message) -> Result<Self, anyhow::Error> {
        let response = match msg.body {
            MessageBody::AuthenticationResponse(result) => match result {
                Ok(token) => Self {
                    id: msg.id,
                    success: true,
                    message: Some(token),
                },
                Err(err_msg) => Self {
                    id: msg.id,
                    success: false,
                    message: Some(err_msg),
                },
            },
            MessageBody::BuildResponse(result) => match result {
                Ok(building) => Self {
                    id: msg.id,
                    success: true,
                    message: Some(building),
                },
                Err(err_msg) => Self {
                    id: msg.id,
                    success: false,
                    message: Some(err_msg),
                },
            },
            _ => {
                tracing::debug!("Unsupported message body for RtcResponse: {:?}", msg.body);
                return Err(anyhow::anyhow!("Unsupported message body for RtcResponse"));
            }
        };
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtc_request_serialization() {
        let request = RtcRequest {
            body: RtcRequestBody::Build {
                blueprint_id: "example_blueprint".to_string(),
            },
        };

        let serialized = serde_json::to_string(&request).unwrap();
        let expected = r#"{"body":{"build":{"blueprint":"example_blueprint"}}}"#;
        assert_eq!(serialized, expected);

        let deserialized: RtcRequest = serde_json::from_str(&serialized).unwrap();
        match deserialized.body {
            RtcRequestBody::Build { blueprint_id } => {
                assert_eq!(blueprint_id, "example_blueprint");
            }
        }
    }
}
