// @generated automatically by Diesel CLI.

diesel::table! {
    labels (id) {
        id -> Integer,
        #[max_length = 50]
        project_id -> Varchar,
        #[max_length = 50]
        key -> Varchar,
        #[max_length = 50]
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
        name_zh -> Varchar,
        description_zh -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 100]
        name_en -> Varchar,
        description_en -> Text,
    }
}

diesel::table! {
    role_managers (id) {
        id -> Integer,
        #[max_length = 50]
        role_id -> Varchar,
        target_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    roles (id) {
        #[max_length = 50]
        id -> Varchar,
        #[max_length = 50]
        name_zh -> Varchar,
        #[max_length = 50]
        project_id -> Varchar,
        login_message_zh -> Nullable<Text>,
        welcome_message_zh -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 50]
        name_en -> Varchar,
        login_message_en -> Nullable<Text>,
        welcome_message_en -> Nullable<Text>,
    }
}

diesel::table! {
    targets (id) {
        id -> Integer,
        #[max_length = 36]
        user_id -> Nullable<Char>,
        label_id -> Nullable<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_flows (id) {
        id -> Integer,
        ticket_id -> Integer,
        #[max_length = 36]
        user_id -> Nullable<Char>,
        ticket_schema_flow_id -> Integer,
        finished -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_form_answers (id) {
        id -> Integer,
        ticket_flow_id -> Integer,
        ticket_schema_form_id -> Integer,
        value -> Json,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_form_files (id) {
        #[max_length = 64]
        id -> Char,
        ticket_schema_form_field_id -> Integer,
        #[max_length = 255]
        path -> Varchar,
        #[max_length = 20]
        mime -> Varchar,
        size -> Unsigned<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_form_images (id) {
        #[max_length = 64]
        id -> Char,
        ticket_schema_form_field_id -> Integer,
        #[max_length = 255]
        path -> Varchar,
        #[max_length = 20]
        mime -> Varchar,
        size -> Unsigned<Integer>,
        width -> Unsigned<Integer>,
        height -> Unsigned<Integer>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_reviews (id) {
        id -> Integer,
        ticket_flow_id -> Integer,
        ticket_schema_review_id -> Integer,
        approved -> Bool,
        comment -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_schema_flows (id) {
        id -> Integer,
        ticket_schema_id -> Integer,
        order -> Integer,
        operator_id -> Integer,
        #[max_length = 100]
        name_zh -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 100]
        name_en -> Varchar,
    }
}

diesel::table! {
    ticket_schema_form_fields (id) {
        id -> Integer,
        ticket_schema_form_id -> Integer,
        order -> Integer,
        #[max_length = 100]
        key -> Varchar,
        #[max_length = 100]
        name_zh -> Varchar,
        description_zh -> Text,
        define -> Json,
        required -> Bool,
        editable -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 100]
        name_en -> Varchar,
        description_en -> Text,
    }
}

diesel::table! {
    ticket_schema_forms (id) {
        id -> Integer,
        ticket_schema_flow_id -> Integer,
        expired_at -> Nullable<Timestamp>,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_schema_managers (id) {
        id -> Integer,
        ticket_schema_id -> Integer,
        target_id -> Integer,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_schema_reviews (id) {
        id -> Integer,
        ticket_schema_flow_id -> Integer,
        restarted -> Bool,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    ticket_schemas (id) {
        id -> Integer,
        #[max_length = 100]
        title_zh -> Varchar,
        description_zh -> Text,
        #[max_length = 50]
        project_id -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        #[max_length = 100]
        title_en -> Varchar,
        description_en -> Text,
    }
}

diesel::table! {
    tickets (id) {
        id -> Integer,
        ticket_schema_id -> Integer,
        #[max_length = 150]
        title -> Varchar,
        finished -> Bool,
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
        #[max_length = 10]
        locale -> Varchar,
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
diesel::joinable!(role_managers -> roles (role_id));
diesel::joinable!(role_managers -> targets (target_id));
diesel::joinable!(roles -> projects (project_id));
diesel::joinable!(targets -> labels (label_id));
diesel::joinable!(targets -> users (user_id));
diesel::joinable!(ticket_flows -> ticket_schema_flows (ticket_schema_flow_id));
diesel::joinable!(ticket_flows -> tickets (ticket_id));
diesel::joinable!(ticket_flows -> users (user_id));
diesel::joinable!(ticket_form_answers -> ticket_flows (ticket_flow_id));
diesel::joinable!(ticket_form_answers -> ticket_schema_forms (ticket_schema_form_id));
diesel::joinable!(ticket_form_files -> ticket_schema_form_fields (ticket_schema_form_field_id));
diesel::joinable!(ticket_form_images -> ticket_schema_form_fields (ticket_schema_form_field_id));
diesel::joinable!(ticket_reviews -> ticket_flows (ticket_flow_id));
diesel::joinable!(ticket_reviews -> ticket_schema_reviews (ticket_schema_review_id));
diesel::joinable!(ticket_schema_flows -> targets (operator_id));
diesel::joinable!(ticket_schema_flows -> ticket_schemas (ticket_schema_id));
diesel::joinable!(ticket_schema_form_fields -> ticket_schema_forms (ticket_schema_form_id));
diesel::joinable!(ticket_schema_forms -> ticket_schema_flows (ticket_schema_flow_id));
diesel::joinable!(ticket_schema_managers -> targets (target_id));
diesel::joinable!(ticket_schema_managers -> ticket_schemas (ticket_schema_id));
diesel::joinable!(ticket_schema_reviews -> ticket_schema_flows (ticket_schema_flow_id));
diesel::joinable!(ticket_schemas -> projects (project_id));
diesel::joinable!(tickets -> ticket_schemas (ticket_schema_id));
diesel::joinable!(user_emails -> users (user_id));
diesel::joinable!(user_sessions -> users (user_id));
diesel::joinable!(users -> projects (project_id));
diesel::joinable!(users_labels -> labels (label_id));
diesel::joinable!(users_labels -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    labels,
    projects,
    role_managers,
    roles,
    targets,
    ticket_flows,
    ticket_form_answers,
    ticket_form_files,
    ticket_form_images,
    ticket_reviews,
    ticket_schema_flows,
    ticket_schema_form_fields,
    ticket_schema_forms,
    ticket_schema_managers,
    ticket_schema_reviews,
    ticket_schemas,
    tickets,
    user_emails,
    user_sessions,
    users,
    users_labels,
);
