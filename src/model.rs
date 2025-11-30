use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::accounts)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Account {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub state: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

pub struct AccountRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::account_sessions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AccountSession {
    pub id: Uuid,
    pub account_id: Uuid,
    pub session_token: String,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub expires_at: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::inventories_x_buildings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InventoryBuilding {
    pub id: Uuid,
    pub inventory_id: Uuid,
    pub blueprint_slug: String,
    pub status: String,
    pub progress: i32,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
