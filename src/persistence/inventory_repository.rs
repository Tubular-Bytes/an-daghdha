use std::fmt::Display;

use diesel::prelude::*;
use diesel::PgConnection;
use uuid::Uuid;

enum Status {
    InProgress,
    Completed,
    Stopped,
}

impl From<String> for Status {
    fn from(s: String) -> Self {
        match s.as_str() {
            "in_progress" => Status::InProgress,
            "completed" => Status::Completed,
            "stopped" => Status::Stopped,
            _ => Status::InProgress, // Default case
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status_str = match self {
            Status::InProgress => "in_progress",
            Status::Completed => "completed",
            Status::Stopped => "stopped",
        };
        write!(f, "{}", status_str)
    }
}

pub async fn create_building(
    conn: &mut PgConnection,
    inventory_id: Uuid,
    blueprint_slug: String,
) -> Result<Uuid, diesel::result::Error> {
    use crate::schema::inventories_x_buildings;

    // TODO return from query
    let id = Uuid::new_v4();

    let new_building = (
        inventories_x_buildings::id.eq(id),
        inventories_x_buildings::inventory_id.eq(inventory_id),
        inventories_x_buildings::blueprint_slug.eq(blueprint_slug),
        inventories_x_buildings::status.eq(Status::InProgress.to_string()),
        inventories_x_buildings::progress.eq(0),
        inventories_x_buildings::created_at.eq(chrono::Utc::now().naive_utc()),
        inventories_x_buildings::updated_at.eq(chrono::Utc::now().naive_utc()),
    );

    diesel::insert_into(inventories_x_buildings::table)
        .values(&new_building)
        .execute(conn)?;

    Ok(id)
}

// pub async fn process_building_ticks(
//     conn: &mut PgConnection,
//     inventory: Uuid,
// ) -> Result<(), diesel::result::Error> {
//     use crate::schema::account_buildings::dsl::*;

//     diesel::update(account_buildings
//         .filter(id.)
//         .filter(id.eq(inventory)))
//         .set(updated_at.eq(chrono::Utc::now().naive_utc()))
//         .execute(conn)?;

//     Ok(())
// }
