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

/// Resolve the identity of an email sender within a project's organization.
///
/// 1. Check `accounts` table for email -> Member
/// 2. Check `contacts` table for email -> Contact
/// 3. Auto-create a new Contact -> `NewContact`
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
