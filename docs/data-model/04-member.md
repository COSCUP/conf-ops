# Member（成員）

## 1. 實體總覽

Member 代表帳號在特定專案中的身份。一個帳號加入一個專案即成為該專案的一位成員。成員可透過 Web 介面操作，也可以透過 Email 與系統互動。

**主要用途：**
- 建立帳號（Account）與專案（Project）之間的關聯
- 定義帳號在專案中的角色與權限
- 作為任務參與、待辦指派、成員標籤指派的基礎

**關聯實體：**
- `accounts`：關聯的帳號
- `projects`：所屬專案
- `member_tag_assignments`：成員標籤指派
- `todo_assignees`：待辦事項指派
- `messages`：任務對話中的訊息來源

---

## 2. Table 定義

```sql
CREATE TYPE member_role AS ENUM ('owner', 'tag_admin', 'member');

CREATE TABLE members (
    id          UUID        PRIMARY KEY,
    project_id  UUID        NOT NULL REFERENCES projects(id),
    account_id  UUID        NOT NULL REFERENCES accounts(id),
    role        member_role NOT NULL DEFAULT 'member',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `project_id` | UUID | 所屬專案 ID，FK → `projects(id)` |
| `account_id` | UUID | 關聯帳號 ID，FK → `accounts(id)` |
| `role` | member_role | 專案角色：`owner` / `tag_admin` / `member` |
| `created_at` | TIMESTAMPTZ | 加入時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

---

## 3. 索引定義

```sql
-- 同一專案中同一帳號不可重複（排除已刪除）
CREATE UNIQUE INDEX uq_members_project_account
    ON members (project_id, account_id)
    WHERE deleted_at IS NULL;

-- 依專案查詢成員
CREATE INDEX idx_members_project_id
    ON members (project_id)
    WHERE deleted_at IS NULL;

-- 依帳號查詢所屬專案
CREATE INDEX idx_members_account_id
    ON members (account_id)
    WHERE deleted_at IS NULL;
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Owner,
    TagAdmin,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub id: Uuid,
    pub project_id: Uuid,
    pub account_id: Uuid,
    pub role: MemberRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| Member → Account | `accounts` | 多對一 | 成員關聯的帳號 |
| Member → Project | `projects` | 多對一 | 成員所屬的專案 |
| Member → MemberTagAssignment | `member_tag_assignments` | 一對多 | 成員被指派的標籤 |
| Member → TodoAssignee | `todo_assignees` | 一對多 | 成員被指派的待辦事項 |

**外鍵約束：**

```sql
ALTER TABLE members
    ADD CONSTRAINT fk_members_projects
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE members
    ADD CONSTRAINT fk_members_accounts
    FOREIGN KEY (account_id) REFERENCES accounts(id);
```

---

## 6. 業務規則與約束

### 唯一性

- **同一帳號在同一專案中只能有一個成員身份**：由 `uq_members_project_account` 唯一約束保證（排除已刪除記錄）
- 一個帳號可以同時是多個專案的成員

### 專案角色權限

| 角色 | 說明 | 權限 |
|------|------|------|
| `owner` | 專案擁有者 | 完整管理權限：設定權限規則、管理所有成員與標籤、管理所有任務模板 |
| `tag_admin` | 標籤管理者 | 權限由專案擁有者設定，可包含：管理所屬標籤設定、管理所屬標籤下的任務模板、查看所屬標籤下所有任務、管理所屬標籤的成員指派 |
| `member` | 專案成員 | 權限由專案擁有者設定，可包含：建立任務（從自身標籤的模板）、查看自身參與的任務、在任務對話中操作 |

### 擁有者規則

- 專案建立者自動成為 `owner`
- 複製專案時，操作者自動成為新專案的 `owner`
- 專案必須始終至少有一位 `owner`

### 互動方式

- **Web 介面**：成員可登入系統進行完整操作
- **Email**：成員透過 Email 發送的內容會進入對應的任務對話

### 軟刪除

- 成員退出專案採用軟刪除機制（設定 `deleted_at`）
- 已刪除的成員身份不影響唯一約束（索引排除已刪除記錄），帳號可重新加入專案
