// @generated automatically by Diesel CLI.

diesel::table! {
    labels (id) {
        id -> Integer,
        #[max_length = 50]
        project_id -> Varchar,
        #[max_length = 20]
        key -> Varchar,
        #[max_length = 20]
        value -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    projects (id) {
        #[max_length = 50]
        id -> Varchar,
        #[max_length = 100]
        name -> Varchar,
        description -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    user_emails (id) {
        id -> Integer,
        #[max_length = 36]
        user_id -> Char,
        #[max_length = 255]
        email -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    user_sessions (id) {
        #[max_length = 36]
        id -> Char,
        #[max_length = 36]
        user_id -> Char,
        #[max_length = 255]
        user_agent -> Varchar,
        #[max_length = 45]
        ip -> Varchar,
        expired_at -> Timestamp,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        #[max_length = 36]
        id -> Char,
        #[max_length = 100]
        name -> Varchar,
        #[max_length = 50]
        project_id -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    users_labels (user_id, label_id) {
        #[max_length = 36]
        user_id -> Char,
        label_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::joinable!(labels -> projects (project_id));
diesel::joinable!(user_emails -> users (user_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(users -> projects (project_id));
diesel::joinable!(users_labels -> labels (label_id));
diesel::joinable!(users_labels -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    labels,
    projects,
    user_emails,
    user_sessions,
    users,
    users_labels,
);
