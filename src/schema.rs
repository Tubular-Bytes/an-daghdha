// @generated automatically by Diesel CLI.

diesel::table! {
    account_sessions (id) {
        id -> Uuid,
        account_id -> Uuid,
        #[max_length = 255]
        session_token -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        expires_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    accounts (id) {
        id -> Uuid,
        #[max_length = 50]
        username -> Varchar,
        #[max_length = 100]
        email -> Varchar,
        #[max_length = 255]
        password_hash -> Varchar,
        #[max_length = 20]
        state -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    accounts_x_inventories (account_id, inventory_id) {
        account_id -> Uuid,
        inventory_id -> Uuid,
    }
}

diesel::table! {
    blueprints (slug) {
        slug -> Text,
        name -> Text,
        properties -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    inventories (id) {
        id -> Uuid,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    inventories_x_buildings (id) {
        id -> Uuid,
        inventory_id -> Uuid,
        blueprint_slug -> Text,
        status -> Text,
        progress -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    inventories_x_resources (inventory_id, resource) {
        inventory_id -> Uuid,
        resource -> Text,
        quantity -> Int4,
    }
}

diesel::table! {
    resources (slug) {
        slug -> Text,
        name -> Text,
    }
}

diesel::joinable!(account_sessions -> accounts (account_id));
diesel::joinable!(accounts_x_inventories -> accounts (account_id));
diesel::joinable!(accounts_x_inventories -> inventories (inventory_id));
diesel::joinable!(inventories_x_buildings -> blueprints (blueprint_slug));
diesel::joinable!(inventories_x_buildings -> inventories (inventory_id));
diesel::joinable!(inventories_x_resources -> inventories (inventory_id));
diesel::joinable!(inventories_x_resources -> resources (resource));

diesel::allow_tables_to_appear_in_same_query!(
    account_sessions,
    accounts,
    accounts_x_inventories,
    blueprints,
    inventories,
    inventories_x_buildings,
    inventories_x_resources,
    resources,
);
