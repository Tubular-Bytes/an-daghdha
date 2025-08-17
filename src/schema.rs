// @generated automatically by Diesel CLI.

diesel::table! {
    account_buildings (id) {
        id -> Int4,
        account_id -> Int4,
        buildings -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    account_resources (id) {
        id -> Int4,
        account_id -> Int4,
        inventory -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    account_sessions (id) {
        id -> Int4,
        account_id -> Int4,
        #[max_length = 255]
        session_token -> Varchar,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
        expires_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    accounts (id) {
        id -> Int4,
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
    blueprints (id) {
        id -> Int4,
        account_id -> Int4,
        #[max_length = 100]
        name -> Varchar,
        blueprint -> Jsonb,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(account_buildings -> accounts (account_id));
diesel::joinable!(account_resources -> accounts (account_id));
diesel::joinable!(account_sessions -> accounts (account_id));
diesel::joinable!(blueprints -> accounts (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    account_buildings,
    account_resources,
    account_sessions,
    accounts,
    blueprints,
);
