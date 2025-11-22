use diesel::prelude::*;
use diesel::PgConnection;
use sha2::Digest;
use sha2::Sha256;

pub fn hash_str(input: &str) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub async fn _get_id_by_auth(
    conn: &mut PgConnection,
    auth: &str,
) -> Result<Option<uuid::Uuid>, diesel::result::Error> {
    use crate::schema::account_sessions::dsl::*;

    let result = account_sessions
        .filter(session_token.eq(auth))
        .select(id)
        .first::<uuid::Uuid>(conn)
        .optional()?;

    Ok(result)
}

pub async fn _save_account(conn: &mut PgConnection, account: &crate::model::AccountRequest) {
    use crate::schema::accounts;

    let hash = hash_str(&account.password);

    let new_account = (
        accounts::username.eq(&account.username),
        accounts::password_hash.eq(&hash),
        accounts::email.eq(&account.email),
    );

    diesel::insert_into(accounts::table)
        .values(&new_account)
        .execute(conn)
        .expect("Error saving new account");
}

pub async fn authenticate(
    conn: &mut PgConnection,
    user: &str,
    password: &str,
) -> Result<Option<uuid::Uuid>, diesel::result::Error> {
    use crate::schema::accounts::dsl::*;

    let hash = hash_str(password);

    let user_id = accounts
        .filter(username.eq(user))
        .filter(password_hash.eq(hash)) // In real code, use hashed passwords!
        .select(id)
        .first::<uuid::Uuid>(conn)
        .optional()?;
    Ok(user_id)
}

pub async fn get_inventory_ids(
    conn: &mut PgConnection,
) -> Result<Vec<uuid::Uuid>, diesel::result::Error> {
    use crate::schema::inventories::dsl::*;

    let ids = inventories.select(id).load::<uuid::Uuid>(conn)?;

    Ok(ids)
}
