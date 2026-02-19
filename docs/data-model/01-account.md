# Account（帳號）

## 1. 實體總覽

Account 是 Conf-Ops 系統的使用者身份實體，透過 Email 作為唯一識別。一個帳號可以加入多個組織，並透過組織參與專案。

**主要用途：**
- 使用者身份驗證與識別（Passkey / Email Magic Link）
- 個人資料管理（公開的 `bio` 與私有的 `profile_data`）
- 通知偏好設定
- 作為 Organization、Member 等實體的關聯基礎

**關聯實體：**
- `organization_members`：帳號在組織中的成員關係
- `members`：帳號在專案中的成員身份
- `audit_logs`：帳號執行的操作記錄
- `notifications`：帳號收到的通知

---

## 2. Table 定義

```sql
CREATE TABLE accounts (
    id                       UUID        PRIMARY KEY,
    name                     VARCHAR     NOT NULL,
    email                    VARCHAR     NOT NULL,
    avatar_url               VARCHAR,
    bio                      TEXT,
    profile_data             JSONB       NOT NULL DEFAULT '{}',
    profile_schema           JSONB       NOT NULL DEFAULT '[]',
    notification_preferences JSONB       NOT NULL DEFAULT '{}',
    locale                   VARCHAR     NOT NULL DEFAULT 'zh-TW',
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at               TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `name` | VARCHAR | 使用者名稱 |
| `email` | VARCHAR | Email，系統唯一識別 |
| `avatar_url` | VARCHAR | 頭像 URL，可為 NULL |
| `bio` | TEXT | 公開的自我介紹文字，可供專案內其他成員查閱 |
| `profile_data` | JSONB | 私人的結構化資料（電話、地址、銀行帳號等），僅帳號本人可查看 |
| `profile_schema` | JSONB | 個人欄位結構描述，系統自動維護，AI 僅接收此結構 |
| `notification_preferences` | JSONB | 通知偏好設定（各頻道的啟用與設定） |
| `locale` | VARCHAR | 使用者偏好語系，預設 `zh-TW` |
| `created_at` | TIMESTAMPTZ | 建立時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

---

## 3. 索引定義

```sql
-- Email 唯一索引（排除已刪除帳號）
CREATE UNIQUE INDEX uq_accounts_email
    ON accounts (email)
    WHERE deleted_at IS NULL;

-- 語系索引，用於批次通知等查詢
CREATE INDEX idx_accounts_locale
    ON accounts (locale);
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub profile_data: ProfileData,
    pub profile_schema: Vec<ProfileSchemaField>,
    pub notification_preferences: NotificationPreferences,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/// 個人資料表 — 自由新增的 key-value 結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

/// 個人欄位結構描述 — 系統自動維護
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSchemaField {
    pub key: String,
    pub label: String,
    pub description: String,
    #[serde(rename = "type")]
    pub field_type: String,
}

/// 通知偏好設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPreferences {
    pub channels: NotificationChannels,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannels {
    pub email: ChannelConfig,
    pub web_push: ChannelConfig,
    pub in_app: ChannelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub enabled: bool,
    pub categories: NotificationCategories,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationCategories {
    pub task_updates: bool,
    pub todo_assignments: bool,
    pub ai_suggestions: bool,
    pub mentions: bool,
    pub system_announcements: bool,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| Account → OrganizationMember | `organization_members` | 一對多 | 帳號可加入多個組織 |
| Account → Member | `members` | 一對多 | 帳號可成為多個專案的成員 |
| Account → AuditLog | `audit_logs` | 一對多 | 帳號的操作記錄 |
| Account → Notification | `notifications` | 一對多 | 帳號收到的通知 |

---

## 6. JSONB 欄位 Schema

### 6.1 `profile_data`

自由新增的 key-value 結構，欄位由使用者手動新增或透過 `saveToProfile` 工具從任務資料中存入。

```json
{
  "phone": "+886-912-345-678",
  "address": "台北市信義區...",
  "bank_account": "012-345678901234",
  "dietary_preference": "素食"
}
```

**JSON Schema：**

```json
{
  "type": "object",
  "additionalProperties": true,
  "description": "自由新增的 key-value 結構，value 可為任意 JSON 型別"
}
```

### 6.2 `profile_schema`

系統自動維護的個人欄位結構描述。AI 僅接收此結構以判斷對應關係，不會接觸到 `profile_data` 的實際值。

```json
[
  {
    "key": "phone",
    "label": "聯絡電話",
    "description": "個人手機號碼",
    "type": "string"
  },
  {
    "key": "dietary_preference",
    "label": "飲食偏好",
    "description": "素食、葷食或其他飲食需求",
    "type": "string"
  }
]
```

**JSON Schema：**

```json
{
  "type": "array",
  "items": {
    "type": "object",
    "required": ["key", "label", "description", "type"],
    "properties": {
      "key": {
        "type": "string",
        "description": "欄位鍵名，對應 profile_data 中的 key"
      },
      "label": {
        "type": "string",
        "description": "欄位顯示名稱"
      },
      "description": {
        "type": "string",
        "description": "欄位用途說明"
      },
      "type": {
        "type": "string",
        "description": "欄位資料型別（如 string、number、boolean）"
      }
    }
  }
}
```

### 6.3 `notification_preferences`

通知偏好設定，定義各通知頻道的啟用狀態與各類別的開關。

```json
{
  "channels": {
    "email": {
      "enabled": true,
      "categories": {
        "task_updates": true,
        "todo_assignments": true,
        "ai_suggestions": true,
        "mentions": true,
        "system_announcements": true
      }
    },
    "web_push": {
      "enabled": false,
      "categories": {
        "task_updates": false,
        "todo_assignments": false,
        "ai_suggestions": false,
        "mentions": false,
        "system_announcements": false
      }
    },
    "in_app": {
      "enabled": true,
      "categories": {
        "task_updates": true,
        "todo_assignments": true,
        "ai_suggestions": true,
        "mentions": true,
        "system_announcements": true
      }
    }
  }
}
```

**JSON Schema：**

```json
{
  "type": "object",
  "required": ["channels"],
  "properties": {
    "channels": {
      "type": "object",
      "properties": {
        "email": { "$ref": "#/$defs/channelConfig" },
        "web_push": { "$ref": "#/$defs/channelConfig" },
        "in_app": { "$ref": "#/$defs/channelConfig" }
      }
    }
  },
  "$defs": {
    "channelConfig": {
      "type": "object",
      "required": ["enabled", "categories"],
      "properties": {
        "enabled": { "type": "boolean" },
        "categories": {
          "type": "object",
          "properties": {
            "task_updates": { "type": "boolean" },
            "todo_assignments": { "type": "boolean" },
            "ai_suggestions": { "type": "boolean" },
            "mentions": { "type": "boolean" },
            "system_announcements": { "type": "boolean" }
          }
        }
      }
    }
  }
}
```

---

## 7. 業務規則與約束

### 身份識別

- **Email 為唯一識別**：系統以 Email 作為帳號的唯一識別依據，不允許重複（在未刪除的帳號中）
- **登入方式**：支援 Passkey（WebAuthn）與 Email Magic Link 兩種方式

### 隱私規則

- **`profile_data` 為私人資料**：僅帳號本人可查看與編輯，不對其他成員公開
- **`profile_data` 不傳送給 AI**：AI 不會接觸到個人資料表的實際值
- **AI 僅接收 `profile_schema`**：AI 透過欄位結構描述判斷對應關係，用於 `saveToProfile` 工具的自動填入功能
- **`bio` 為公開資料**：自我介紹文字可供專案內其他成員查閱

### 欄位維護

- **`profile_schema` 維護方式**：可透過 PUT `/accounts/me/profile` 的 `profileSchema` 欄位由使用者手動定義，也可由 AI 工具 `saveToProfile` 自動產生。當 `profile_data` 新增或修改欄位時，若未提供 `profileSchema`，系統自動更新對應的結構描述
- **`notification_preferences` 預設值**：新建帳號時，通知頻道預設為啟用（email、in_app）或停用（web_push），各類別預設為啟用

### 軟刪除

- 帳號刪除採用軟刪除機制（設定 `deleted_at`），不實際刪除資料
- 已刪除帳號的 Email 可被新帳號使用（唯一索引排除已刪除記錄）
