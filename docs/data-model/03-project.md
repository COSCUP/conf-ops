# Project（專案）

## 1. 實體總覽

Project 屬於一個組織，代表一項活動或專案（如 COSCUP 2025）。專案可從空白建立，或從同組織內的現有專案複製，繼承完整的組織結構與累積經驗。

**主要用途：**
- 作為活動籌備的工作單位，包含成員、任務、記憶等
- 管理專案層級的權限設定與工具設定
- 支援專案複製，實現跨屆經驗傳承

**關聯實體：**
- `organizations`：所屬組織
- `members`：專案成員
- `member_tags`：專案中定義的成員標籤
- `tasks`：專案中的任務
- `tool_configs`：專案層級的工具設定
- `memories`：專案層級的記憶
- `webhooks`：專案註冊的 Webhook

---

## 2. Table 定義

```sql
CREATE TYPE project_status AS ENUM ('preparing', 'active', 'completed', 'archived');

CREATE TABLE projects (
    id                  UUID            PRIMARY KEY,
    organization_id     UUID            NOT NULL REFERENCES organizations(id),
    name                VARCHAR         NOT NULL,
    description         TEXT,
    status              project_status  NOT NULL DEFAULT 'preparing',
    source_project_id   UUID            REFERENCES projects(id),
    permission_settings JSONB           NOT NULL DEFAULT '{}',
    created_by          UUID            NOT NULL REFERENCES accounts(id),
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `organization_id` | UUID | 所屬組織 ID，FK → `organizations(id)` |
| `name` | VARCHAR | 專案名稱 |
| `description` | TEXT | 專案描述，可為 NULL |
| `status` | project_status | 專案狀態：`preparing` / `active` / `completed` / `archived` |
| `source_project_id` | UUID | 來源專案 ID（若為複製建立），FK → `projects(id)`，可為 NULL |
| `permission_settings` | JSONB | 專案層級的權限設定 |
| `created_by` | UUID | 建立者帳號 ID，FK → `accounts(id)` |
| `created_at` | TIMESTAMPTZ | 建立時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

---

## 3. 索引定義

```sql
CREATE INDEX idx_projects_organization_id
    ON projects (organization_id)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_projects_status
    ON projects (status)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_projects_created_by
    ON projects (created_by)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_projects_source_project_id
    ON projects (source_project_id)
    WHERE source_project_id IS NOT NULL AND deleted_at IS NULL;
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Preparing,
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub source_project_id: Option<Uuid>,
    pub permission_settings: PermissionSettings,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSettings {
    pub task_visibility: Vec<PermissionRule>,
    pub tool_permissions: Vec<PermissionRule>,
    pub memory_visibility: Vec<PermissionRule>,
    pub data_access: Vec<PermissionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub tag: String,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| Project → Organization | `organizations` | 多對一 | 專案屬於一個組織 |
| Project → Member | `members` | 一對多 | 專案包含多位成員 |
| Project → MemberTag | `member_tags` | 一對多 | 專案定義多個成員標籤 |
| Project → Task | `tasks` | 一對多 | 專案包含多個任務 |
| Project → ToolConfig | `tool_configs` | 一對多 | 專案層級的工具設定 |
| Project → Memory | `memories` | 一對多 | 專案層級的記憶 |
| Project → Webhook | `webhooks` | 一對多 | 專案註冊的 Webhook |
| Project → Project（自參照） | `projects` | 多對一 | 來源專案（複製來源） |
| Project.created_by → Account | `accounts` | 多對一 | 專案建立者 |

**外鍵約束：**

```sql
ALTER TABLE projects
    ADD CONSTRAINT fk_projects_organizations
    FOREIGN KEY (organization_id) REFERENCES organizations(id);

ALTER TABLE projects
    ADD CONSTRAINT fk_projects_source
    FOREIGN KEY (source_project_id) REFERENCES projects(id);

ALTER TABLE projects
    ADD CONSTRAINT fk_projects_accounts
    FOREIGN KEY (created_by) REFERENCES accounts(id);
```

---

## 6. JSONB 欄位 Schema

### `permission_settings`

專案層級的權限設定，控制各成員標籤對任務、工具、記憶與資料的存取權限。

```json
{
  "task_visibility": [
    {
      "tag": "贊助組",
      "allow": ["own_tag_tasks"],
      "deny": []
    },
    {
      "tag": "資訊組",
      "allow": ["own_tag_tasks", "cross_tag_linked_tasks"],
      "deny": []
    }
  ],
  "tool_permissions": [
    {
      "tag": "贊助組",
      "allow": ["smtp/sendEmail", "google/sheets"],
      "deny": []
    }
  ],
  "memory_visibility": [
    {
      "tag": "*",
      "allow": ["project_memories", "own_tag_memories"],
      "deny": ["other_tag_memories"]
    }
  ],
  "data_access": [
    {
      "tag": "贊助組",
      "allow": ["own_tag_data"],
      "deny": ["other_tag_data"]
    }
  ]
}
```

**JSON Schema：**

```json
{
  "type": "object",
  "properties": {
    "task_visibility": {
      "type": "array",
      "items": { "$ref": "#/$defs/permissionRule" },
      "description": "任務可見性設定，控制各標籤可查看的任務範圍"
    },
    "tool_permissions": {
      "type": "array",
      "items": { "$ref": "#/$defs/permissionRule" },
      "description": "工具權限設定，控制各標籤可使用的執行工具"
    },
    "memory_visibility": {
      "type": "array",
      "items": { "$ref": "#/$defs/permissionRule" },
      "description": "記憶可見性設定，控制各標籤可查看的記憶範圍"
    },
    "data_access": {
      "type": "array",
      "items": { "$ref": "#/$defs/permissionRule" },
      "description": "資料存取設定，控制各標籤可存取的資料表範圍"
    }
  },
  "$defs": {
    "permissionRule": {
      "type": "object",
      "required": ["tag", "allow", "deny"],
      "properties": {
        "tag": {
          "type": "string",
          "description": "成員標籤名稱，\"*\" 表示所有標籤"
        },
        "allow": {
          "type": "array",
          "items": { "type": "string" },
          "description": "允許的權限項目"
        },
        "deny": {
          "type": "array",
          "items": { "type": "string" },
          "description": "拒絕的權限項目"
        }
      }
    }
  }
}
```

---

## 7. 業務規則與約束

### 狀態轉換

```
preparing → active → completed → archived
                  ↘              ↗
                    archived
```

| 轉換 | 說明 |
|------|------|
| `preparing` → `active` | 專案開始進行，通常在成員與任務模板就緒後 |
| `active` → `completed` | 活動結束，所有主要任務完成 |
| `active` → `archived` | 專案提前封存（如活動取消） |
| `completed` → `archived` | 完成的專案進行封存 |

- `preparing` 不可直接跳到 `completed`
- `archived` 為終態，不可再變更
- 狀態轉換由專案擁有者（`owner`）執行

### 複製專案行為

僅該專案的現有成員可複製專案。複製後的新專案屬於同一組織，操作者自動成為新專案的 `owner`。

> **Phase 2 僅複製基本資料，Phase 12 實作深複製。**

| 項目 | 複製行為 |
|------|---------|
| `name`、`description` | 複製，可在建立時修改 |
| `status` | 重設為 `preparing` |
| `source_project_id` | 記錄為來源專案的參考 |
| 成員標籤 `member_tags` | **完整複製**（含名稱、描述、記憶、`external_task_creation` 設定） |
| 任務模板 `task_templates` | **完整複製**（含待辦事項模板、子待辦事項模板、資料表定義、記憶） |
| 專案記憶 `memories` | **完整複製** |
| 工具設定 `tool_configs` | **完整複製**（含 API Key、連線參數、啟用狀態等） |
| 權限設定 `permission_settings` | **完整複製** |
| 成員 `members` | **不複製**，需重新邀請 |
| 任務 `tasks` | **不複製** |
| 聯絡人 `contacts` | **不複製**（屬於組織層級，仍可使用，但標籤指派不複製） |
| 資料表資料 `data_entries` | **不複製**（僅複製定義） |

### 權限

- 專案建立者自動成為專案的 `owner`（記錄在 `members` 表中）
- `permission_settings` 由專案擁有者管理
