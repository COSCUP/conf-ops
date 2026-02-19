# Contact（外部聯絡人）

## 1. 實體總覽

Contact 代表不具備帳號的外部參與者（如贊助商、講者、外部合作對象），僅透過 Email 與系統互動。Contact 屬於組織層級，同一組織內的所有專案共用。

**主要用途：**
- 管理外部參與者的基本資訊
- 透過 Email 與外部參與者互動（經由執行工具如 `smtp/sendEmail`）
- 可被指派成員標籤，參與專案中的任務對話

**關聯實體：**
- `organizations`：所屬組織
- `member_tag_assignments`：在各專案中被指派的成員標籤
- `messages`：任務對話中透過 Email 的訊息來源

---

## 2. Table 定義

```sql
CREATE TABLE contacts (
    id              UUID        PRIMARY KEY,
    organization_id UUID        NOT NULL REFERENCES organizations(id),
    name            VARCHAR     NOT NULL,
    email           VARCHAR     NOT NULL,
    merged_into_id  UUID        REFERENCES contacts(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ
);
```

**欄位說明：**

| 欄位 | 型別 | 說明 |
|------|------|------|
| `id` | UUID | 主鍵，UUID v7，應用層產生 |
| `organization_id` | UUID | 所屬組織 ID，FK → `organizations(id)` |
| `name` | VARCHAR | 聯絡人名稱 |
| `email` | VARCHAR | 聯絡人 Email |
| `merged_into_id` | UUID | 合併目標 Contact ID，FK → `contacts(id)`，NULL 表示未被合併 |
| `created_at` | TIMESTAMPTZ | 建立時間，UTC |
| `updated_at` | TIMESTAMPTZ | 最後更新時間，UTC |
| `deleted_at` | TIMESTAMPTZ | 軟刪除時間，NULL 表示有效 |

---

## 3. 索引定義

```sql
-- 同一組織內 Email 索引（用於查詢與重複偵測）
CREATE INDEX idx_contacts_organization_email
    ON contacts (organization_id, email)
    WHERE deleted_at IS NULL AND merged_into_id IS NULL;

-- 依組織查詢聯絡人
CREATE INDEX idx_contacts_organization_id
    ON contacts (organization_id)
    WHERE deleted_at IS NULL;

-- 合併追蹤索引
CREATE INDEX idx_contacts_merged_into_id
    ON contacts (merged_into_id)
    WHERE merged_into_id IS NOT NULL;
```

---

## 4. Rust Struct

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub email: String,
    pub merged_into_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

---

## 5. 關聯說明

| 關聯 | 對應表 | 類型 | 說明 |
|------|--------|------|------|
| Contact → Organization | `organizations` | 多對一 | 聯絡人屬於一個組織 |
| Contact → MemberTagAssignment | `member_tag_assignments` | 一對多 | 聯絡人在各專案中被指派的標籤 |
| Contact → Contact（自參照） | `contacts` | 多對一 | 合併目標（`merged_into_id`） |

**外鍵約束：**

```sql
ALTER TABLE contacts
    ADD CONSTRAINT fk_contacts_organizations
    FOREIGN KEY (organization_id) REFERENCES organizations(id);

ALTER TABLE contacts
    ADD CONSTRAINT fk_contacts_merged_into
    FOREIGN KEY (merged_into_id) REFERENCES contacts(id);
```

---

## 6. 業務規則與約束

### 建立方式

- **手動建立**：成員可在組織中手動建立 Contact，填寫名稱與 Email
- **自動建立**：系統收到未知 Email 來信時，自動建立 Contact 並歸入對應組織

### 合併操作（Merge）

當同一外部參與者存在多個 Contact 記錄（如不同 Email 或重複建立）時，可將多個 Contact 合併為一個：

1. **選擇主要 Contact**：保留一個作為主要記錄
2. **設定合併指向**：被合併的 Contact 設定 `merged_into_id` 指向主要 Contact
3. **歸併關聯**：被合併 Contact 的對話記錄、成員標籤指派等全部歸併到主要 Contact
4. **查詢過濾**：查詢時預設排除已合併的 Contact（`WHERE merged_into_id IS NULL`）
5. **組織層級操作**：合併影響該組織內所有專案

**合併後的被合併 Contact：**
- `merged_into_id` 設定為主要 Contact 的 ID
- 不進行軟刪除（不設定 `deleted_at`），保留記錄以供追溯
- 系統在查詢到已合併的 Contact 時，自動跟隨 `merged_into_id` 指向主要 Contact

### 合併防循環保護

合併操作時必須解析 `merged_into_id` 鏈至最終目標（非已合併的 Contact），並拒絕會造成循環的合併請求。應用層 ContactService 在執行合併前須檢查目標 Contact 的 `merged_into_id` 是否指向來源，防止形成 A → B → A 的循環鏈。

### 與 Member 的比較

| 特性 | Member | Contact |
|------|--------|---------|
| 有帳號 | 是 | 否 |
| 可登入系統 | 是 | 否 |
| Web 介面操作 | 是 | 否 |
| 查看任務對話 | 是 | 否 |
| Email 互動 | 是 | 是（唯一互動方式） |
| 被指派成員標籤 | 是 | 是 |
| 被指派待辦事項 | 是 | 否 |
| 確認 AI 建議 | 是 | 否 |
| 對話中的來源類型 | `member` | `member`（透過 `sourceId` 區分） |

### 跨專案行為

- 同一 Contact 可出現在組織內多個專案的任務對話中
- 各專案可為同一 Contact 獨立指派成員標籤
- 複製專案時不複製 Contact 的成員標籤指派（Contact 本身仍在組織中可用）

### 軟刪除

- Contact 刪除採用軟刪除機制（設定 `deleted_at`）
- 已合併的 Contact 不進行軟刪除，透過 `merged_into_id` 追蹤
