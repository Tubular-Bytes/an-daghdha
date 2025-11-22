// @generated automatically by Diesel CLI.

diesel::table! {
    account_buildings (id) {
        id -> Uuid,
        account_id -> Uuid,
        buildings -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    account_resources (id) {
        id -> Uuid,
        account_id -> Uuid,
        inventory -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

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
    blueprints (id) {
        id -> Uuid,
        #[max_length = 100]
        name -> Varchar,
        blueprint -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    inventories (id) {
        id -> Uuid,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(account_buildings -> accounts (account_id));
diesel::joinable!(account_resources -> accounts (account_id));
diesel::joinable!(account_sessions -> accounts (account_id));
diesel::joinable!(accounts_x_inventories -> accounts (account_id));
diesel::joinable!(accounts_x_inventories -> inventories (inventory_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_buildings,
    account_resources,
    account_sessions,
    accounts,
    accounts_x_inventories,
    blueprints,
    inventories,
);
