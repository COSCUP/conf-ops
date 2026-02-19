# MemberTag（成員標籤）

## 1. 實體總覽

MemberTag 代表專案中的角色或組別（如：組長、贊助組、資訊組）。屬於專案，可指派給 Member 或 Contact。成員標籤是權限控管、任務歸屬與跨組協作的核心機制。

**主要用途：**
- 定義專案中的角色或組別
- 作為權限範圍的基礎（`permission_settings` 以標籤為單位設定）
- 控制任務模板的歸屬與可見性
- 設定跨組任務建立規則（`external_task_creation`）
- 存放角色專屬的記憶與經驗

**關聯實體：**
- `projects`：所屬專案
- `member_tag_assignments`：標籤指派記錄（junction table）
- `members`：透過指派關聯的成員
- `contacts`：透過指派關聯的聯絡人
- `task_template_tags`：與任務模板的多對多關係
- `tasks`：以此標籤為 `owner_tag` 的任務
- `memories`：標籤層級的記憶

---

## 2. Table 定義

### 2.1 `member_tags` 主表

```sql
CREATE TABLE member_tags (
    id                     UUID        PRIMARY KEY,
    project_id             UUID        NOT NULL REFERENCES projects(id),
    name                   VARCHAR     NOT NULL,
    description            TEXT,
    external_task_creation JSONB       NOT NULL DEFAULT '[]',
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at             TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `project_id` | UUID | 所屬專案 ID，FK → `projects(id)` |
| `name` | VARCHAR | 標籤名稱（如「贊助組」、「資訊組」） |
| `description` | TEXT | 標籤描述，可為 NULL |
| `external_task_creation` | JSONB | 外部建立任務設定，定義哪些任務模板允許由哪些其他標籤建立 |
| `created_at` | TIMESTAMPTZ | 建立時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

### 2.2 `member_tag_assignments` 關聯表

```sql
CREATE TABLE member_tag_assignments (
    id          UUID        PRIMARY KEY,
    tag_id      UUID        NOT NULL REFERENCES member_tags(id),
    member_id   UUID        REFERENCES members(id),
    contact_id  UUID        REFERENCES contacts(id),
    project_id  UUID        NOT NULL REFERENCES projects(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT chk_member_or_contact
        CHECK (
            (member_id IS NOT NULL AND contact_id IS NULL) OR
            (member_id IS NULL AND contact_id IS NOT NULL)
        )
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7 |
| `tag_id` | UUID | 成員標籤 ID，FK → `member_tags(id)` |
| `member_id` | UUID | 成員 ID，FK → `members(id)`，可為 NULL |
| `contact_id` | UUID | 聯絡人 ID，FK → `contacts(id)`，可為 NULL |
| `project_id` | UUID | 專案 ID，FK → `projects(id)`，用於快速查詢 |
| `created_at` | TIMESTAMPTZ | 指派時間，UTC |

**CHECK 約束：** `member_id` 與 `contact_id` 恰好其中一個為 NOT NULL — 每筆指派記錄只能對應一位成員或一位聯絡人。

---

## 3. 索引定義

```sql
-- member_tags 索引
CREATE INDEX idx_member_tags_project_id
    ON member_tags (project_id)
    WHERE deleted_at IS NULL;

-- member_tag_assignments 索引
CREATE INDEX idx_member_tag_assignments_tag_id
    ON member_tag_assignments (tag_id);

CREATE INDEX idx_member_tag_assignments_member_id
    ON member_tag_assignments (member_id)
    WHERE member_id IS NOT NULL;

CREATE INDEX idx_member_tag_assignments_contact_id
    ON member_tag_assignments (contact_id)
    WHERE contact_id IS NOT NULL;

CREATE INDEX idx_member_tag_assignments_project_id
    ON member_tag_assignments (project_id);

-- 防止同一成員在同一標籤中重複指派
CREATE UNIQUE INDEX uq_member_tag_assignments_tag_member
    ON member_tag_assignments (tag_id, member_id)
    WHERE member_id IS NOT NULL;

-- 防止同一聯絡人在同一標籤中重複指派
CREATE UNIQUE INDEX uq_member_tag_assignments_tag_contact
    ON member_tag_assignments (tag_id, contact_id)
    WHERE contact_id IS NOT NULL;
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberTag {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub external_task_creation: Vec<ExternalTaskCreationRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTaskCreationRule {
    pub task_template_id: Uuid,
    pub allowed_from_tags: AllowedFromTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AllowedFromTags {
    All(String),           // "*" 表示所有標籤
    Specific(Vec<String>), // 指定的標籤名稱列表
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberTagAssignment {
    pub id: Uuid,
    pub tag_id: Uuid,
    pub member_id: Option<Uuid>,
    pub contact_id: Option<Uuid>,
    pub project_id: Uuid,
    pub created_at: DateTime<Utc>,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| MemberTag → Project | `projects` | 多對一 | 標籤屬於一個專案 |
| MemberTag → MemberTagAssignment | `member_tag_assignments` | 一對多 | 標籤的指派記錄 |
| MemberTag → Member（透過 junction） | `member_tag_assignments` | 多對多 | 標籤可指派給多位成員 |
| MemberTag → Contact（透過 junction） | `member_tag_assignments` | 多對多 | 標籤可指派給多位聯絡人 |
| MemberTag → TaskTemplate（透過 junction） | `task_template_tags` | 多對多 | 標籤關聯多個任務模板 |
| MemberTag → Task | `tasks` | 一對多 | 以此標籤為 owner_tag 的任務 |
| MemberTag → Memory | `memories` | 一對多 | 標籤層級的記憶 |

**外鍵約束：**

```sql
ALTER TABLE member_tags
    ADD CONSTRAINT fk_member_tags_projects
    FOREIGN KEY (project_id) REFERENCES projects(id);

ALTER TABLE member_tag_assignments
    ADD CONSTRAINT fk_member_tag_assignments_tags
    FOREIGN KEY (tag_id) REFERENCES member_tags(id);

ALTER TABLE member_tag_assignments
    ADD CONSTRAINT fk_member_tag_assignments_members
    FOREIGN KEY (member_id) REFERENCES members(id);

ALTER TABLE member_tag_assignments
    ADD CONSTRAINT fk_member_tag_assignments_contacts
    FOREIGN KEY (contact_id) REFERENCES contacts(id);

ALTER TABLE member_tag_assignments
    ADD CONSTRAINT fk_member_tag_assignments_projects
    FOREIGN KEY (project_id) REFERENCES projects(id);
```

---

## 6. JSONB 欄位 Schema

### `external_task_creation`

定義哪些自身擁有的任務模板，允許由哪些其他成員標籤的成員建立任務。用於跨組協作場景。

```json
[
  {
    "taskTemplateId": "550e8400-e29b-41d4-a716-446655440000",
    "allowedFromTags": "*"
  },
  {
    "taskTemplateId": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
    "allowedFromTags": ["贊助組", "行銷組"]
  }
]
```

**JSON Schema：**

```json
{
  "type": "array",
  "items": {
    "type": "object",
    "required": ["taskTemplateId", "allowedFromTags"],
    "properties": {
      "taskTemplateId": {
        "type": "string",
        "format": "uuid",
        "description": "允許外部建立的任務模板 ID，必須屬於此成員標籤"
      },
      "allowedFromTags": {
        "oneOf": [
          {
            "type": "string",
            "const": "*",
            "description": "允許所有成員標籤的成員建立"
          },
          {
            "type": "array",
            "items": { "type": "string" },
            "description": "允許指定成員標籤名稱列表中的成員建立"
          }
        ]
      }
    }
  }
}
```

**使用情境範例：**

資訊組設定允許贊助組透過「技術支援請求」模板建立任務：

```json
[
  {
    "taskTemplateId": "tech-support-template-uuid",
    "allowedFromTags": ["贊助組"]
  }
]
```

贊助組成員即可從資訊組的「技術支援請求」模板建立任務，該任務歸屬於資訊組，但建立者（贊助組成員）也成為任務的參與人。

---

## 7. 業務規則與約束

### 標籤指派

- **成員與聯絡人皆可指派**：透過 `member_tag_assignments` 的 CHECK 約束，每筆指派恰好對應一位 Member 或一位 Contact
- **不可重複指派**：由唯一索引 `uq_member_tag_assignments_tag_member` 與 `uq_member_tag_assignments_tag_contact` 保證
- 一位成員或聯絡人可擁有多個標籤
- 一個標籤可指派給多位成員與聯絡人

### 權限範圍

- 成員標籤是 `permission_settings` 的基礎單位 — 權限以標籤為範圍設定
- `tag_admin` 角色的管理範圍以所屬標籤為限
- 任務的 `owner_tag` 決定該任務的組別歸屬與可見性

### 複製專案行為

- 複製專案時，成員標籤**完整複製**（含名稱、描述、記憶、`external_task_creation` 設定）
- 標籤的成員指派**不複製**（因為成員不複製）
- 標籤的聯絡人指派**不複製**（需在新專案中重新指派）

### 軟刪除

- `member_tags` 使用軟刪除機制（設定 `deleted_at`）
- `member_tag_assignments` 不使用軟刪除 — 取消指派時直接刪除記錄
