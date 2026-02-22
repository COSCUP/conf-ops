// utoipa's OpenApi derive macro generates code that triggers clippy::needless_for_each.
// This cannot be fixed without modifying the upstream utoipa crate.
#![allow(clippy::needless_for_each)]

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use super::routes::{
    accounts, ai_suggestions, api_keys, audit, auth, contacts, data_entries, data_external,
    email_inbound, email_threads, health, member_tags, members, memories, notifications,
    organizations, projects, task_templates, tasks, todos, tools, webhooks,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Conf-Ops API",
        version = "0.1.0",
        description = "AI-assisted conference/event project management system"
    ),
    paths(
        health::healthz,
        health::readyz,
        auth::request_magic_link,
        auth::verify_magic_link,
        auth::passkey_register_begin,
        auth::passkey_register_complete,
        auth::passkey_login_begin,
        auth::passkey_login_complete,
        auth::refresh,
        auth::logout,
        accounts::get_me,
        accounts::update_me,
        accounts::get_profile,
        accounts::update_profile,
        accounts::list_passkeys,
        accounts::delete_passkey,
        accounts::get_notification_preferences,
        accounts::update_notification_preferences,
        organizations::create_organization,
        organizations::list_organizations,
        organizations::get_organization,
        organizations::update_organization,
        organizations::delete_organization,
        organizations::list_members,
        organizations::invite_member,
        organizations::update_member_role,
        organizations::remove_member,
        projects::create_project,
        projects::list_projects,
        projects::copy_project,
        projects::get_project,
        projects::update_project,
        projects::delete_project,
        projects::update_project_status,
        projects::get_permission_settings,
        projects::update_permission_settings,
        members::list_members,
        members::invite_member,
        members::get_member,
        members::update_member,
        members::delete_member,
        contacts::list_contacts,
        contacts::create_contact,
        contacts::get_contact,
        contacts::update_contact,
        contacts::delete_contact,
        contacts::merge_contacts,
        member_tags::list_tags,
        member_tags::create_tag,
        member_tags::get_tag,
        member_tags::update_tag,
        member_tags::delete_tag,
        member_tags::assign_tag,
        member_tags::remove_assignment,
        member_tags::update_external_task_creation,
        task_templates::list_templates,
        task_templates::create_template,
        task_templates::get_template,
        task_templates::update_template,
        task_templates::delete_template,
        task_templates::list_template_tags,
        task_templates::link_tag,
        task_templates::unlink_tag,
        task_templates::list_todo_templates,
        task_templates::create_todo_template,
        task_templates::get_todo_template,
        task_templates::update_todo_template,
        task_templates::delete_todo_template,
        task_templates::reorder_todo_templates,
        task_templates::list_data_schemas,
        task_templates::create_data_schema,
        task_templates::get_data_schema,
        task_templates::update_data_schema,
        task_templates::delete_data_schema,
        tasks::create_task,
        tasks::list_tasks,
        tasks::get_task,
        tasks::update_task,
        tasks::delete_task,
        tasks::update_task_status,
        todos::list_todos,
        todos::create_todo,
        todos::get_todo,
        todos::update_todo,
        todos::delete_todo,
        todos::update_todo_status,
        todos::add_assignee,
        todos::remove_assignee,
        todos::list_my_todos,
        data_entries::list_entries,
        data_entries::upsert_entry,
        data_entries::get_entry,
        data_entries::delete_entry,
        data_entries::get_aggregated_sheet,
        data_external::get_template_data,
        data_external::list_external_templates,
        data_external::get_task_data,
        data_external::update_task_data,
        email_threads::list_email_threads,
        email_threads::create_email_thread,
        email_threads::update_email_thread,
        email_threads::delete_email_thread,
        email_threads::list_thread_messages,
        email_threads::send_thread_email,
        email_inbound::receive_inbound_email,
        email_inbound::list_unassigned_inbox,
        email_inbound::assign_unassigned_email,
        memories::list_memories,
        memories::create_memory,
        memories::get_memory,
        memories::update_memory,
        memories::delete_memory,
        memories::list_memory_versions,
        memories::list_library_documents,
        memories::create_library_document,
        memories::get_library_document,
        memories::update_library_document,
        memories::delete_library_document,
        memories::list_library_document_versions,
        ai_suggestions::list_suggestions,
        ai_suggestions::get_suggestion_group,
        ai_suggestions::decide_suggestion,
        ai_suggestions::request_suggestion,
        ai_suggestions::resolve_placeholders,
        notifications::list_notifications,
        notifications::get_unread_count,
        notifications::mark_as_read,
        notifications::mark_all_as_read,
        notifications::subscribe_web_push,
        notifications::unsubscribe_web_push,
        tools::list_tools,
        tools::get_tool_details,
        tools::execute_tool,
        tools::list_project_tool_configs,
        tools::create_project_tool_config,
        tools::update_project_tool_config,
        tools::delete_project_tool_config,
        tools::list_org_tool_configs,
        webhooks::list_webhooks,
        webhooks::create_webhook,
        webhooks::get_webhook,
        webhooks::update_webhook,
        webhooks::delete_webhook,
        webhooks::test_webhook,
        webhooks::list_webhook_logs,
        api_keys::create_api_key,
        api_keys::list_api_keys,
        api_keys::delete_api_key,
        audit::list_org_audit_logs,
        audit::list_project_audit_logs,
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "accounts", description = "Account management endpoints"),
        (name = "organizations", description = "Organization management endpoints"),
        (name = "projects", description = "Project management endpoints"),
        (name = "members", description = "Project member management endpoints"),
        (name = "contacts", description = "Contact management endpoints"),
        (name = "member-tags", description = "Member tag management endpoints"),
        (name = "task-templates", description = "Task template management endpoints"),
        (name = "tasks", description = "Task management endpoints"),
        (name = "todos", description = "Todo management endpoints"),
        (name = "data-entries", description = "Data entry management endpoints"),
        (name = "external-data", description = "External data access endpoints"),
        (name = "email-threads", description = "Email thread management endpoints"),
        (name = "email-inbound", description = "Email inbound webhook and unassigned inbox endpoints"),
        (name = "memories", description = "Memory and library document management endpoints"),
        (name = "ai-suggestions", description = "AI suggestion pipeline endpoints"),
        (name = "notifications", description = "Notification and Web Push endpoints"),
        (name = "tools", description = "Tool execution endpoints"),
        (name = "tool-configs", description = "Tool configuration management endpoints"),
        (name = "webhooks", description = "Webhook management endpoints"),
        (name = "api-keys", description = "API key management endpoints"),
        (name = "audit", description = "Audit log endpoints"),
        (name = "observability", description = "Observability and metrics endpoints")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

/// Generate the `OpenAPI` spec as a JSON string.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn generate_openapi_json() -> Result<String, serde_json::Error> {
    let doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&doc)
}
