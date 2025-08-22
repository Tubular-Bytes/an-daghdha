use sha2::{Digest, Sha256};
use rusty_paseto::{core::{Key, Local, PasetoSymmetricKey, V4}, prelude::{ExpirationClaim, PasetoBuilder, SubjectClaim}};

use crate::actor::auth::User;

pub fn generate_token(user: &User) -> Result<String, anyhow::Error> {
    let expiration = chrono::Utc::now() + chrono::Duration::days(2);
    let user_id = user.id.clone().to_string();

    let pass_hash = Sha256::digest(user.password.as_bytes());

    let key = PasetoSymmetricKey::<V4, Local>::from(Key::from(pass_hash.as_slice()));
    let token = PasetoBuilder::<V4, Local>::default()
    .set_claim(SubjectClaim::from(user_id.as_str()))
    .set_claim(ExpirationClaim::from(expiration.to_rfc3339().try_into()?))
    .build(&key)?;
    Ok(token)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn test_generate_token_success() {
        let id = Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap();
        let user = User {id: id, password: "supersecretpassword".to_string(), username: "testuser".to_string() };
        let token = generate_token(&user);
        assert!(token.is_ok());
        let token_str = token.unwrap();
        assert!(!token_str.is_empty());
    }
}
