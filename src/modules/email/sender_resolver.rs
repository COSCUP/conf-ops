use sqlx::PgPool;
use uuid::Uuid;

use crate::id::generate_id;

use super::error::EmailError;

#[derive(Debug)]
pub enum SenderIdentity {
    Member { account_id: Uuid, member_id: Uuid },
    Contact { contact_id: Uuid },
    NewContact { contact_id: Uuid },
}

/// Detect if an email body contains forwarded content and extract the original sender's email.
fn detect_forwarded_sender(body: &str) -> Option<String> {
    let forward_markers = [
        "---------- Forwarded message ----------",
        "Begin forwarded message:",
        "-------- Original Message --------",
    ];

    let has_forward = forward_markers.iter().any(|marker| body.contains(marker));
    if !has_forward {
        return None;
    }

    // Try to extract "From:" from the forwarded block
    let from_pattern = regex::Regex::new(
        r"(?i)From:\s*(?:.*<)?([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})>?",
    )
    .ok()?;

    // Find all From: matches; the one after the forward marker is the original sender
    for marker in &forward_markers {
        if let Some(pos) = body.find(marker) {
            let after_marker = &body[pos..];
            if let Some(cap) = from_pattern.captures(after_marker) {
                return cap.get(1).map(|m| m.as_str().to_string());
            }
        }
    }

    None
}

/// Resolve the identity of an email sender within a project's organization.
///
/// 1. Check `accounts` table for email -> Member
/// 2. Check `contacts` table for email -> Contact
/// 3. Auto-create a new Contact -> `NewContact`
///
/// If the email body contains a forwarded message, the original sender's
/// email address is extracted and resolved instead.
///
/// # Errors
///
/// Returns `EmailError::Database` on database failure.
pub async fn resolve_sender(
    pool: &PgPool,
    from_address: &str,
    from_name: Option<&str>,
    organization_id: Uuid,
    project_id: Uuid,
    email_body: Option<&str>,
) -> Result<SenderIdentity, EmailError> {
    // Check for forwarded email and resolve the original sender if detected
    if let Some(body) = email_body {
        if let Some(original_sender) = detect_forwarded_sender(body) {
            // Resolve the original sender instead of the forwarder
            return resolve_sender_inner(pool, &original_sender, None, organization_id, project_id)
                .await;
        }
    }

    resolve_sender_inner(pool, from_address, from_name, organization_id, project_id).await
}

async fn resolve_sender_inner(
    pool: &PgPool,
    from_address: &str,
    from_name: Option<&str>,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<SenderIdentity, EmailError> {
    // 1. Check if sender is a registered account
    let account = sqlx::query_as!(
        AccountRow,
        r#"SELECT id FROM accounts WHERE email = $1 AND deleted_at IS NULL"#,
        from_address,
    )
    .fetch_optional(pool)
    .await
    .map_err(EmailError::Database)?;

    if let Some(acct) = account {
        // Check if this account is a member of the project
        let member = sqlx::query_as!(
            MemberRow,
            r#"SELECT id FROM members
             WHERE project_id = $1 AND account_id = $2 AND deleted_at IS NULL"#,
            project_id,
            acct.id,
        )
        .fetch_optional(pool)
        .await
        .map_err(EmailError::Database)?;

        if let Some(m) = member {
            return Ok(SenderIdentity::Member {
                account_id: acct.id,
                member_id: m.id,
            });
        }
    }

    // 2. Check if sender is an existing contact
    let contact = sqlx::query_as!(
        ContactRow,
        r#"SELECT id FROM contacts
         WHERE organization_id = $1 AND email = $2
           AND deleted_at IS NULL AND merged_into_id IS NULL"#,
        organization_id,
        from_address,
    )
    .fetch_optional(pool)
    .await
    .map_err(EmailError::Database)?;

    if let Some(c) = contact {
        return Ok(SenderIdentity::Contact { contact_id: c.id });
    }

    // 3. Auto-create a new contact
    let contact_id = generate_id();
    let name = from_name.filter(|n| !n.is_empty()).unwrap_or(from_address);

    sqlx::query!(
        r#"INSERT INTO contacts (id, organization_id, name, email)
         VALUES ($1, $2, $3, $4)"#,
        contact_id,
        organization_id,
        name,
        from_address,
    )
    .execute(pool)
    .await
    .map_err(EmailError::Database)?;

    Ok(SenderIdentity::NewContact { contact_id })
}

#[derive(sqlx::FromRow)]
struct AccountRow {
    id: Uuid,
}

#[derive(sqlx::FromRow)]
struct MemberRow {
    id: Uuid,
}

#[derive(sqlx::FromRow)]
struct ContactRow {
    id: Uuid,
}
