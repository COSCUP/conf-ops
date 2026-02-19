# Organization（組織）

## 1. 實體總覽

Organization 代表一個團體或社群（如 COSCUP 籌備團隊、開源社群基金會等）。組織是專案的容器，管理跨專案共用的設定、聯絡人與記憶。

**主要用途：**
- 作為專案（Project）的上層容器
- 管理組織成員與角色權限
- 管理跨專案共用的外部聯絡人（Contact）
- 組織層級的工具設定與記憶

**關聯實體：**
- `organization_members`：組織成員關係（junction table）
- `projects`：組織擁有的專案
- `contacts`：組織管理的外部聯絡人
- `tool_configs`：組織層級的工具設定
- `memories`：組織層級的記憶
- `library_documents`：組織的記憶庫文件

---

## 2. Table 定義

### 2.1 `organizations` 主表

```sql
CREATE TABLE organizations (
    id          UUID        PRIMARY KEY,
    name        VARCHAR     NOT NULL,
    description TEXT,
    logo_url    VARCHAR,
    created_by  UUID        NOT NULL REFERENCES accounts(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `name` | VARCHAR | 組織名稱 |
| `description` | TEXT | 組織描述，可為 NULL |
| `logo_url` | VARCHAR | 組織 Logo URL，可為 NULL |
| `created_by` | UUID | 建立者帳號 ID，FK → `accounts(id)` |
| `created_at` | TIMESTAMPTZ | 建立時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

### 2.2 `organization_members` 關聯表

```sql
CREATE TYPE org_role AS ENUM ('org_owner', 'org_admin', 'org_member');

CREATE TABLE organization_members (
    id              UUID        PRIMARY KEY,
    organization_id UUID        NOT NULL REFERENCES organizations(id),
    account_id      UUID        NOT NULL REFERENCES accounts(id),
    role            org_role    NOT NULL DEFAULT 'org_member',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7 |
| `organization_id` | UUID | 組織 ID，FK → `organizations(id)` |
| `account_id` | UUID | 帳號 ID，FK → `accounts(id)` |
| `role` | org_role | 組織角色：`org_owner` / `org_admin` / `org_member` |
| `created_at` | TIMESTAMPTZ | 加入時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |

---

## 3. 索引定義

```sql
-- organizations 索引
CREATE INDEX idx_organizations_name
    ON organizations (name)
    WHERE deleted_at IS NULL;

CREATE INDEX idx_organizations_created_by
    ON organizations (created_by)
    WHERE deleted_at IS NULL;

-- organization_members 索引
CREATE UNIQUE INDEX uq_organization_members_org_account
    ON organization_members (organization_id, account_id);

CREATE INDEX idx_organization_members_account_id
    ON organization_members (account_id);
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrgRole {
    OrgOwner,
    OrgAdmin,
    OrgMember,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationMember {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub account_id: Uuid,
    pub role: OrgRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| Organization → Project | `projects` | 一對多 | 組織包含多個專案 |
| Organization → Contact | `contacts` | 一對多 | 組織管理跨專案共用的外部聯絡人 |
| Organization → OrganizationMember | `organization_members` | 一對多 | 組織的成員列表 |
| Organization → Account（透過 junction） | `organization_members` | 多對多 | 帳號可加入多個組織 |
| Organization → ToolConfig | `tool_configs` | 一對多 | 組織層級的工具設定 |
| Organization → Memory | `memories` | 一對多 | 組織層級的記憶 |
| Organization → LibraryDocument | `library_documents` | 一對多 | 組織的記憶庫文件 |
| Organization.created_by → Account | `accounts` | 多對一 | 組織建立者 |

**外鍵約束：**

```sql
ALTER TABLE organizations
    ADD CONSTRAINT fk_organizations_accounts
    FOREIGN KEY (created_by) REFERENCES accounts(id);

ALTER TABLE organization_members
    ADD CONSTRAINT fk_organization_members_organizations
    FOREIGN KEY (organization_id) REFERENCES organizations(id);

ALTER TABLE organization_members
    ADD CONSTRAINT fk_organization_members_accounts
    FOREIGN KEY (account_id) REFERENCES accounts(id);
```

---

## 6. 業務規則與約束

### 組織角色權限

| 角色 | 說明 | 權限 |
|------|------|------|
| `org_owner` | 組織擁有者 | 完整管理權限：管理組織設定、成員、建立專案、管理所有角色 |
| `org_admin` | 組織管理者 | 可管理專案、邀請成員，具體權限由組織擁有者設定 |
| `org_member` | 組織成員 | 可查看組織內的專案，加入專案的權限由各專案控管 |

### 擁有者規則

- **至少一位擁有者**：組織必須始終至少有一位 `org_owner`。若僅剩一位 `org_owner`，不允許將其角色降級或移除
- **建立者自動成為擁有者**：建立組織的帳號自動成為 `org_owner`，並在 `organization_members` 中新增對應記錄

### 成員管理

- **同一帳號不可重複加入同一組織**：由 `uq_organization_members_org_account` 唯一約束保證
- **邀請流程**：組織擁有者與管理者可透過 Email 邀請成員
  - 已有帳號：直接加入組織
  - 尚無帳號：發送邀請信，註冊後自動加入組織

### 軟刪除

- 組織刪除採用軟刪除機制（設定 `deleted_at`）
- 組織刪除前應確認無進行中的專案
- `organization_members` 不使用軟刪除 — 成員退出時直接刪除記錄
