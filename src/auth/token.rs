use rusty_paseto::{
    core::{Key, Local, PasetoSymmetricKey, V4},
    prelude::{ExpirationClaim, PasetoBuilder, PasetoParser, SubjectClaim},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

static PASETO_KEY: &str = "your-secret-key";

pub fn generate_token(user_id: &str) -> Result<String, anyhow::Error> {
    let expiration = chrono::Utc::now() + chrono::Duration::days(2);

    let pass_hash = Sha256::digest(PASETO_KEY.as_bytes());

    let expiration_claim: ExpirationClaim = expiration.to_rfc3339().try_into()?;

    let key = PasetoSymmetricKey::<V4, Local>::from(Key::from(pass_hash.as_slice()));
    let token = PasetoBuilder::<V4, Local>::default()
        .set_claim(SubjectClaim::from(user_id))
        .set_claim(expiration_claim)
        .build(&key)?;
    Ok(token)
}

pub fn validate_token(token: &str) -> Result<Uuid, anyhow::Error> {
    let pass_hash = Sha256::digest(PASETO_KEY.as_bytes());

    let key = PasetoSymmetricKey::<V4, Local>::from(Key::from(pass_hash.as_slice()));

    let parsed_token = PasetoParser::<V4, Local>::default()
        // you can check any claim even custom claims
        .parse(token, &key)?;

    Ok(Uuid::parse_str(parsed_token["sub"].as_str().ok_or_else(
        || anyhow::anyhow!("Missing subject claim"),
    )?)?)
}

pub fn validate_headers(headers: &axum::http::HeaderMap) -> Result<Uuid, anyhow::Error> {
    if let Some(auth_header) = headers.get("Authorization") {
        let auth_str = auth_header.to_str()?;
        if auth_str.starts_with("Bearer ") {
            let token = auth_str.trim_start_matches("Bearer ").trim();
            return validate_token(token);
        }
    }
    Err(anyhow::anyhow!("Missing or invalid Authorization header"))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_generate_token_success() {
        let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let token = generate_token(id.to_string().as_str());
        assert!(token.is_ok());
        let token_str = token.unwrap();
        assert!(!token_str.is_empty());

        let user_id = validate_token(&token_str);
        assert!(user_id.is_ok());
        let user_id = user_id.unwrap();
        assert_eq!(id, user_id);
    }
}
