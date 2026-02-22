use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Scope Type ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScopeType {
    Account,
    Organization,
    Project,
    MemberTag,
    TaskTemplate,
    Task,
}

impl ScopeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Organization => "organization",
            Self::Project => "project",
            Self::MemberTag => "member_tag",
            Self::TaskTemplate => "task_template",
            Self::Task => "task",
        }
    }

    pub fn sort_order(&self) -> i32 {
        match self {
            Self::Task => 0,
            Self::TaskTemplate => 1,
            Self::MemberTag => 2,
            Self::Project => 3,
            Self::Organization => 4,
            Self::Account => 5,
        }
    }
}

impl std::fmt::Display for ScopeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ScopeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "account" => Ok(Self::Account),
            "organization" => Ok(Self::Organization),
            "project" => Ok(Self::Project),
            "member_tag" => Ok(Self::MemberTag),
            "task_template" => Ok(Self::TaskTemplate),
            "task" => Ok(Self::Task),
            other => Err(format!("Invalid scope type: {other}")),
        }
    }
}

// ── Memory Source ───────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemorySource {
    Manual,
    AutoExtracted,
}

impl MemorySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::AutoExtracted => "auto_extracted",
        }
    }
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MemorySource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "manual" => Ok(Self::Manual),
            "auto_extracted" => Ok(Self::AutoExtracted),
            other => Err(format!("Invalid memory source: {other}")),
        }
    }
}

// ── Memory ──────────────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Memory {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub content: String,
    pub library_ref: Option<Uuid>,
    pub source: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub struct CreateMemoryParams {
    pub id: Uuid,
    pub scope_type: ScopeType,
    pub scope_id: Uuid,
    pub content: String,
    pub library_ref: Option<Uuid>,
    pub source: MemorySource,
    pub created_by: Uuid,
}

pub struct UpdateMemoryParams {
    pub content: String,
    pub library_ref: Option<Uuid>,
}

// ── Library Document ────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LibraryDocument {
    pub id: Uuid,
    pub scope_type: String,
    pub scope_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

pub struct CreateLibraryDocumentParams {
    pub id: Uuid,
    pub scope_type: ScopeType,
    pub scope_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_by: Uuid,
}

pub struct UpdateLibraryDocumentParams {
    pub title: Option<String>,
    pub content: String,
}

// ── Memory Version ──────────────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemoryVersion {
    pub id: Uuid,
    pub memory_id: Uuid,
    pub content: String,
    pub changed_by: Uuid,
    pub created_at: DateTime<Utc>,
}

// ── Library Document Version ────────────────────────────────

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LibraryDocumentVersion {
    pub id: Uuid,
    pub document_id: Uuid,
    pub title: Option<String>,
    pub content: String,
    pub changed_by: Uuid,
    pub created_at: DateTime<Utc>,
}

// ── Inherited Memories ──────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InheritedMemories {
    pub task_id: Uuid,
    pub memories: Vec<Memory>,
}
