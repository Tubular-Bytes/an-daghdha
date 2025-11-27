use std::fmt::Display;

use diesel::prelude::*;
use diesel::PgConnection;
use uuid::Uuid;

use crate::model::InventoryBuilding;

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

    let new_building = InventoryBuilding {
        id: Uuid::new_v4(),
        inventory_id,
        blueprint_slug: blueprint_slug,
        status: Status::InProgress.to_string(),
        progress: 0,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let id = diesel::insert_into(inventories_x_buildings::table)
        .values(&new_building)
        .returning(inventories_x_buildings::id)
        .get_result(conn)?;

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
