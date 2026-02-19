# 服務拆分與模組架構

## 1. 問題描述

Conf-Ops 是一套以 AI 輔助的研討會/活動專案管理系統，涵蓋認證、專案管理、任務對話、AI 協作、Email 收發、通知排程、檔案儲存、審計日誌等多個功能領域。隨著功能增長，若缺乏明確的模組邊界，程式碼將快速耦合，導致維護困難與團隊協作衝突。

然而，在專案初期就採用微服務架構會帶來不必要的分散式系統複雜度——服務間通訊、分散式事務、部署協調、監控與除錯成本等。在團隊規模有限且需求仍在快速迭代的階段，這些額外成本會嚴重拖慢開發速度。

因此，我們需要一種架構，既能保持清晰的模組邊界以利未來拆分，又能以單一部署單元運行以簡化營運。

## 2. 設計決策

### 選擇：Modular Monolith（模組化單體）

以單一 Rust 二進位檔部署，內部劃分明確的模組邊界。每個模組擁有獨立的公開介面（Rust traits）、資料庫 schema 分區，以及定義清楚的依賴方向。

### 考慮過的替代方案

| 方案 | 優點 | 缺點 | 結論 |
|------|------|------|------|
| **純單體（無邊界）** | 開發速度快、最簡單 | 模組間耦合嚴重，未來拆分困難，團隊協作易衝突 | 否決 |
| **微服務** | 獨立部署、技術棧自由、故障隔離 | 分散式複雜度高、部署與監控成本大、初期團隊規模不足以支撐 | 否決 |
| **Modular Monolith** | 清晰邊界、單一部署簡化營運、共用 DB 降低一致性成本、未來可拆分 | 需紀律維持邊界、無法獨立擴展單一模組 | **採用** |

### 設計理由

- **清晰邊界實現未來拆分**：每個模組透過 Rust trait 定義公開 API，禁止跨模組直接存取資料庫，使得未來將模組抽離為獨立服務時，只需將 trait 替換為 RPC 呼叫。
- **單一部署簡化營運**：初期僅需部署一個二進位檔、一組資料庫連線、一套監控。大幅降低 DevOps 負擔。
- **共用資料庫搭配模組專屬 Schema**：各模組在同一資料庫中擁有獨立的表（table），跨模組引用透過 UUID 外鍵。應用層禁止跨模組 JOIN，確保資料存取邊界。

## 3. 模組定義

### 3.1 auth 模組（認證與授權）

**職責：**
- Passkey / WebAuthn 註冊與認證
- Email Magic Link 產生與驗證
- JWT token 簽發與驗證
- Session 管理

**對外介面：**
- `authenticate(credential) -> AuthResult`
- `issue_token(account_id) -> JWT`
- `validate_token(token) -> Claims`
- `create_magic_link(email) -> MagicLink`
- `register_passkey(account_id, attestation) -> PasskeyCredential`

**依賴：** 無（獨立模組）

---

### 3.2 core 模組（核心業務）

**職責：**
- Account、Organization、Project、Member、Contact、MemberTag 管理
- Task、TaskTemplate 管理
- Todo 管理
- DataSheet / DataEntry 管理
- 權限計算（RBAC，deny-first 策略）

**對外介面：**
- 各實體的 CRUD 操作
- `check_permission(actor, resource, action) -> PermissionResult`
- `list_todos(account_id, filters) -> Vec<Todo>`
- `get_data_entries(data_sheet_id) -> Vec<DataEntry>`

**依賴：** auth（取得身份資訊）

---

### 3.3 conversation 模組（對話系統）

**職責：**
- Task Conversation（Message）管理
- CRDT 操作（基於 yrs 實作即時同步）
- WebSocket 連線與 awareness protocol
- `lastSeenMessageId` 驗證與追蹤

**對外介面：**
- `send_message(task_id, sender, content) -> Message`
- `get_conversation(task_id, pagination) -> Vec<Message>`
- `sync_crdt(task_id, update) -> CrdtResult`
- `update_last_seen(task_id, member_id, message_id)`

**依賴：** core（取得 Task、Member 上下文）

---

### 3.4 ai 模組（AI 協作）

**職責：**
- AI-Human 協作迴圈（trigger → suggestion → decision → execution → record）
- 上下文蒐集（memory chain + schema + 遮蔽後的對話歷史）
- 隱私引擎（placeholder resolution、資料遮蔽）
- SuggestionGroup 產生與生命週期管理
- LLM API 客戶端（透過 Tokio 非同步呼叫）

**對外介面：**
- `trigger_suggestion(task_id, trigger_event) -> SuggestionGroup`
- `apply_decision(suggestion_id, decision) -> ExecutionResult`
- `collect_context(task_id) -> AiContext`
- `mask_data(data) -> MaskedData`

**依賴：** core、conversation、tools（Memory 功能由 AI 模組內部管理，非獨立模組）

---

### 3.5 tools 模組（工具執行）

**職責：**
- MCP 工具執行環境（內建工具以 Rust 實作，外部工具透過 STDIO/SSE 連接）
- 工具設定管理（Organization → Project 繼承機制）
- 工具執行與參數解析（placeholder 替換）

**對外介面：**
- `execute_tool(tool_name, params, context) -> ToolResult`
- `list_available_tools(project_id) -> Vec<ToolDefinition>`
- `resolve_placeholders(params, context) -> ResolvedParams`

**依賴：** core（取得專案與組織設定）

---

### 3.6 email 模組（郵件處理）

**職責：**
- 收信：HTTP Webhook 接收（AWS SES Lambda / Cloudflare Email Worker），使用 mail-parser 解析
- 寄信：透過 lettre + SMTP 發送
- 信件串聯比對（In-Reply-To / References headers）
- 寄件人身份解析（Member 或 Contact）

**對外介面：**
- `receive_mail(raw_mail) -> ProcessedMail`
- `send_mail(to, subject, body, reply_to) -> SendResult`
- `match_thread(headers) -> Option<TaskId>`
- `resolve_sender(email_address) -> SenderIdentity`

**依賴：** core（取得成員與聯絡人資訊）、conversation（將郵件歸入對話）

---

### 3.7 notifications 模組（通知系統）

**職責：**
- 通知派送（Web Push、Email、in-app）
- 提醒排程（截止日期逼近/逾期、停滯的待辦事項）
- 通知偏好路由

**對外介面：**
- `dispatch_notification(recipient, notification) -> DispatchResult`
- `schedule_reminder(reminder_config) -> ScheduledReminder`
- `get_notification_preferences(account_id) -> NotificationPreferences`

**依賴：** core（取得成員與待辦資訊）

---

### 3.8 storage 模組（檔案儲存）

**職責：**
- 檔案/附件儲存（本地檔案系統，未來可擴展至 S3）
- 上傳/下載管理

**對外介面：**
- `upload_file(data, metadata) -> FileHandle`
- `download_file(file_id) -> FileStream`
- `delete_file(file_id) -> DeleteResult`
- `generate_download_url(file_id, expiry) -> Url`

**依賴：** 無（獨立基礎設施模組）

---

### 3.9 audit 模組（審計日誌）

**職責：**
- AuditLog 記錄
- 全域審計查詢

**對外介面：**
- `record_event(event: AuditEvent)`
- `query_logs(filters) -> Vec<AuditLog>`

**依賴：** 所有模組（接收各模組事件）

## 4. 模組依賴圖

```mermaid
graph TD
    auth["auth<br/>認證與授權"]
    core["core<br/>核心業務"]
    conversation["conversation<br/>對話系統"]
    ai["ai<br/>AI 協作"]
    tools["tools<br/>工具執行"]
    email["email<br/>郵件處理"]
    notifications["notifications<br/>通知系統"]
    storage["storage<br/>檔案儲存"]
    audit["audit<br/>審計日誌"]

    core --> auth
    conversation --> core
    ai --> core
    ai --> conversation
    ai --> tools
    tools --> core
    email --> core
    email --> conversation
    notifications --> core

    audit -.->|事件訂閱| auth
    audit -.->|事件訂閱| core
    audit -.->|事件訂閱| conversation
    audit -.->|事件訂閱| ai
    audit -.->|事件訂閱| tools
    audit -.->|事件訂閱| email
    audit -.->|事件訂閱| notifications
    audit -.->|事件訂閱| storage
```

> 實線箭頭表示直接依賴（透過 trait 呼叫），虛線箭頭表示事件訂閱（透過 event bus）。

## 5. 模組間通訊

### 同步呼叫（in-process function calls）

所有模組運行在同一個 Rust 二進位檔中，模組間的同步呼叫透過 Rust trait 實現：

```rust
// 每個模組定義自己的公開 trait
pub trait CoreService: Send + Sync {
    async fn get_task(&self, task_id: Uuid) -> Result<Task>;
    async fn check_permission(&self, actor: Uuid, resource: Uuid, action: Action) -> Result<bool>;
    // ...
}

// 其他模組透過 trait object 呼叫
pub struct ConversationModule {
    core: Arc<dyn CoreService>,
}
```

### 非同步事件（event bus）

跨模組的非同步通知透過 Tokio channels 實現：

```rust
// 事件定義
pub enum DomainEvent {
    MessageSent { task_id: Uuid, message_id: Uuid },
    TodoCompleted { todo_id: Uuid, completed_by: Uuid },
    SuggestionApplied { suggestion_id: Uuid, decision: Decision },
    FileUploaded { file_id: Uuid, uploader: Uuid },
    // ...
}

// 事件發布
event_bus.publish(DomainEvent::MessageSent { task_id, message_id }).await;
```

### 通訊規則

- **禁止跨模組直接存取資料庫**：每個模組擁有自己的表，其他模組必須透過公開 trait 取得資料。
- **事件用於旁路關注點**：審計日誌、通知派送等不影響主流程的邏輯，透過事件匯流排解耦。
- **Tokio channels 進行非同步事件傳播**：使用 `tokio::sync::broadcast` 或 `mpsc` channel，確保事件傳遞不阻塞主流程。

## 6. 資料庫 Schema 分區

各模組擁有的資料表如下：

| 模組 | 資料表 |
|------|--------|
| **auth** | `accounts`, `passkey_credentials`, `magic_link_tokens`, `refresh_tokens` |
| **core** | `organizations`, `projects`, `members`, `contacts`, `member_tags`, `tasks`, `task_templates`, `todos`, `todo_templates`, `data_schemas`, `data_entries`, `permissions`, `profile_data` |
| **conversation** | `messages`, `conversation_states`, `last_seen_positions` |
| **ai** | `suggestion_groups`, `suggestions`, `ai_contexts`, `memories` |
| **tools** | `tool_configs`, `tool_executions` |
| **email** | `email_messages`, `email_threads` |
| **notifications** | `notifications`, `scheduled_reminders`（通知偏好存放於 `accounts.notification_preferences` JSONB 欄位，由 notifications 模組透過 core 模組 API 讀取） |
| **storage** | `files`, `file_metadata` |
| **audit** | `audit_logs` |

### 跨模組引用規則

- 跨模組的資料引用一律使用 UUID 外鍵（例如 `messages.task_id` 引用 `tasks.id`）。
- **應用程式碼禁止跨模組 JOIN**：需要關聯資料時，透過目標模組的公開 API 查詢，而非直接在 SQL 中 JOIN 其他模組的表。
- 資料庫層級的外鍵約束仍可存在（確保引用完整性），但查詢邏輯必須遵守模組邊界。

## 7. 未來拆分策略

### 優先拆分候選

以下模組因具備較高的獨立性與特殊的運行需求，適合優先從單體中拆離：

1. **email 模組**：郵件處理（MIME 解析、附件存儲）的資源需求與主應用不同，且可獨立擴展。拆離後收信轉發函式改為呼叫獨立 email 服務。
2. **ai 模組**：LLM API 呼叫延遲高且資源密集，獨立部署可避免影響主應用的回應速度，也方便針對 GPU/記憶體需求獨立擴展。
3. **notifications 模組**：通知派送屬於背景任務，獨立部署後可專注於排程與重試邏輯，不佔用主應用資源。

### 拆分路徑

```mermaid
graph LR
    subgraph Phase1["階段一：當前"]
        mono["Modular Monolith<br/>單一二進位檔"]
    end

    subgraph Phase2["階段二：部分拆離"]
        main["主服務"]
        email_svc["email 服務"]
        ai_svc["ai 服務"]
        notif_svc["notifications 服務"]
        main <-->|gRPC / MQ| email_svc
        main <-->|gRPC / MQ| ai_svc
        main <-->|gRPC / MQ| notif_svc
    end

    subgraph Phase3["階段三：完全微服務"]
        auth_s["auth"]
        core_s["core"]
        conv_s["conversation"]
        ai_s["ai"]
        tools_s["tools"]
        email_s["email"]
        notif_s["notifications"]
        storage_s["storage"]
        audit_s["audit"]
    end

    Phase1 --> Phase2 --> Phase3
```

### 拆分啟用機制

- **Trait 替換為 RPC**：模組公開的 trait 在單體中是直接函式呼叫，拆分後替換為 gRPC 客戶端實作，呼叫端程式碼不需修改。
- **Event bus 替換為 Message Queue**：Tokio channel 替換為 NATS、RabbitMQ 或 Kafka 等訊息佇列，事件訂閱端程式碼維持相同介面。
- **獨立資料庫**：拆離的模組遷移至獨立的資料庫實例，透過 API 取代外鍵引用。

## 8. 設定項

系統運行所需的主要環境變數：

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `DATABASE_URL` | PostgreSQL 連線字串 | `postgres://user:pass@localhost:5432/confops` |
| `CACHE_MAX_CAPACITY` | In-memory 快取最大容量 | `10000` |
| `CACHE_TTL_SECONDS` | In-memory 快取 TTL（秒） | `300` |
| `AUTH_JWT_SECRET` | JWT 簽發密鑰 | `your-secret-key` |
| `AUTH_JWT_ACCESS_EXPIRY` | JWT Access Token 過期秒數 | `900` |
| `STORAGE_BACKEND` | 儲存後端：`local`（預設）或 `s3`（未來） | `local` |
| `STORAGE_BASE_PATH` | 本地儲存根目錄 | `./data/files` |
| `SMTP_HOST` | SMTP 伺服器位址 | `smtp.example.com` |
| `SMTP_PORT` | SMTP 埠號 | `587` |
| `SMTP_USERNAME` | SMTP 帳號 | `noreply@confops.dev` |
| `SMTP_PASSWORD` | SMTP 密碼 | `password` |
| `EMAIL_INBOUND_API_KEY` | 收信轉發函式認證 token | `secret-key` |
| `LLM_API_URL` | Gemini API 端點 | `https://generativelanguage.googleapis.com/v1beta` |
| `LLM_API_KEY` | Gemini API 金鑰 | `AIza...` |
| `LLM_MODEL` | 預設 LLM 模型名稱 | `gemini-2.0-flash` |
| `AUTH_WEBAUTHN_RP_ID` | WebAuthn Relying Party ID | `confops.dev` |
| `AUTH_WEBAUTHN_RP_ORIGIN` | WebAuthn Relying Party Origin | `https://confops.dev` |
| `WEB_PUSH_VAPID_PRIVATE_KEY` | Web Push VAPID 私鑰 | `base64-encoded-key` |
| `WEB_PUSH_VAPID_PUBLIC_KEY` | Web Push VAPID 公鑰 | `base64-encoded-key` |
| `APP_HOST` | HTTP 伺服器監聽位址 | `0.0.0.0` |
| `APP_PORT` | HTTP 伺服器監聽埠號 | `8080` |
| `APP_LOG_LEVEL` | 日誌等級 | `info,confops=debug` |
