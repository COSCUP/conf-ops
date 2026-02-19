# Plans 與規格文件交叉比對報告

**產生日期：** 2026-02-18
**比對範圍：** `docs/plans/` (13 個 Phase) vs `docs/api/` (24 檔) vs `docs/data-model/` (18 檔) vs `docs/system/` (13 檔) vs `docs/architecture.md` vs `docs/development-guidelines.md`

---

# 第一次比對（2026-02-18）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ Message source_type 枚舉不一致

- **修復**：在 `docs/data-model/09-task-conversation.md` 新增 `email_inbound` 為第 5 種 source_type，含完整 JSONB content 結構定義與業務規則

### 2. ✅ CRDT 適用範圍不明確

- **修復**：
  - `docs/plans/phase-04-task-system.md`：B-4.3 和 B-4.4 補充 todos (Y.Map) 和 data_entries (Y.Map) 的 CRDT 寫入邏輯與管理欄位說明
  - `docs/plans/phase-05-conversation-crdt.md`：B-5.2 新增整合 Phase 4 CRDT 實體的說明，明確 todos/data_entries/memories 的同步時程

### 3. ✅ deny-first 權限引擎實作時程模糊

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.4 新增 `DenyFirstEngine` trait 參考、`PermissionSettings` 結構體定義（含 tool_permissions, task_visibility, data_access, memory_access）

### 4. ✅ AI Pipeline 事件持久化缺失

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 將事件佇列改為 PostgreSQL `ai_pipeline_events` 持久化表實作，含 NOTIFY/LISTEN、SELECT FOR UPDATE SKIP LOCKED worker, 重試策略、超時處理

### 5. ✅ `reminders` 資料表命名不一致

- **修復**：在 `docs/data-model/15-notification.md` 將 `reminders` 全面改為 `scheduled_reminders`（表名、索引、Rust struct、Mermaid 圖）

---

## 高優先問題（HIGH）

### 6. ✅ Contact 表 `notes` 欄位不一致

- **修復**：從 `docs/plans/phase-03-member-tag-contact.md` B-3.2 移除 `notes` 欄位，與 Data Model 一致

### 7. ✅ Account 更新 HTTP method 不一致

- **修復**：在 `docs/api/paths/accounts.yaml` 將 `/accounts/me` 更新方法從 `PUT` 改為 `PATCH`，與 Phase 1 Plan 一致

### 8. ✅ 個人待辦端點路徑不一致

- **修復**：在 `docs/plans/phase-04-task-system.md` 將 `/my/todos` 改為 `/accounts/me/todos`，與 API Spec 一致

### 9. ✅ AI 建議 re-suggest 參數命名不一致

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.4 將 `"instructions"` 改為 `"additionalInstructions"`，與 API Spec 一致

### 10. ✅ Privacy Engine 缺少實作細節

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.2 補充 `aho-corasick` crate 依賴、最長匹配優先策略、字典排除過短值（< 2 字元）、moka cache TTL 10 分鐘

### 11. ✅ Gemini Context Cache 未安排獨立實作任務

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 補充 `GeminiCacheManager` 結構體定義、moka cache 管理、get_or_create/invalidate 方法、streamGenerateContent 串流接收

### 12. ✅ API Spec 缺少多個端點文件

- **修復**：
  - 新建 `docs/api/paths/api-keys.yaml`：API Key CRUD（list, create, revoke）
  - 新建 `docs/api/paths/audit-logs.yaml`：審計日誌查詢（organization, project 層級）
  - 在 `docs/api/paths/data-external.yaml` 新增 `PUT` 外部資料寫入端點
  - 在 `docs/api/openapi.yaml` 新增 ApiKeys、AuditLogs tags 與所有路徑 $ref

---

## 中優先問題（MEDIUM）

### 13. ✅ Webhook 事件類型不一致

- **修復**：在 `docs/api/paths/webhooks.yaml` 修正範例中的 `suggestion.created`（不存在的事件類型）為 `data_entry.updated`。API schema 的事件列表已包含所有 Phase 11 定義的事件

### 14. ✅ Todo `due_date` 類型模糊

- **修復**：在 `docs/data-model/10-todo.md` 將 `due_date` 從 `DATE` 改為 `TIMESTAMPTZ`（SQL、欄位說明、Rust struct、Mermaid 圖），與 Phase 4 Plan 一致

### 15. ✅ Contact 合併 auto-follow 查詢邏輯未在 Plan 中說明

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.2 新增 auto-follow 查詢機制說明

### 16. ✅ WebSocket 環境變數命名未對齊

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.3 補充完整環境變數名稱：`CRDT_WS_MAX_CONNECTIONS_PER_TASK`、`CRDT_WS_HEARTBEAT_INTERVAL_SECS`、`CRDT_WS_IDLE_TIMEOUT_SECS`，含預設值

### 17. ✅ Memory 繼承鏈查詢缺少效能要求

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.1 補充效能要求 < 5ms，並在測試要求新增效能測試項目

### 18. ✅ 審計日誌月分區策略不完整

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` B-11.3 補充分區自動建立排程（Tokio scheduled task，每月 1 日建立未來 3 個月分區）、保留策略（預設 12 個月，`AUDIT_RETENTION_DAYS` 環境變數）

---

## 低優先問題（LOW）

### 19. ✅ `profile_schema` 欄位在 Data Model README 概覽表中遺漏

- **修復**：在 `docs/data-model/README.md` JSONB 欄位表格新增 `profile_schema` 列

### 20. ✅ Webhook `webhook_logs` 表命名與 Data Model 不一致

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 將 `webhook_logs` 全面改為 `webhook_event_logs`，與 Data Model (`docs/data-model/17-webhook.md`) 一致

### 21. ✅ Notification preferences 端點位置不一致

- **修復**：在 `docs/api/paths/notifications.yaml` 將端點從 `/notifications/preferences` 改為 `/accounts/me/notification-preferences`，在 `docs/api/openapi.yaml` 更新對應 $ref

### 22. ✅ WebSocket 訊息類型 schema 定義缺失

- **修復**：在 `docs/api/paths/conversations.yaml` 的 `x-websocket-frame-types` 說明中補充 CRDT binary frame 包含的協定訊息類型（SyncStep1、SyncStep2、AwarenessUpdate），並引用 `docs/system/03-crdt-implementation.md`

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 5 | 5 ✅ |
| HIGH | 7 | 7 ✅ |
| MEDIUM | 6 | 6 ✅ |
| LOW | 4 | 4 ✅ |
| **合計** | **22** | **22 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/data-model/09-task-conversation.md` | 新增 email_inbound source_type |
| `docs/data-model/10-todo.md` | due_date DATE → TIMESTAMPTZ |
| `docs/data-model/15-notification.md` | reminders → scheduled_reminders |
| `docs/data-model/README.md` | 新增 profile_schema 到 JSONB 表 |
| `docs/api/openapi.yaml` | 新增 tags、路徑 $ref、修正描述 |
| `docs/api/paths/accounts.yaml` | PUT → PATCH |
| `docs/api/paths/notifications.yaml` | 端點路徑修正 |
| `docs/api/paths/webhooks.yaml` | 修正範例事件類型 |
| `docs/api/paths/conversations.yaml` | 補充 CRDT binary frame 說明 |
| `docs/api/paths/data-external.yaml` | 新增 PUT 外部寫入端點 |
| `docs/api/paths/api-keys.yaml` | **新建** — API Key CRUD |
| `docs/api/paths/audit-logs.yaml` | **新建** — 審計日誌查詢 |
| `docs/plans/phase-03-member-tag-contact.md` | 移除 notes、補充 auto-follow、DenyFirstEngine |
| `docs/plans/phase-04-task-system.md` | 補充 CRDT 範圍、修正 todos 路徑 |
| `docs/plans/phase-05-conversation-crdt.md` | 補充 CRDT 整合、WS 環境變數 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | AI 事件持久化、Privacy Engine 細節、Gemini Cache、參數命名、效能要求 |
| `docs/plans/phase-11-webhook-audit-observability.md` | webhook_event_logs 命名、分區策略 |

---

# 第二次比對（2026-02-18）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ SuggestionGroup JSONB 欄位命名不一致

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 將 SuggestionGroup JSONB 結構全面改為與 `docs/data-model/09-task-conversation.md` 一方的欄位命名（`trigger`、`tool`、`parameters`、`contextUsed`、`decision`、`decidedBy`、`decidedAt`、`executionResult`），移除不存在的 `triggerMessageId` 和群組 `status`

### 2. ✅ Todo 資料表 migration 缺少 3 個欄位

- **修復**：在 `docs/plans/phase-04-task-system.md` B-4.3 的 `todos` 表欄位列表中補充 `source_template_id (UUID, FK → todo_templates, nullable)`、`completed_at (TIMESTAMPTZ, nullable)`、`completed_by (UUID, FK → accounts, nullable)`，與 `docs/data-model/10-todo.md` 一致

### 3. ✅ Messages 資料表 migration 缺少 `last_seen_message_id` 欄位

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.1 的 `messages` 表欄位列表中補充 `last_seen_message_id (UUID, nullable — 寫入操作時客戶端附帶的已讀訊息 ID)`，與 Data Model 一致

### 4. ✅ Magic Link verify HTTP method 不一致

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.3 將 `POST /api/v1/auth/magic-link/verify` 改為 `GET`，並將 `/auth/magic-link` 改為 `/auth/magic-link/request`，與 API Spec 一致

---

## 高優先問題（HIGH）

### 5. ✅ Organization 更新 HTTP method 不一致

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.1 將 `PATCH` 改為 `PUT`，與 API Spec 一致（完整替換語義）

### 6. ✅ Organization 成員路徑參數不一致

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.1 將 `/members/{accountId}` 改為 `/members/{memberId}`，與 API Spec 一致

### 7. ✅ MemberTag 更新 HTTP method 不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.3 將 `PATCH` 改為 `PUT`，與 API Spec 一致

### 8. ✅ MemberTag 指派端點路徑不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.3 將 `POST .../assignments` 改為 `POST .../assign`，保留 `DELETE .../assignments/{assignmentId}`（取消指派），與 API Spec 一致

### 9. ✅ Placeholder 類型範圍不一致

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.2 移除 `{{contact.*}}` 和 `{{task.*}}` placeholder 類型，僅保留 `{{profile.*}}` 和 `{{data.*}}`，與 `docs/system/05-privacy-engine.md` 的正則表達式一致

### 10. ✅ `crdt_operations` 表建立時機不明確

- **修復**：
  - 在 `docs/plans/phase-04-task-system.md` B-4.3 新增 `crdt_operations` 表的建立（在 `0011_todos.sql` 或獨立 migration 中），含完整欄位定義
  - 在 `docs/plans/phase-05-conversation-crdt.md` B-5.1 將 `crdt_operations` 表改為「已在 Phase 4 migration 中建立，本 Phase 驗證該表已存在」

### 11. ✅ CI Pipeline 缺少型別生成同步驗證

- **修復**：
  - 在 `docs/plans/phase-00-scaffolding.md` S-0.4 CI Pipeline 中補充 `cargo xtask generate-openapi --check` 和 `npx openapi-typescript` 型別同步驗證步驟
  - 在 `docs/plans/phase-01-auth-account.md` B-1.1 驗收標準中新增 `cargo xtask generate-openapi --check` 通過項目，作為所有 API 變更 Phase 的範例

### 12. ✅ Phase 1 Magic Link 請求路徑不一致

- **修復**：與問題 #4 一併修復，在 `docs/plans/phase-01-auth-account.md` B-1.3 將 `POST /api/v1/auth/magic-link` 改為 `POST /api/v1/auth/magic-link/request`

---

## 中優先問題（MEDIUM）

### 13. ✅ 各 Phase Plan 未提及禁止 `#[allow(...)]` 規則

- **修復**：確認所有 Phase Plan 的 clippy 驗收標準皆已包含括號標註「零警告（無使用 `#[allow(...)]` 忽略）」，各 Phase 已一致

### 14. ✅ 前端 Block 順序要求未在 Plan 中提及

- **修復**：此要求由 ESLint 規則（`@antfu/eslint-config` 或等效配置）自動覆蓋，各 Phase 前端驗收標準已包含 `npm run lint -- --max-warnings 0` 零警告，隱含涵蓋 block 順序檢查

### 15. ✅ `pg_lite` 測試要求未在 Plan 中標註

- **修復**：Phase 0 B-0.1 已包含 `pg_lite` 整合設定（TestContext 結構體使用 `pg_lite` 管理測試用 PostgreSQL），各 Phase 測試隱含遵循此準則，不需逐一標註

### 16. ✅ 缺少 `spawn_blocking` 使用指引

- **修復**：此為通用 Rust 開發準則（已在 `docs/development-guidelines.md` 3.2 節明確說明），不需在每個 Phase 重複。開發者在實作 CPU 密集型或阻塞操作（如 bcrypt、檔案 I/O、SMTP）時應自行遵循

### 17. ✅ 跨模組 JOIN 禁止規則在部分 Phase 可能被違反

- **修復**：在 `docs/plans/phase-04-task-system.md` B-4.2 的參與人計算邏輯中明確說明「透過各模組 Service 層聚合，禁止跨模組 SQL JOIN」，分別透過 MemberTagService、ConversationService、TodoService 查詢後在應用層合併

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 4 | 4 ✅ |
| HIGH | 8 | 8 ✅ |
| MEDIUM | 5 | 5 ✅ |
| LOW | 3 | 3 ✅ |
| **合計** | **20** | **20 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/plans/phase-08-ai-pipeline-privacy.md` | SuggestionGroup JSONB 欄位命名修正、Placeholder 類型縮減為 profile/data |
| `docs/plans/phase-04-task-system.md` | Todo migration 補充 3 欄位、新增 crdt_operations 表建立、參與人計算策略說明 |
| `docs/plans/phase-05-conversation-crdt.md` | Messages 表補充 last_seen_message_id、crdt_operations 改為引用 Phase 4 |
| `docs/plans/phase-01-auth-account.md` | Magic Link 路徑與 HTTP method 修正、新增型別同步驗收項 |
| `docs/plans/phase-02-org-project.md` | Organization 更新 PATCH→PUT、成員路徑 accountId→memberId |
| `docs/plans/phase-03-member-tag-contact.md` | MemberTag 更新 PATCH→PUT、指派端點 assignments→assign |
| `docs/plans/phase-00-scaffolding.md` | 新增 xtask 設定任務、CI Pipeline 補充型別同步驗證 |

---

# 第三次比對（2026-02-18）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ `email_messages` 資料表結構重大不一致

- **修復**：在 `docs/plans/phase-07-email-integration.md` B-7.1 將 `email_messages` 表定義統一為 Data Model 結構（移除 `task_id`、`status`、`sender_type`、`sender_id`、`attachments`、`error_message`、`retry_count`、`sent_at`、`body_html`、`body_text` 等多餘欄位，新增 `conversation_message_id` FK）

### 2. ✅ `magic_link_tokens.account_id` 可 Null 性矛盾

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.1 將 `account_id` 從 `nullable` 改為 `NOT NULL`，因 B-1.3 流程為先建帳號再發 token，與系統認證文件一致

### 3. ✅ AI 建議決策 API 結構不一致（批次 vs 個別）

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.4 將批次決策端點改為逐一決策端點 `POST .../suggestions/{suggestionId}/decide`，與 API Spec 一致

---

## 高優先問題（HIGH）

### 4. ✅ `AUTH_JWT_ACCESS_EXPIRY` 環境變數預設值不一致

- **修復**：在 `docs/system/02-deployment-architecture.md` 將 `.env` 範例與環境變數表的 `AUTH_JWT_ACCESS_EXPIRY` 從 `3600` 改為 `900`（15 分鐘）

### 5. ✅ Memory 模組未正式定義但被引用為依賴

- **修復**：在 `docs/system/01-service-decomposition.md` 將 AI 模組依賴列表中的 `memory` 改為括號說明「Memory 功能由 AI 模組內部管理，非獨立模組」

### 6. ✅ Member 邀請端點路徑不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.1 將 `POST .../members` 改為 `POST .../members/invite`，與 API Spec 一致

### 7. ✅ 多個端點 HTTP method 不一致（PATCH vs PUT）

- **修復**：將以下 5 處 Plan 中的 `PATCH` 改為 `PUT`：
  - `docs/plans/phase-03-member-tag-contact.md` B-3.1 members/{memberId}、B-3.2 contacts/{contactId}
  - `docs/plans/phase-02-org-project.md` B-2.2 projects/{projectId} 與 projects/{projectId}/status
  - `docs/plans/phase-04-task-system.md` B-4.1 task-templates/{templateId}
  - `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.1 memories/{memoryId}

### 8. ✅ Notification 相關端點路徑與方法多處不一致

- **修復**：在 `docs/plans/phase-10-notifications.md` B-10.1 將所有 4 個端點更新為與 API Spec 一致：
  - `PATCH` → `PUT` 標記已讀
  - `POST .../mark-all-read` → `PUT .../read-all`
  - `POST /web-push/subscribe` → `POST /notifications/web-push/subscribe`
  - `DELETE /web-push/subscribe` → `DELETE /notifications/web-push/subscriptions/{endpoint}`

### 9. ✅ SuggestionGroup trigger 類型缺少 `manual_request`

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 的 trigger 枚舉與觸發偵測列表中補充 `manual_request`（對應 `POST /tasks/{taskId}/suggestions/request` 手動觸發端點）

### 10. ✅ `audit_logs.actor_type` 缺少 `api_key` 枚舉值

- **修復**：在 `docs/data-model/14-audit-log.md` 的 SQL 定義、Rust enum、Business Rules 表格中補充 `api_key` 枚舉值

---

## 中優先問題（MEDIUM）

### 11. ✅ Contact 列表端點在 API Spec 中缺失

- **修復**：在 `docs/api/paths/contacts.yaml` 的 `/organizations/{orgId}/contacts` 路徑新增 GET 列表端點（operationId: listContacts），支援游標分頁與搜尋

### 12. ✅ Member 表欄位命名不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.1 將 `member_role (ENUM)` 改為 `role (member_role ENUM)`，區分欄位名與類型名

### 13. ✅ WebSocket 端點路徑不一致

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.3 將 `GET /api/v1/ws` 改為 `GET /api/v1/tasks/{taskId}/conversation/ws`，與 API Spec 一致

### 14. ✅ 附件 JSONB 結構 `fileId` 欄位不一致

- **修復**：在 `docs/data-model/09-task-conversation.md` 的附件結構中補充 `fileId` (UUID) 欄位，與 Phase 6 計劃一致

### 15. ✅ Phase 0 缺少可觀測性基礎設施任務

- **修復**：在 `docs/plans/phase-00-scaffolding.md` B-0.2 新增註記：`/metrics` 端點與完整可觀測性基礎設施延遲至 Phase 11 (B-11.4) 實作

### 16. ✅ Organization 刪除的業務規則限制未在 Plan 中說明

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.1 驗收標準補充：組織刪除時若有進行中專案回傳 409 Conflict

### 17. ✅ Email Webhook 處理細節不完整

- **修復**：在 `docs/plans/phase-07-email-integration.md` B-7.2 補充來源識別與驗證說明（AWS SES `X-Amz-Sns-*` / Cloudflare `CF-*` headers 區分與對應解析邏輯）

### 18. ✅ Webhook 回應狀態欄位命名不一致

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 的 `response_status` 欄位補充說明：DB 欄位名為 `response_status`，API 回應時映射為 `statusCode`

### 19. ✅ Notification 偏好設定結構差異

- **修復**：在 `docs/plans/phase-10-notifications.md` 補充註記：`due_date_approaching`、`due_date_overdue`、`todo_stale` 在 API Schema 中統一歸類為 `reminder`，後端儲存仍使用細粒度類型

### 20. ✅ 環境變數命名前綴不一致（Web Push）

- **修復**：在 `docs/plans/phase-10-notifications.md` 補充環境變數統一使用 `WEB_PUSH_VAPID_*` 前綴（不使用 `NOTIFICATION_` 前綴）

---

## 低優先問題（LOW）

### 21. ✅ Phase 1 缺少 GET notification-preferences 端點

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.5 補充 `GET /api/v1/accounts/me/notification-preferences` 端點

### 22. ✅ Email 信件串比對缺少 GIN 索引說明

- **修復**：在 `docs/plans/phase-07-email-integration.md` B-7.2 精確比對說明中補充 GIN 索引名稱 `idx_email_threads_message_ids` 與 Data Model 參照

### 23. ✅ Webhook test 事件類型未列入 Phase 11 計劃

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 事件列表補充 `test` 事件類型

### 24. ✅ `notification_preferences` 預設值結構不完整

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.1 的 `notification_preferences` 欄位補充說明 Repository 層查詢時自動合併預設值結構

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 7 | 7 ✅ |
| MEDIUM | 10 | 10 ✅ |
| LOW | 4 | 4 ✅ |
| **合計** | **24** | **24 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/plans/phase-01-auth-account.md` | magic_link NOT NULL、GET notification-preferences、預設值說明 |
| `docs/plans/phase-02-org-project.md` | PATCH→PUT、Org 刪除 409 規則 |
| `docs/plans/phase-03-member-tag-contact.md` | invite 路徑、PATCH→PUT（members + contacts）、role 欄位命名 |
| `docs/plans/phase-04-task-system.md` | PATCH→PUT |
| `docs/plans/phase-05-conversation-crdt.md` | WebSocket 路徑改為 per-task |
| `docs/plans/phase-07-email-integration.md` | email_messages 結構統一、Webhook 來源驗證、GIN 索引說明 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | 決策 API 改為逐一、PATCH→PUT、manual_request trigger |
| `docs/plans/phase-10-notifications.md` | 端點路徑/方法統一、偏好結構說明、VAPID 環境變數前綴 |
| `docs/plans/phase-11-webhook-audit-observability.md` | response_status 映射說明、test 事件類型 |
| `docs/plans/phase-00-scaffolding.md` | 可觀測性延遲至 Phase 11 說明 |
| `docs/data-model/14-audit-log.md` | actor_type 補充 api_key |
| `docs/data-model/09-task-conversation.md` | 附件結構補充 fileId |
| `docs/api/paths/contacts.yaml` | **新增** GET 列表端點 |
| `docs/system/01-service-decomposition.md` | Email 表名修正 |
| `docs/system/02-deployment-architecture.md` | JWT 過期時間 3600→900 |

---

# 第四次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ NotificationPreferences schema 三方不一致

- **修復**：統一所有檔案為 `channels` 包裝結構（含 `email`/`web_push`/`in_app`），使用 `categories`（5 種：task_updates/todo_assignments/ai_suggestions/mentions/system_announcements）。Data Model 使用 snake_case，API Schema 使用 camelCase
  - `docs/data-model/01-account.md`：新增 `in_app` 通道
  - `docs/api/schemas/notifications.yaml`：改為 `channels` 結構，新增 `inApp`，`types` → `categories`
  - `docs/api/schemas/entities.yaml`：新增 `inApp` 通道
  - `docs/api/schemas/accounts.yaml`：新增 `inApp` 通道
  - `docs/api/paths/notifications.yaml`：更新 GET/PUT 範例為新結構
  - `docs/plans/phase-10-notifications.md`：對齊統一 schema

### 2. ✅ PermissionSettings 結構三方不一致

- **修復**：Plan 和 System Doc 統一為 Data Model 的 `Vec<PermissionRule>`（tag/allow/deny）結構
  - `docs/plans/phase-02-org-project.md`：`HashMap` → `Vec<PermissionRule>`
  - `docs/system/09-authorization.md`：改為引用 Data Model 結構

### 3. ✅ Storage path 格式不一致

- **修復**：在 `docs/plans/phase-06-file-storage.md` 將路徑格式改為 `{scope_type}/{scope_id}/{year}/{month}/{uuid}/{filename}`，與 System Doc 一致

### 4. ✅ `member_tag_assignments` FK 欄位命名不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` 將 `member_tag_id` → `tag_id`，與 Data Model 一致

### 5. ✅ `tool_configs` 表使用分離 FK 而非多型 scope 模式

- **修復**：在 `docs/plans/phase-09-mcp-tool-runtime.md` 將 `organization_id/project_id` 分離 FK 改為 `scope_type VARCHAR(20) NOT NULL` + `scope_id UUID NOT NULL`，與 Data Model 一致

---

## 高優先問題（HIGH）

### 6. ✅ `member_tag_assignments` CHECK 約束邏輯不一致

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` 將 CHECK 約束改為 XOR：`(member_id IS NOT NULL AND contact_id IS NULL) OR (member_id IS NULL AND contact_id IS NOT NULL)`

### 7. ✅ Contact 合併 API：單一來源 vs 多來源

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` 將 `merge_contacts` 改為支援多來源 `source_ids: Vec<Uuid>`，與 API Spec 一致

### 8. ✅ Organization DELETE 端點未列入 Phase 2 Plan

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.1 新增 `DELETE /api/v1/organizations/{orgId}` 路由

### 9. ✅ `GET /projects/{projectId}/members/{memberId}` 端點未列入 Phase 3 Plan

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.1 新增 `GET /api/v1/projects/{projectId}/members/{memberId}` 端點

### 10. ✅ `PUT /projects/{projectId}/member-tags/{tagId}/external-task-creation` 端點未列入 Phase 3 Plan

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.3 新增 `PUT .../member-tags/{tagId}/external-task-creation` 端點

### 11. ✅ StorageTrait 命名不一致

- **修復**：在 `docs/plans/phase-06-file-storage.md` 將 trait `StorageBackend` → `StorageService`，方法名對齊：`store`→`upload`、`retrieve`→`download`、`delete`→`delete_file`，新增 `cleanup_orphaned_files`

### 12. ✅ 檔案大小限制不一致

- **修復**：在 `docs/plans/phase-06-file-storage.md` 將統一 50MB 改為依類型限制：Image 10MB、Document 50MB、Other 20MB

### 13. ✅ 本地端下載行為不一致（302 vs 200 串流）

- **修復**：在 `docs/plans/phase-06-file-storage.md` 區分下載行為：Local backend 使用 200 OK 串流、S3 backend 使用 302 redirect

### 14. ✅ Inbound email source_type 在 System Doc 中不一致

- **修復**：在 `docs/system/06-email-integration.md` 將 `source_type = 'member'` → `'email_inbound'`

### 15. ✅ Email 模組表名在 service-decomposition 中過時

- **修復**：在 `docs/system/01-service-decomposition.md` 將 `inbound_mails`/`outbound_mails` → `email_messages`/`email_threads`

### 16. ✅ Profile 端點 HTTP method 不一致：Plan PATCH vs Spec PUT

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.5 將 `PATCH /accounts/me/profile` → `PUT`

### 17. ✅ Plan PATCH /accounts/me 遺漏 `locale` 可更新欄位

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.5 更新欄位列表加入 `locale`

### 18. ✅ Magic Link verify 錯誤狀態碼不一致：API 400 vs System 401

- **修復**：在 `docs/system/08-authentication.md` 將 Magic Link 錯誤碼 `401` → `400`，與 API Spec 一致

### 19. ✅ Webhook 更新端點 HTTP method 不一致：Plan PATCH vs Spec PUT

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 將 `PATCH` → `PUT`

### 20. ✅ Notification types 數量不一致：Plan 9 種 vs Spec 7 種

- **修復**：在 `docs/plans/phase-10-notifications.md` 將通知類型從 9 種縮減為 7 種，移除 `task_status_changed`、`suggestion_generated`

### 21. ✅ Notifications 表欄位多處不一致

- **修復**：在 `docs/plans/phase-10-notifications.md` 將 `notification_type` → `type`，移除 `project_name`/`task_name`/`task_id` 非正規化欄位，`delivered_channels` 從 `VARCHAR[]` → `JSONB DEFAULT '[]'`

### 22. ✅ Notification preferences 端點未列入 Phase 10 Plan

- **修復**：在 `docs/plans/phase-10-notifications.md` B-10.1 補充 `GET/PUT /accounts/me/notification-preferences` 路由

### 23. ✅ Phase 5 B-5.1 驗收標準 source_type 數量錯誤

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` 驗收標準從「4 種」→「5 種」source_type

### 24. ✅ `conversation_states` 和 `last_seen_positions` 表缺少 Data Model 定義

- **修復**：
  - 在 `docs/data-model/09-task-conversation.md` 新增 `conversation_states` 和 `last_seen_positions` 表定義（SQL + Rust struct + 索引）
  - 在 `docs/plans/phase-05-conversation-crdt.md` 補充引用 Data Model 定義

### 25. ✅ `contextUsed` 類型不一致：Data Model string[] vs API object[]

- **修復**：
  - 在 `docs/data-model/09-task-conversation.md` 將 `contextUsed` 從 `["string"]` 改為 `SuggestionContextRef` 物件陣列（含 `scopeType`/`memoryId`/`content`）
  - 在 `docs/plans/phase-08-ai-pipeline-privacy.md` 同步更新引用

---

## 中優先問題（MEDIUM）

### 26. ✅ `memory_access` vs `memory_visibility` 欄位命名

- **修復**：在 `docs/plans/phase-02-org-project.md` 將 `memory_access` → `memory_visibility`

### 27. ✅ Members unique constraint 缺少 partial index

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` 的 UNIQUE 約束加上 `WHERE deleted_at IS NULL`

### 28. ✅ Project DELETE 端點未列入 Phase 2 Plan

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.2 新增 `DELETE /api/v1/projects/{projectId}` 路由

### 29. ✅ `GET /projects/{projectId}/member-tags/{tagId}` 端點未列入 Phase 3 Plan

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.3 新增 `GET .../member-tags/{tagId}` 端點

### 30. ✅ Permission settings API 端點未列入 Phase 2 Plan

- **修復**：在 `docs/plans/phase-02-org-project.md` B-2.2 新增 `GET/PUT /projects/{projectId}/permission-settings` 路由

### 31. ✅ 孤立檔案清理寬限期不一致：Plan 30 天 vs System Doc 7 天

- **修復**：在 `docs/plans/phase-06-file-storage.md` 將清理寬限期 30 天 → 7 天

### 32. ✅ Passkey register/complete 回應狀態碼不一致

- **修復**：在 `docs/system/08-authentication.md` 將 Passkey 註冊回應 `201 Created` → `200 OK`

### 33. ✅ Phase 1 Plan migration 缺少多個索引定義

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.1 補充 6 個遺漏索引定義

### 34. ✅ Auth 相關環境變數未列入 deployment architecture

- **修復**：在 `docs/system/02-deployment-architecture.md` Section 6.1 新增 `AUTH_MAGIC_LINK_EXPIRY`、`AUTH_MAGIC_LINK_BASE_URL`、`AUTH_RATE_LIMIT_MAGIC_LINK`

### 35. ✅ `data_entries` migration 缺少 `source_links` 欄位

- **修復**：在 `docs/plans/phase-04-task-system.md` B-4.4 data_entries 表補充 `source_links (JSONB, nullable)`

### 36. ✅ `GET /api/v1/accounts/me/todos` 端點在 API Spec 中缺失

- **修復**：在 `docs/api/paths/todos.yaml` 新增 `GET /accounts/me/todos` 端點定義

### 37. ✅ Webhook `max_attempts` 預設值不一致

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 將 `max_attempts DEFAULT 5` → `DEFAULT 3`

### 38. ✅ Webhook event log 表欄位差異

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 移除非 Data Model 欄位（`error_message`/`response_time_ms`/`is_test`），`attempt_count` → `attempts`

### 39. ✅ Web Push VAPID 環境變數前綴不一致

- **修復**：在 `docs/system/10-notification-system.md` 將 `NOTIFICATION_WEB_PUSH_VAPID_*` → `WEB_PUSH_VAPID_*`

### 40. ✅ `scheduled_reminders` 表 schema 大幅偏離

- **修復**：在 `docs/plans/phase-10-notifications.md` 將 scheduled_reminders 改為 Data Model 結構（`todo_id` FK、`type`、`trigger_at`、`fired`/`fired_at`、`notification_id` FK、`config` JSONB、`updated_at`）

### 41. ✅ Plan B-8.4 決策請求欄位名 `action` vs Spec `decision`

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` 將 `"action"` → `"decision"`

### 42. ✅ `tool_configs` Plan migration 欄位不完整

- **修復**：在 `docs/plans/phase-09-mcp-tool-runtime.md` 補充 `display_name`、`description`、`mcp_server_config JSONB` 欄位和 `UNIQUE(scope_type, scope_id, tool_name)` 約束

### 43. ✅ `manual_request` trigger 未列入 Data Model JSONB 枚舉

- **修復**：在 `docs/data-model/09-task-conversation.md` trigger 枚舉新增 `manual_request`（第 6 種）

### 44. ✅ `readyz` 回應包含 `redis` 檢查但 Plan 未提及

- **修復**：系統不使用 Redis（見 Phase 12），從 API Spec（`health.yaml`、`responses.yaml`、`openapi-bundled.yaml`）移除 redis 檢查欄位。Plan B-0.2 已補充 `/readyz` 含 `checks.database` 欄位說明

---

## 低優先問題（LOW）

### 45. ✅ Phase 3 B-3.1 invite service 方法缺少 `tagIds` 參數

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.1 的 `add_member` 新增 `tag_ids: Option<Vec<Uuid>>` 可選參數

### 46. ✅ Contact 列表端點路徑重複定義

- **修復**：在 `docs/api/paths/organizations.yaml` 移除重複的 `/organizations/{orgId}/contacts` GET 端點（保留 contacts.yaml 中的定義）

### 47. ✅ `refresh_tokens` 表：Plan 含 `rotated_at`/`replaced_by` 但 System Spec Section 6 未定義

- **修復**：在 `docs/system/08-authentication.md` Section 6 的 `refresh_tokens` CREATE TABLE 定義中整合 `rotated_at`/`replaced_by` 欄位（從 Section 8 ALTER 移入）

### 48. ✅ Plan accounts 表欄位缺少明確 NOT NULL 標註

- **修復**：在 `docs/plans/phase-01-auth-account.md` B-1.1 accounts 表欄位補充 `NOT NULL` 標註

### 49. ✅ `SMTP_FROM` 環境變數未列入正式環境變數表

- **修復**：在 `docs/system/02-deployment-architecture.md` Section 6.1 Email 群組新增 `SMTP_FROM`

### 50. ✅ `memories.source` 和 `scope_type` Plan 描述為 ENUM 但 Data Model 為 VARCHAR

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` 將 `source (ENUM)` / `scope_type (ENUM)` → `VARCHAR(20)`

### 51. ✅ `library_documents` 表使用 `organization_id` 而非多型 `scope_type`/`scope_id`

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` 將 `organization_id` → `scope_type/scope_id` 多型模式

### 52. ✅ `upsertMemory` 工具含 `title` 參數但 memories 表無此欄位

- **修復**：在 `docs/plans/phase-09-mcp-tool-runtime.md` 的 `upsertMemory` 移除 `title` 參數

### 53. ✅ Phase 9 tool config API 路由與 API Spec 不一致

- **修復**：在 `docs/plans/phase-09-mcp-tool-runtime.md` 移除不存在的 org-level PUT，新增 project-level DELETE，與 API Spec 對齊

### 54. ✅ `memory_versions` 表遺漏

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` 補充 `memory_versions` 表定義（id, memory_id FK, version_number, content, change_summary, created_by, created_at）

### 55. ✅ Webhook test 端點及 GET 單一 webhook 端點未列入 Phase 11 Plan

- **修復**：在 `docs/plans/phase-11-webhook-audit-observability.md` 補充 `POST .../test`、`GET .../webhooks/{webhookId}`、`POST /webhooks/inbound/{endpointId}` 端點

### 56. ✅ Webhook `test` 事件類型不在 Data Model 支援列表中

- **修復**：在 `docs/data-model/17-webhook.md` 支援事件列表新增 `test` 事件類型

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 5 | 5 ✅ |
| HIGH | 20 | 20 ✅ |
| MEDIUM | 19 | 19 ✅ |
| LOW | 12 | 12 ✅ |
| **合計** | **56** | **56 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/data-model/01-account.md` | 新增 in_app 通道 |
| `docs/data-model/09-task-conversation.md` | 新增 conversation_states/last_seen_positions 表、contextUsed 改為物件陣列、新增 manual_request trigger |
| `docs/data-model/17-webhook.md` | 新增 test 事件類型 |
| `docs/api/schemas/notifications.yaml` | 統一為 channels 結構，新增 inApp，types→categories |
| `docs/api/schemas/entities.yaml` | 新增 inApp 通道 |
| `docs/api/schemas/accounts.yaml` | 新增 inApp 通道 |
| `docs/api/paths/notifications.yaml` | 更新 GET/PUT 範例為新結構 |
| `docs/api/paths/organizations.yaml` | 移除重複 contacts 端點 |
| `docs/api/paths/todos.yaml` | 新增 GET /accounts/me/todos |
| `docs/system/01-service-decomposition.md` | Email 表名修正 |
| `docs/system/02-deployment-architecture.md` | 新增 auth 環境變數、SMTP_FROM |
| `docs/system/06-email-integration.md` | source_type 修正 |
| `docs/system/08-authentication.md` | Magic Link 錯誤碼、Passkey 狀態碼、refresh_tokens 整合 |
| `docs/system/09-authorization.md` | PermissionSettings 引用 Data Model |
| `docs/system/10-notification-system.md` | VAPID 環境變數前綴修正 |
| `docs/plans/phase-00-scaffolding.md` | readyz 檢查說明補充 |
| `docs/api/paths/health.yaml` | 移除 redis 檢查欄位 |
| `docs/api/schemas/responses.yaml` | 移除 redis 檢查欄位 |
| `docs/plans/phase-01-auth-account.md` | Profile PUT、locale、索引、NOT NULL |
| `docs/plans/phase-02-org-project.md` | PermissionSettings、memory_visibility、DELETE 端點、permission-settings 路由 |
| `docs/plans/phase-03-member-tag-contact.md` | tag_id、XOR CHECK、merge 多來源、新增端點、partial index、tagIds |
| `docs/plans/phase-04-task-system.md` | source_links 補充 |
| `docs/plans/phase-05-conversation-crdt.md` | 5 種 source_type、Data Model 引用 |
| `docs/plans/phase-06-file-storage.md` | scope path、StorageService、檔案限制、下載行為、7 天清理 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | decision 欄位、VARCHAR(20)、scope 模式、memory_versions、contextUsed |
| `docs/plans/phase-09-mcp-tool-runtime.md` | scope 模式、欄位補充、移除 title、API 路由對齊 |
| `docs/plans/phase-10-notifications.md` | 7 種類型、表欄位對齊、preferences 路由、reminders 結構 |
| `docs/plans/phase-11-webhook-audit-observability.md` | PUT、max_attempts 3、欄位對齊、新增端點 |

---

# 第五次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ 核心實體命名與 ID 引用矛盾 (Conversation vs Task)

- **修復**：在 `docs/data-model/09-task-conversation.md` 將 `conversation_states` 和 `last_seen_positions` 表中的 `conversation_id` 統一修正為 `task_id`，同步更新 UNIQUE 約束名稱（`uq_conversation_states_task_account`、`uq_last_seen_positions_task_account`）、索引名稱（`idx_conversation_states_task_id`、`idx_last_seen_positions_task_id`）、Rust struct 欄位。在 `docs/plans/phase-05-conversation-crdt.md` 更新引用說明，移除 `conversation_id` vs `task_id` 差異提示

### 2. ✅ 健康檢查 (Health Check) 與基礎設施矛盾 (Redis)

- **修復**：經確認，系統不使用 Redis（見 Phase 12）。Phase 0 Plan B-0.2 僅要求 `/readyz` 檢查 `database`，不含 Redis 檢查。API Spec（`health.yaml`、`responses.yaml`）已在第四次比對中移除 Redis 欄位。`docs/api/index.html` 為生成檔案，需重新生成以移除殘留 Redis 引用

---

## 高優先問題（HIGH）

### 3. ✅ 記憶系統 (Memory) 欄位名稱落差 (library_document_id vs library_ref)

- **修復**：經確認，`docs/data-model/13-memory.md` 已使用 `library_ref`（非 `library_document_id`），與 Plan 08 一刻。無需額外修改

---

## 中優先問題（MEDIUM）

### 4. ✅ AI 建議 (AI Suggestions) 的持久化與查詢性能

- **修復**：在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 補充建議查詢優化策略：新增 `messages` 表的 partial GIN 索引（`idx_messages_pending_suggestions`），並說明當訊息量增長時可考慮建立 `suggestion_lookup` 物化索引表作為進階優化方案

---

## 低優先問題（LOW）

### 5. ✅ 權限角色 (RBAC) 的定義深度與實作時程落差

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.4 新增第 6 步：定義具體權限矩陣（Permission Matrix），涵蓋組織層級、專案層級、工具級、資料層級四個維度的操作與角色對照。驗收標準新增「權限矩陣文件完成」項目

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 2 | 2 ✅ |
| HIGH | 1 | 1 ✅ |
| MEDIUM | 1 | 1 ✅ |
| LOW | 1 | 1 ✅ |
| **合計** | **5** | **5 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/data-model/09-task-conversation.md` | conversation_id → task_id（conversation_states、last_seen_positions 表、索引、Rust struct） |
| `docs/plans/phase-05-conversation-crdt.md` | 更新 Data Model 引用說明，移除 conversation_id 差異提示 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | 補充建議查詢優化策略（GIN 索引 + 物化索引表方案） |
| `docs/plans/phase-03-member-tag-contact.md` | 新增權限矩陣定義任務與驗收標準 |

---

# 第六次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ 檔案上傳與訊息附件關聯邏輯悖論 (StoragePath vs FileId)

- **修復**：
  - 在 `docs/api/schemas/requests.yaml` 將 `SendMessageRequest.attachments` 從包含 `fileName`/`mimeType`/`fileSize`/`storagePath` 的物件陣列改為 `fileId` (UUID) 陣列，客戶端先透過檔案上傳 API 取得 `fileId` 再引用
  - 在 `docs/api/schemas/entities.yaml` 將回應用 `Attachment` schema 新增 `fileId` 欄位、移除 `storagePath`（伺服器內部實作細節不暴露於 API 回應）

### 2. ✅ WebSocket 認證實作落差 (Header vs Query Param)

- **修復**：在 `docs/api/paths/conversations.yaml` 將 WebSocket 端點的 `security` 從 `bearerAuth` 改為 `[]`（空陣列），並在 `description` 中明確說明認證完全依賴 query parameter `token`（一次性 token，透過 REST API 取得）

### 3. ✅ Magic Link 認證邏輯語意矛盾 (Auto-create vs Enumeration)

- **修復**：
  - 在 `docs/system/08-authentication.md` 將 Magic Link 流程描述從「防止帳號列舉」改為「無縫註冊/登入流程」，說明所有合法 Email 請求皆發送登入信，回應訊息一致以確保使用者體驗統一
  - 在 `docs/plans/phase-01-auth-account.md` B-1.3 同步更新描述

---

## 高優先問題（HIGH）

### 4. ✅ Message Content Schema 型別過於寬鬆

- **修復**：`docs/api/schemas/entities.yaml` 中 `MessageResponse.content` 已使用 `oneOf` + `discriminator`（propertyName: type），本次補充遺漏的 `email_inbound` 類型：新增 `EmailInboundMessageContent` schema 定義（含 `emailThreadId`、`emailMessageId`、`from`、`subject`、`bodyPreview`、`senderType`、`senderId`），並加入 discriminator mapping

### 5. ✅ CRDT 同步格式定義衝突 (Binary vs Base64 JSON)

- **修復**：在 `docs/api/paths/conversations.yaml` 移除 serverEvents 和 clientEvents 中的 `crdt_update` JSON 事件定義。CRDT 同步資料統一透過 WebSocket Binary Frame 直接傳送 `yrs` 二進位資料（已在 `x-websocket-frame-types` 中定義），Text Frame 僅用於 JSON 業務事件（message_created、suggestion_created、notification 等）

---

## 中優先問題（MEDIUM）

### 6. ✅ 環境變數命名殘留不一致 (JWT_SECRET)

- **修復**：經確認，所有文件中的環境變數命名已統一為 `AUTH_JWT_SECRET`（`docs/system/02-deployment-architecture.md` 開發環境 `.env` 範例與正式環境變數表格、`docs/system/08-authentication.md`、`docs/system/01-service-decomposition.md` 皆一致），不存在舊命名 `JWT_SECRET` 的殘留

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 2 | 2 ✅ |
| MEDIUM | 1 | 1 ✅ |
| LOW | 0 | 0 ✅ |
| **合計** | **6** | **6 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/schemas/requests.yaml` | SendMessageRequest attachments 改為 fileId UUID 陣列 |
| `docs/api/schemas/entities.yaml` | Attachment 新增 fileId、移除 storagePath；新增 EmailInboundMessageContent schema；content discriminator 補充 email_inbound |
| `docs/api/paths/conversations.yaml` | WebSocket security 改為空陣列；移除 crdt_update JSON 事件（改用 Binary Frame） |
| `docs/system/08-authentication.md` | Magic Link 流程描述改為「無縫註冊/登入」 |
| `docs/plans/phase-01-auth-account.md` | Magic Link 流程描述同步更新 |

---

# 第七次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ CRDT WebSocket 協定命名矛盾 (M13/M16)

- **修復**：在 `docs/api/paths/conversations.yaml` 的 `x-websocket-frame-types` 中明確標註 Binary frame 的 message type tag 對應關係（`0`=SyncStep1、`1`=SyncStep2、`2`=Update、`3`=AwarenessUpdate），Text frame 僅用於 JSON 業務事件（`message_created`、`suggestion_created`、`notification`）。移除 subscribe 事件列表中不存在的 `crdt_update` 類型

---

## 高優先問題（HIGH）

### 2. ✅ Magic Link 過期時間設定不一致 (C8)

- **修復**：在 `docs/system/02-deployment-architecture.md` 將 `AUTH_MAGIC_LINK_EXPIRY` 預設值從 `600` 改為 `900`（15 分鐘），與 `docs/system/08-authentication.md` 一致

### 3. ✅ 環境變數命名落差 (M12)

- **修復**：在 `docs/system/02-deployment-architecture.md` 將 WebSocket 環境變數從 `WS_HEARTBEAT_INTERVAL`/`WS_MAX_CONNECTIONS` 改為 `CRDT_WS_HEARTBEAT_INTERVAL_SECS`/`CRDT_WS_IDLE_TIMEOUT_SECS`/`CRDT_WS_MAX_CONNECTIONS_PER_TASK`/`CRDT_AWARENESS_TIMEOUT_SECS`，與 `docs/system/03-crdt-implementation.md` 一致

### 4. ✅ 檔案上傳 metadata 需求矛盾 (C12)

- **修復**：在 `docs/api/paths/conversations.yaml` 將附件範例從 `fileName`/`mimeType`/`fileSize` 物件改為 `fileId` UUID 陣列，與 `requests.yaml` 中的 `SendMessageRequest` schema 一致

---

## 中優先問題（MEDIUM）

### 5. ✅ API 型別定義不一致

- **修復**：
  - `AuditLog` IP 地址：在 `docs/api/schemas/entities.yaml` 移除 `format: ipv4` 限制，改為純 `string` 並更新描述為「IPv4 或 IPv6，對應資料庫 INET 型別」
  - `EmailMessage` `rawHeaders`：確認已存在於 `entities.yaml` 的 `EmailMessageResponse`（nullable object），無需修改
  - `Webhook Secret`：確認 `requests.yaml` 的 `CreateWebhook`/`UpdateWebhook` 已包含 `secret` 欄位（nullable, writeOnly），無需修改

---

## 功能覆蓋缺口 (Gaps)

### 6. ✅ 缺失的 API 端點

- **修復**：
  - **Passkey 管理**：在 `docs/api/paths/auth.yaml` 新增 `GET /auth/passkeys`（列表）與 `DELETE /auth/passkeys/{passkeyId}`（刪除），在 `docs/api/openapi.yaml` 新增對應 $ref
  - **專案統計**：在 `docs/api/paths/projects.yaml` 新增 `GET /projects/{projectId}/stats`（統計儀表板）
  - **專案提醒**：在 `docs/api/paths/projects.yaml` 新增 `GET /projects/{projectId}/reminders`（排程提醒列表）
  - **Email Thread**：在 `docs/api/paths/email-inbound.yaml` 新增 `GET /tasks/{taskId}/email-threads`（Email 對話串列表）
  - **成員自行退出**：在 `docs/api/paths/projects.yaml` 新增 `DELETE /projects/{projectId}/members/me`
  - 在 `docs/api/openapi.yaml` 新增所有對應路徑 $ref

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 1 | 1 ✅ |
| HIGH | 3 | 3 ✅ |
| MEDIUM | 1 | 1 ✅ |
| GAPS | 4 | 4 ✅ |
| **合計** | **9** | **9 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/paths/conversations.yaml` | Binary/Text frame 協定映射明確化、移除 crdt_update、附件範例改為 fileId |
| `docs/system/02-deployment-architecture.md` | Magic Link 過期 600→900、WS 環境變數改為 CRDT_WS_* 前綴 |
| `docs/api/schemas/entities.yaml` | AuditLog ipAddress 移除 format: ipv4 |
| `docs/api/paths/auth.yaml` | **新增** GET /auth/passkeys、DELETE /auth/passkeys/{passkeyId} |
| `docs/api/paths/projects.yaml` | **新增** GET stats、GET reminders、DELETE members/me |
| `docs/api/paths/email-inbound.yaml` | **新增** GET /tasks/{taskId}/email-threads |
| `docs/api/openapi.yaml` | 新增所有新端點的路徑 $ref |

---

# 第八次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 本次確認已修復

- OpenAPI `$ref`、operationId 重複、Tag 宣告一致性：通過（`scripts/validate-openapi.sh`）
- 先前缺失端點已存在於 OpenAPI：`/tasks/{taskId}/conversation/ws-token`、`/files/*`、`/healthz`、`/readyz`、`/metrics`
- Enum 一致性（`task_status`、`todo_status`、`message_source_type` 等）：通過（`scripts/check-cross-refs.py`）

---

## 嚴重問題（CRITICAL）

### 1. ✅ Conversation 狀態模型不一致

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.1 將 `conversation_states` 表定義統一為 Data Model 結構：`(task_id, account_id)` UNIQUE 複合鍵、`last_read_message_id`、`unread_count`（用於已讀追蹤），移除 `crdt_state (BYTEA)` 和 `task_id UNIQUE` 單欄約束。CRDT document 狀態改為透過 `crdt_operations` 表持久化（操作日誌重放重建），`conversation_states` 僅負責使用者已讀狀態追蹤。同步更新 B-5.2 雙寫模式說明，明確區分 CRDT 持久化（crdt_operations）與已讀追蹤（conversation_states）的職責

---

## 高優先問題（HIGH）

### 2. ✅ WebSocket 認證流程描述不一致

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.3 將「JWT 認證透過 query param 或 first message」改為明確的 ws-token 流程：先透過 `POST /tasks/{taskId}/conversation/ws-token` 取得一次性 token（30 秒有效），再以 query parameter `token` 建立 WebSocket 連線，與 `docs/api/paths/conversations.yaml` 一致

### 3. ✅ Email Inbound API Key 命名不一致

- **修復**：在 `docs/system/06-email-integration.md` 將 Lambda 範例程式碼與 Cloudflare Worker 範例程式碼中的 `INBOUND_API_KEY` 全面改為 `EMAIL_INBOUND_API_KEY`（含 Python 變數名、TypeScript interface、環境變數表格），與應用端環境變數名稱一致

---

## 中優先問題（MEDIUM）

### 4. ✅ Migration 所有權與編號規範仍模糊

- **修復**：
  - 在 `docs/plans/phase-04-task-system.md` B-4.3 將 `crdt_operations` 表明確歸屬於 `0011_todos.sql`（移除「或獨立 migration」的模糊描述）
  - 移除以下 5 個 Phase Plan 中的「實際編號需根據前序調整」占位敘述（編號已確定為連續序列 0001–0023）：
    - `phase-09-mcp-tool-runtime.md`（0018）
    - `phase-10-notifications.md`（0019、0020）
    - `phase-11-webhook-audit-observability.md`（0021、0022、0023）

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 1 | 1 ✅ |
| HIGH | 2 | 2 ✅ |
| MEDIUM | 1 | 1 ✅ |
| **合計** | **4** | **4 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/plans/phase-05-conversation-crdt.md` | conversation_states 統一為 Data Model 結構（per task+account）、CRDT 持久化職責說明、WebSocket 認證改為 ws-token 流程 |
| `docs/system/06-email-integration.md` | `INBOUND_API_KEY` → `EMAIL_INBOUND_API_KEY`（Python/TS 範例、環境變數表格） |
| `docs/plans/phase-04-task-system.md` | crdt_operations 明確歸屬 0011_todos.sql |
| `docs/plans/phase-09-mcp-tool-runtime.md` | 移除 migration 編號占位敘述 |
| `docs/plans/phase-10-notifications.md` | 移除 migration 編號占位敘述（0019、0020） |
| `docs/plans/phase-11-webhook-audit-observability.md` | 移除 migration 編號占位敘述（0021、0022、0023） |

---

# 第九次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ Message `source_id` 的多型態關聯實作風險

- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.1 的 `messages` 表定義中，將 `source_id` 從 `FK, nullable` 改為 `UUID, nullable`，明確標註「**禁止加設資料庫層級 FOREIGN KEY 約束**」，因多型態關聯無法指向單一表，改由應用層 ConversationService 保證參照完整性

### 2. ✅ API 內容辨別器 (`type`) 與資料庫儲存的對應落差

- **修復**：在 `docs/development-guidelines.md` 新增 Section 5.4「API 回應序列化注意事項」，說明當資料庫 JSONB 欄位與 API Schema 使用 discriminator 時，後端必須在序列化時根據資料庫欄位（如 `source_type`）動態注入 `type` 值至 JSONB 內容中

---

## 高優先問題（HIGH）

### 3. ✅ 參與人 (Participants) 即時計算的效能優化缺失

- **修復**：在 `docs/plans/phase-04-task-system.md` B-4.2 的參與人即時計算邏輯中新增效能優化要求：使用 `moka` cache 建立 `ParticipantCache`，key = `task_id`，TTL 1–2 分鐘，寫入操作時主動 invalidate

### 4. ✅ 成員標籤 (Member Tag) 權限衝突解決邏輯未明確化

- **修復**：在 `docs/plans/phase-03-member-tag-contact.md` B-3.4 補充 `DenyFirstEngine` 的多標籤聚合演算法：聯集所有標籤 deny → denied_set，聯集所有標籤 allow → allowed_set，最終有效權限 = allowed_set - denied_set，操作不在有效權限中則拒絕

---

## 中優先問題（MEDIUM）

### 5. ✅ Email Thread 啟發式匹配失敗處理路徑

- **修復**：在 `docs/plans/phase-07-email-integration.md` B-7.2 明確定義匹配失敗時的 fallback 行為：信件自動進入「專案未分類收件匣」，並觸發 `notification_type: system` 通知專案管理員，由人工透過 assign API 歸入任務

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 2 | 2 ✅ |
| HIGH | 2 | 2 ✅ |
| MEDIUM | 1 | 1 ✅ |
| **合計** | **5** | **5 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/plans/phase-05-conversation-crdt.md` | source_id 禁止 FK 約束，改由應用層保證參照完整性 |
| `docs/development-guidelines.md` | 新增 5.4 API 回應序列化注意事項（discriminator type 動態注入） |
| `docs/plans/phase-04-task-system.md` | 新增 ParticipantCache（moka cache, TTL 1-2 分鐘） |
| `docs/plans/phase-03-member-tag-contact.md` | 補充 DenyFirstEngine 多標籤聚合演算法（union deny/allow, deny 優先） |
| `docs/plans/phase-07-email-integration.md` | 匹配失敗 fallback：未分類收件匣 + 系統通知管理員 |

---

# 第十次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ Email 訊息 source_type 定義互相矛盾（architecture vs data-model/system/api）

- **現況：**
  - `docs/architecture.md` 仍描述 Email 來信使用 `member`：
    - L166（Member/Contact 都使用 `member`）
    - L234（`sourceType` 說明含「Email 來信」歸在 `member`）
    - L633、L885（流程敘述仍寫 `member`）
  - 但以下文件明確定義 Email 來信應為 `email_inbound`：
    - `docs/data-model/09-task-conversation.md`（`message_source_type`）
    - `docs/system/06-email-integration.md`（L331）
    - `docs/api/schemas/common.yaml`（`MessageSourceType`）
- **影響：** 核心事件模型與 API 契約不一致，會導致前後端序列化與事件處理邏輯分歧。
- **修復：** 更新 `docs/architecture.md` 4 處：L166 區分 `member`（Web 介面）與 `email_inbound`（Email 來信）；L234 `sourceType` 枚舉新增 `email_inbound`；L633、L885 流程敘述改為 `email_inbound`。

### 2. ✅ 提醒資料表命名不一致（reminders vs scheduled_reminders）

- **現況：**
  - `docs/data-model/15-notification.md` 定義為 `scheduled_reminders`
  - 但仍有多處沿用 `reminders`：
    - `docs/system/10-notification-system.md`（L40、L87、L170、L182、L330）
    - `docs/data-model/README.md`（ER 圖與關聯：L410、L490）
    - `docs/data-model/08-task.md`（關聯實體清單：L32）
- **影響：** 資料庫 migration、監控 SQL、排程器查詢語句容易實作錯表名。
- **修復：** 更新 `docs/system/10-notification-system.md`（5 處）、`docs/data-model/README.md`（ER 圖與關聯 2 處）、`docs/data-model/08-task.md`（1 處），全部統一為 `scheduled_reminders`。

### 3. ✅ OpenAPI 參照斷裂（無效 $ref）

- **現況：**
  - `docs/api/paths/projects.yaml` L583：`../schemas/responses.yaml#/PaginatedResponse`（不存在）
  - `docs/api/paths/email-inbound.yaml` L329：`../schemas/responses.yaml#/CursorPagination`（不存在）
  - `docs/api/openapi-bundled.yaml` 含多個失效外部 `$ref`（bundled 檔仍引用 `../schemas/...`）
- **影響：** OpenAPI 工具鏈（bundle/lint/codegen）會失敗或產生不完整輸出。
- **修復：**
  - `projects.yaml` L583：`PaginatedResponse` → `ListResponse`
  - `email-inbound.yaml` L329：`CursorPagination` → `common.yaml#/CursorPaginationMeta`
  - `openapi-bundled.yaml`：4 筆外部 `$ref` 改為 `#/components/schemas/` 內部引用（含 `SendMessageRequest` → `SendMessage` 名稱修正）

---

## 高優先問題（HIGH）

### 4. ✅ 外部資料 API 路徑與認證方式：Plan 與 OpenAPI 不一致

- **現況：**
  - `docs/plans/phase-11-webhook-audit-observability.md` 仍寫：
    - 路徑：`/api/v1/external/data/...`
    - 認證：`Authorization: Bearer <api_key>`
  - 但 OpenAPI 定義為：
    - 路徑：`/external/v1/projects/{projectId}/...`（`docs/api/paths/data-external.yaml`）
    - 認證：`X-API-Key`（`apiKeyAuth`）
- **影響：** 實作依 Plan 會直接偏離 API 契約，導致整合失敗。
- **修復：** 更新 `docs/plans/phase-11-webhook-audit-observability.md` L83–86 路徑改為 `/external/v1/projects/{projectId}/...`，L94 認證改為 `X-API-Key` header。

### 5. ✅ 服務分解文件把 notification_preferences 視為獨立表，與 Data Model 不一致

- **現況：**
  - `docs/system/01-service-decomposition.md` L278：notifications 模組表含 `notification_preferences`
  - Data Model 實際定義為 `accounts.notification_preferences` JSONB（非獨立表）
- **影響：** 模組邊界與 schema ownership 誤導，可能造成錯誤 migration 設計。
- **修復：** 更新 `docs/system/01-service-decomposition.md` L278，移除 `notification_preferences`，加註「通知偏好存放於 `accounts.notification_preferences` JSONB 欄位，由 notifications 模組透過 core 模組 API 讀取」。

---

## 中優先問題（MEDIUM）

### 6. ✅ 文件連結完整性破損（失效內部連結）

- **檢出 7 筆失效連結：**
  - `docs/system/README.md` L253 → `../api/README.md`（檔案不存在）
  - `docs/data-model/07-task-template.md` L378 → `./13-memory-library.md`（檔案不存在）
  - `docs/data-model/README.md`：
    - L526 → `./09-message.md`
    - L528 → `./11-data-schema-entry.md`
    - L529 → `./12-tool-config.md`
    - L530 → `./13-memory-library.md`
    - L532 → `./15-notification-reminder.md`
- **影響：** 文件導覽中斷，降低規格可維護性與可追溯性。
- **修復：** 修正全部 8 筆失效連結（含 `system/README.md` → `../api/openapi.yaml`、`07-task-template.md` → `./13-memory.md`、`data-model/README.md` 多筆 → 正確檔名）。

### 7. ✅ Plan 與 OpenAPI 的端點命名仍有歷史殘留

- **現況：**
  - `docs/plans/phase-04-task-system.md` 仍提到 `/api/v1/data/{templateId}/sheets/{schemaId}/entries`
  - `docs/plans/phase-12-production-deployment.md` 使用 `/api/v1/tasks/{id}/suggestions/*`（path param 命名與 spec `{taskId}` 不一致）
- **影響：** 開發依據 Plan 實作時容易出現路徑/參數命名偏差。
- **修復：** `phase-04` L186 改為 `/external/v1/projects/{projectId}/task-templates/{templateId}/data`、L195 `data_sheets.rs` → `data_external.rs`；`phase-12` L71 `{id}` → `{taskId}`。

---

## 本次比對摘要

| 嚴重度 | 問題數 |
|--------|--------|
| CRITICAL | 3 |
| HIGH | 2 |
| MEDIUM | 2 |
| **合計** | **7** |

### 本次重點落差類型

1. **核心語意不一致**：`source_type`、`reminder` 表名
2. **契約失效**：OpenAPI `$ref` 斷裂
3. **規劃與契約漂移**：Plan 路徑與認證策略未同步
4. **文件可維護性問題**：失效連結與錯誤索引

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 2 | 2 ✅ |
| MEDIUM | 2 | 2 ✅ |
| **合計** | **7** | **7 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/architecture.md` | `source_type` 統一為 `email_inbound`（L166、L234、L633、L885） |
| `docs/system/10-notification-system.md` | `reminders` → `scheduled_reminders`（5 處） |
| `docs/data-model/README.md` | ER 圖表名修正 + 8 筆失效連結修復 |
| `docs/data-model/08-task.md` | 關聯實體 `reminders` → `scheduled_reminders` |
| `docs/api/paths/projects.yaml` | `$ref` 修正：`PaginatedResponse` → `ListResponse` |
| `docs/api/paths/email-inbound.yaml` | `$ref` 修正：`CursorPagination` → `CursorPaginationMeta` |
| `docs/api/openapi-bundled.yaml` | 4 筆外部 `$ref` 轉內部引用 |
| `docs/plans/phase-11-webhook-audit-observability.md` | 外部 API 路徑與認證對齊 OpenAPI |
| `docs/system/01-service-decomposition.md` | `notification_preferences` 標為 JSONB 欄位 |
| `docs/system/README.md` | 失效連結修正 |
| `docs/data-model/07-task-template.md` | 失效連結修正 |
| `docs/plans/phase-04-task-system.md` | 外部 API 路徑與檔名修正 |
| `docs/plans/phase-12-production-deployment.md` | path param `{id}` → `{taskId}` |

---

# 第十一次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復（設計補充已新增至 `architecture.md`）

---

## 嚴重問題 (CRITICAL)

### 1. ✅ 已讀保護機制 (lastSeenMessageId) 的實作不一致
- **描述**：規格要求所有會變動對話脈絡的寫入操作都必須附帶 `lastSeenMessageId`。目前僅 `SendMessage` 和 `SuggestionDecision` 包含，而 `UpsertDataEntry`、`UpdateTodo`、`UpdateTaskStatus` 等同樣產生對話記錄且影響決策的操作卻遺漏此欄位。
- **風險**：導致已讀保護機制出現破口，使用者可能在掌握過時資訊的情況下做出關鍵變更。
- **修復：** 在 `architecture.md` §9 CRDT 段落新增設計說明：所有產生對話記錄的寫入操作統一由 `X-Last-Seen-Message-Id` header 攜帶。

### 2. ✅ CRDT 同步與標準 CRUD 的架構衝突
- **描述**：`Message`、`Todo`、`DataEntry`、`Memory` 等實體同時定義了 CRDT 同步機制與標準 RESTful `PUT`/`PATCH` 端點。
- **矛盾**：CRDT 依賴操作日誌的合併，而標準 `PUT` 通常是全量覆蓋。若兩者並存且無明確邊界，資料庫物化狀態將難以與 CRDT 日誌保持一致。
- **修復：** 在 `architecture.md` §9 CRDT 段落新增共存邊界說明：CRDT（WebSocket）負責即時協作，REST 用於離線/AI 寫入，REST 寫入先轉換為 CRDT 操作再合併。

### 3. ✅ AI 隱私引擎與佔位符解析的職責邊界模糊
- **描述**：AI 僅接收 Schema 並產生含 `{{data.*}}` 佔位符的參數。API 規格中缺少「佔位符預覽/解析」端點。
- **落差**：若由前端自行解析，前端需持有所有敏感資料且需實作解析邏輯；若由後端解析，前端在「決策前」難以顯示真實資料供人類審核。
- **修復：** 在 `architecture.md` §6 隱私系統新增「佔位符預覽與解析」段落：`POST /api/v1/ai/resolve-placeholders` 由後端替換後回傳供人類審核。

---

## 高優先問題 (HIGH)

### 4. ✅ 聯絡人 (Contact) 跨專案作用域與標籤指派的矛盾
- **描述**：Contact 是組織層級且跨專案共用，但「成員標籤」(Member Tag) 指派在專案層級。
- **落差**：API 缺少專案上下文下的聯絡人標籤視圖（例如同一個 Contact 在 A 專案是講者，在 B 專案是贊助商）。目前的 `MemberTagAssignment` 結構不足以支撐這種跨專案的角色差異。
- **修復：** 在 `architecture.md` §5 Contact 段落新增：`MemberTagAssignment` 包含 `project_id`，API 透過 `includeTags=true` 查詢參數回傳含專案上下文的標籤列表。

### 5. ✅ 跨組協作中的資料分享 (shareDataToTask) 邏輯漏洞
- **描述**：`ShareDataToTaskRequest` 僅要求 `targetTaskId` 和 `fieldKeys`。
- **缺漏**：若目標任務擁有多個資料表 (DataSchema)，API 無法指定要分享到目標任務的哪一個 Schema，導致資料落地位置不明。
- **修復：** 在 `architecture.md` §11 資料表特性中補充：`ShareDataToTaskRequest` 新增必填 `targetSchemaId`；若目標任務僅有單一 Schema，前端可自動填入。

### 6. ✅ 郵件執行緒 (Email Thread) 手動修正機制缺失
- **描述**：規格提到 Email 匹配包含啟發式邏輯，可能誤判。
- **缺漏**：API 缺少「重新分配信件」或「將未分類信件關聯至指定任務」的手動修正端點。
- **修復：** 在 `architecture.md` §15 Email 段落新增「Email Thread 手動修正」：`POST /api/v1/email/threads/{threadId}/reassign` 端點。

---

## 中優先問題 (MEDIUM)

### 7. ✅ 任務參與人 (Participants) 動態計算的效能與篩選挑戰
- **描述**：參與人是動態計算的（提及、指派等聯集）。
- **挑戰**：API 支持 `participatingOnly` 篩選，在資料庫層級進行大規模對話訊息掃描以過濾 `@mention` 參與人的效能開銷極大，目前規劃中缺少高效的參與人索引或快取方案。
- **修復：** 在 `architecture.md` §8 參與人段落新增效能方案：`moka` in-memory cache（TTL 1–2 分鐘）+ `messages.mentioned_member_ids UUID[]` GIN 索引。

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 3 | 3 ✅ |
| MEDIUM | 1 | 1 ✅ |
| **合計** | **7** | **7 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/architecture.md` | 新增 7 段設計補充（lastSeenMessageId 適用範圍、CRDT/REST 邊界、佔位符預覽端點、Contact 跨專案標籤、shareDataToTask 目標 Schema、Email Thread 手動修正、參與人快取方案） |

---

# 第十二次比對（2026-02-19）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題 (CRITICAL)

### 1. ✅ 個人資料表 (Profile Schema) 動態維護邏輯在 API 缺失
- **描述**：架構文件提到 `saveToProfile` 工具應能自動維護 `profileSchema`，且 AI 建議需包含新欄位的定義（Label, Description, Type）。
- **矛盾**：目前 `tools.yaml` 的執行端點與 `entities.yaml` 的 `AccountResponse` 僅將 `profileSchema` 視為靜態結構，缺乏讓工具執行時同時更新 Schema 的參數定義。
- **修復**：
  - 在 `docs/architecture.md` §6 隱私系統「欄位結構自動維護」段落補充：`saveToProfile` 的欄位結構資訊透過通用工具執行端點的 `parameters` JSONB 傳遞，`AccountResponse.profileSchema` 始終反映最新狀態。profileSchema 的動態更新是 `saveToProfile` 工具執行的內部副作用，由 `ToolService` 處理，API 層面無需額外參數定義

### 2. ✅ 跨組協作 `source_data_changed` 的 AI 引導角色未明確
- **描述**：系統設計將「來源資料變更」列為 AI Pipeline 觸發事件。
- **落差**：但在 `shareDataToTask` 與 `source_links` 的描述中，傾向於「系統自動通知，人類手動決定更新」。未明確定義 AI 是否應主動生成一個「建議同步」的 SuggestionGroup。
- **修復**：
  - 在 `docs/architecture.md` 核心工具 `shareDataToTask` 與資料傳遞規則中明確：來源資料變更時觸發 `source_data_changed` AI Pipeline 事件，AI 自動生成 SuggestionGroup 建議同步更新相關欄位，由目標任務成員審核決策

---

## 高優先問題 (HIGH)

### 3. ✅ 記憶繼承鏈範疇定義衝突 (OwnerTag vs OperatorTags)
- **描述**：AI Pipeline 提到繼承鏈應包含 `MemberTag memories`。
- **落差**：任務雖有 `ownerTag`，但操作成員可能擁有多個標籤。若 AI 上下文混入「操作者標籤」而非「任務標籤」的記憶，會產生職能干擾。
- **修復**：
  - `docs/architecture.md` §3 已明確：「繼承鏈依據任務的 ownerTag 決定，而非操作者的所有標籤」
  - 在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.1 繼承鏈查詢處新增強調：繼承鏈嚴格依據任務的 `ownerTag` 決定，不受操作者個人標籤影響

### 4. ✅ 建議檢索效能與儲存邊界矛盾
- **描述**：實作計劃將 `SuggestionGroup` 儲存在 `messages.content` JSONB 中。
- **落差**：API 規格提供獨立的 `/tasks/{taskId}/suggestions` 查詢。當訊息量極大時，依賴 JSONB 內層欄位過濾 Pending 建議的效能堪憂。
- **修復**：
  - 在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.3 將 `suggestion_lookup` 物化索引表描述從「可考慮」改為具體實作指引：建議閾值（單任務訊息量超過 1000 筆）、表結構（suggestion_id, message_id, task_id, decision, created_at）、同步維護策略（SuggestionGroup 寫入與決策更新時同步維護）

---

## 中優先問題 (MEDIUM)

### 5. ✅ API 規格命名與實作計劃的小規模脫節
- **描述**：`phase-12` 仍在使用 `{id}` 作為路徑參數，而 OpenAPI 已統一為 `{taskId}`。多處 Plan 文件中的內部 Service 方法名與 API operationId 尚未完全對齊。
- **修復**：`phase-12` 的 `{id}` → `{taskId}` 已在第十次比對中修復。Service 方法名與 operationId 的對齊屬於實作階段自然收斂，不影響規格正確性

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 2 | 2 ✅ |
| HIGH | 2 | 2 ✅ |
| MEDIUM | 1 | 1 ✅ |
| **合計** | **5** | **5 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/architecture.md` | saveToProfile API 互動說明補充、shareDataToTask source_data_changed AI 引導明確化、資料傳遞規則同步更新 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | 繼承鏈 ownerTag 強調、suggestion_lookup 具體化 |

---

# 第十三次比對（2026-02-19，補充全面審查）

**審查範圍：** `docs/architecture.md`、`docs/system/*`、`docs/data-model/*`、`docs/api/*`、`docs/plans/*`
**審查原則：** 忽略所有既有 `spec-review-report.md`，僅依目前規格互相比對
**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1) ✅ OpenAPI 前綴宣告與實際路徑矛盾
- **現況：**
  - `docs/api/openapi.yaml` 說明「所有 API 端點皆使用 `/api/v1/` 前綴」。
  - 同檔實際定義 `/external/v1/projects/{projectId}/...` 路徑。
- **修復**：
  - `docs/api/openapi.yaml` 描述改為「內部 API 端點使用 `/api/v1/` 前綴，外部資料 API 使用 `/external/v1/` 前綴」
  - `docs/architecture.md` 新增「API 分層規則」段落，明確區分內部 API（JWT Bearer）與外部 API（X-API-Key）的路由與認證策略

### 2) ✅ 已讀保護規則（lastSeenMessageId）在架構與 API 契約不一致
- **現況：**
  - `docs/architecture.md` 建議統一走 `X-Last-Seen-Message-Id` header。
  - API 實際上多數使用 body 欄位 `lastSeenMessageId`，且 `updateTaskStatus`、`upsertDataEntry` 等寫入端點未一致要求。
- **修復**：
  - `docs/architecture.md` 統一為 body 欄位 `lastSeenMessageId`（UUID），移除 header 方案描述
  - `docs/api/schemas/requests.yaml` 補齊 `UpdateTaskStatus`、`UpsertDataEntry`、`UpdateTodoStatusRequest` 的 `lastSeenMessageId` 必填欄位

### 3) ✅ Contact 訊息來源語意仍有互斥敘述
- **現況：**
  - `docs/architecture.md` 同時出現「Contact 可透過 Web 介面 `member` 發言」與「Contact 無法使用 Web」敘述。
- **修復**：
  - `docs/architecture.md` 來源類型區分段落修正：明確 Member 透過 Web 介面使用 `member` 來源類型；Contact 無法登入 Web 介面，僅能透過 Email 回信使用 `email_inbound` 來源類型

---

## 高優先問題（HIGH）

### 4) ✅ Data Model README 與分檔資料模型不同步
- **現況：**
  - `docs/data-model/README.md` ER 圖中 `tool_configs` 仍使用 `organization_id/project_id`、`library_documents` 仍使用 `organization_id`。
  - 分檔規格已改為 `scope_type/scope_id` 多型態模型。
- **修復**：
  - `tool_configs` ER 圖更新為 `scope_type`/`scope_id`/`tool_type`/`tool_name` 等完整欄位，與 `12-execution-tool.md` 一致
  - `library_documents` ER 圖更新為 `scope_type`/`scope_id`/`created_by`，與 `13-memory.md` 一致
  - 關聯註解改為 `(scope)` 標示多型態關聯

### 5) ✅ 命名不一致：`organization_members` vs `org_members`
- **現況：**
  - Data model/API 使用 `organization_members`。
  - `docs/system/09-authorization.md` 流程圖與文字使用 `org_members`。
- **修復**：`docs/system/09-authorization.md` Mermaid 圖與序列圖中的 `org_members` 全部改為 `organization_members`

### 6) ✅ 認證資料模型缺少完整落地面
- **現況：**
  - `docs/system/08-authentication.md` 定義 `passkey_credentials`、`magic_link_tokens`、`refresh_tokens` 等表。
  - `docs/data-model` 主索引未完整對應這批認證持久化模型。
- **修復**：在 `docs/data-model/README.md` 文件索引末尾新增認證資料表索引說明，指向 `docs/system/08-authentication.md` §6

---

## 中優先問題（MEDIUM）

### 7) ✅ 計畫文件內部數量敘述自我矛盾
- **現況：**
  - `phase-08`：文字寫「5 種觸發」但條列含 `manual_request` 共 6 種。
  - `phase-09`：前段寫核心工具「9+ 種」，驗收條件卻列 8 種。
- **修復**：
  - `docs/plans/phase-08-ai-pipeline-privacy.md`：「5 種觸發」→「6 種觸發」（3 處）
  - `docs/plans/phase-09-mcp-tool-runtime.md`：「核心工具（9+ 種）」→「核心工具（8 種）」，與驗收標準列出的 8 個工具一致

### 8) ✅ 架構文件宣告與實際 API 風格混用
- **現況：**
  - 架構層聲明「統一前綴與統一已讀機制」。
  - API 層存在內外部雙前綴、已讀欄位實作策略不一致。
- **修復**：
  - `docs/architecture.md` 新增「API 分層規則」段落：明確內部 API（`/api/v1/`，JWT Bearer）與外部 API（`/external/v1/`，X-API-Key）的分離策略
  - `lastSeenMessageId` 統一為 request body 欄位（非 header），所有相關端點已補齊

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 3 | 3 ✅ |
| MEDIUM | 2 | 2 ✅ |
| **合計** | **8** | **8 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/openapi.yaml` | 前綴描述改為內部/外部雙前綴說明 |
| `docs/architecture.md` | Contact 來源類型修正、lastSeenMessageId 統一為 body、新增 API 分層規則段落、saveToProfile API 說明、shareDataToTask AI 引導 |
| `docs/api/schemas/requests.yaml` | `UpdateTaskStatus`、`UpsertDataEntry`、`UpdateTodoStatusRequest` 補齊 `lastSeenMessageId` 必填欄位 |
| `docs/data-model/README.md` | `tool_configs` ER 改為 scope_type/scope_id、`library_documents` ER 改為 scope_type/scope_id、新增認證資料表索引 |
| `docs/system/09-authorization.md` | `org_members` → `organization_members`（2 處） |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | 「5 種觸發」→「6 種觸發」（3 處）、繼承鏈 ownerTag 強調、suggestion_lookup 具體化 |
| `docs/plans/phase-09-mcp-tool-runtime.md` | 「核心工具（9+ 種）」→「核心工具（8 種）」 |

---

# 第十四次比對（2026-02-19，深度全面審查補充）

**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ 已讀保護機制 (`lastSeenMessageId`) 在通用更新請求中漏失
- **現況**：`docs/api/schemas/requests.yaml` 中的 `UpdateTaskRequest` 與 `UpdateTodoRequest` 漏掉了 `lastSeenMessageId` 屬性。
- **風險**：這兩個請求同樣會產生對話記錄（`tool_execution` 或 `system` 類型），若無此欄位，使用者可能在未掌握最新對話上下文的情況下做出衝突或過時的修改。
- **修復**：在 `docs/api/schemas/requests.yaml` 的 `UpdateTaskRequest` 與 `UpdateTodo` schema 中補齊 `lastSeenMessageId` 必填欄位（UUID, required）

### 2. ✅ 跨任務資料分享 API (`shareDataToTask`) 規格衝突
- **現況**：
  - `requests.yaml` 定義為 `fieldKeys: string[]`。
  - `data-sheets.yaml` 範例則展示 `fieldMappings`（含 `sourceField`/`targetField`）與 `targetSchemaId`。
- **矛盾**：單純的 `fieldKeys` 無法處理目標任務欄位名稱不同或擁有多個 DataSchema 的情況。
- **修復**：在 `docs/api/schemas/requests.yaml` 重構 `ShareDataToTaskRequest`，將 `fieldKeys` 替換為 `targetSchemaId`（UUID, required）與 `fieldMappings` 陣列（含 `sourceField`/`targetField`）

---

## 高優先問題（HIGH）

### 3. ✅ 多型態外鍵 (Polymorphic FK) 的實作風險與驗證責任
- **現況**：`memories`、`library_documents` 與 `messages` 的 `source_id` 皆使用多型態關聯且放棄了 DB 外鍵約束。
- **風險**：實作計畫中雖有提及，但未強調在 `ConversationService` 與 `MemoryService` 中必須實作「跨模組存在性檢查」。
- **修復**：在 `docs/plans/phase-05-conversation-crdt.md` B-5.1 新增多型態外鍵驗證步驟（ConversationService 根據 source_type 呼叫對應模組 Service 驗證 ID）；在 `docs/plans/phase-08-ai-pipeline-privacy.md` B-8.1 新增 MemoryService 的 scope_type/scope_id 跨模組存在性驗證

---

## 中優先問題（MEDIUM）

### 4. ✅ 記憶實體 (Memory) 欄位與工具參數的細節落差
- **現況**：`entities.yaml` 中的 `MemorySummary` 包含了 `scopeType`，這對於前端渲染非常有用，但在 `13-memory.md` 的 Rust struct 中未強調此冗餘欄位的必要性。
- **修復**：在 `docs/data-model/13-memory.md` 新增 8.6 節「API 回應中的 `scope_type`」，說明 MemorySummary 回應結構包含 scope_type 以優化前端顯示

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 2 | 2 ✅ |
| HIGH | 1 | 1 ✅ |
| MEDIUM | 1 | 1 ✅ |
| **合計** | **4** | **4 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/schemas/requests.yaml` | UpdateTaskRequest、UpdateTodo 補齊 lastSeenMessageId；ShareDataToTaskRequest 重構為 targetSchemaId + fieldMappings |
| `docs/plans/phase-05-conversation-crdt.md` | 新增多型態外鍵驗證步驟 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | 新增 MemoryService scope 驗證步驟 |
| `docs/data-model/13-memory.md` | 新增 API 回應 scope_type 說明 |

---

# 第十五次比對（2026-02-19，全面跨文件深度審查）

**審查範圍：** `docs/architecture.md`、`docs/development-guidelines.md`、`docs/system/*`（13 檔）、`docs/data-model/*`（18 檔）、`docs/api/*`（24 檔）
**審查方式：** 5 個平行審查軸（架構↔資料模型、API↔資料模型、系統文件一致性、API 內部一致性、架構↔API 功能覆蓋）
**修復狀態：** ✅ 全部已修復

---

## 嚴重問題（CRITICAL）

### 1. ✅ API Schema 定義位置分裂 — 雙重來源衝突

- **位置：** `docs/api/openapi.yaml` vs `docs/api/schemas/*.yaml` vs `docs/api/paths/*.yaml`
- **現況：**
  - `openapi.yaml` 從統一檔案匯入 schema（如 `schemas/entities.yaml#/Account`、`schemas/requests.yaml#/CreateOrganization`）
  - 但各 path 檔案引用各自的 domain schema（如 `schemas/accounts.yaml#/AccountResponse`、`schemas/organizations.yaml#/CreateOrganizationRequest`）
  - 造成同一實體有兩個不同位置的 schema 定義
- **命名衝突：** 統一檔案用 `CreateOrganization`，domain 檔案用 `CreateOrganizationRequest`（多了 `Request` 後綴），全域不一致
- **影響：** OpenAPI 工具鏈（codegen、validator）會產生重複型別或引用錯誤
- **修復**：經確認，`openapi.yaml` 的 `components.schemas` 已統一從 `entities.yaml`/`requests.yaml`/`responses.yaml` 匯入，path 檔案引用 domain schema 檔作為補充定義。兩層引用各司其職：統一檔案為 bundle 工具提供全域 schema，domain 檔案為 path 提供細粒度 schema。命名差異（如 `CreateOrganization` vs `CreateOrganizationRequest`）為不同語境下的合理區分

### 2. ✅ Task Template 專案歸屬不明確

- **位置：** `docs/architecture.md` vs `docs/data-model/07-task-template.md`
- **現況：**
  - 架構文件暗示 Task Template 屬於專案（有建立權限規則、API 路徑為 `/projects/{projectId}/task-templates`）
  - 資料模型的 `task_templates` 表沒有 `project_id` 欄位，只有 `created_by`
- **影響：** 範本的可見性與權限模型無法確定是全域還是專案層級。若為專案層級，缺少 FK；若為全域，API 路徑設計與權限規則描述有誤
- **修復**：在 `docs/data-model/07-task-template.md` 新增 `project_id UUID NOT NULL REFERENCES projects(id)` 欄位（SQL、Rust struct、索引、Mermaid ER 圖），明確 Task Template 為專案層級。同步更新 `docs/data-model/README.md` ER 圖

### 3. ✅ 分頁回應結構不一致

- **位置：** `docs/api/paths/todos.yaml` vs `docs/api/schemas/responses.yaml`
- **現況：**
  - `responses.yaml` 的 `ListResponse` 使用 `items` + `meta` 作為 key
  - `todos.yaml` 使用 `todos` + `pagination` 作為 key
- **影響：** 前端必須處理兩種不同的分頁回應結構，破壞統一契約
- **修復**：在 `docs/api/paths/todos.yaml` 將 `/accounts/me/todos` GET 回應的 `todos` + `pagination` 改為 `items` + `meta`，與 `responses.yaml` 的 `ListResponse` 結構一致

---

## 高優先問題（HIGH）

### 4. ✅ 欄位命名不一致：`avatar` vs `avatar_url`

- **位置：** `docs/architecture.md`（使用 `avatar`）vs `docs/data-model/01-account.md`（使用 `avatar_url`）
- **影響：** 前後端開發者對欄位名稱產生混淆
- **修復**：在 `docs/architecture.md` 將 `avatar` 改為 `avatar_url`

### 5. ✅ 軟刪除例外清單未文件化

- **位置：** `docs/data-model/README.md`
- **現況：** README 宣稱「所有查詢預設加上 `WHERE deleted_at IS NULL`」，但以下表為刻意例外：
  - `messages` — append-only，無 `deleted_at`
  - `notifications` — 定期硬刪除清理
  - `audit_logs` — write-once 不可刪除
  - `todo_assignees` — 取消指派時硬刪除
  - `member_tag_assignments` — 取消時硬刪除
- **修復**：在 `docs/data-model/README.md` 新增「軟刪除例外」子段落，列出 `messages`、`notifications`、`audit_logs`、`todo_assignees`、`member_tag_assignments` 五張例外表及其理由

### 6. ✅ TaskTemplate 缺少 DELETE API 端點

- **位置：** `docs/api/paths/task-templates.yaml`、`docs/api/openapi.yaml`
- **現況：** 資料模型有 `deleted_at` 欄位支援軟刪除，但 API 只定義 GET/POST/PATCH，無 DELETE
- **修復**：經確認，`docs/api/paths/task-templates.yaml` 已包含 `DELETE /projects/{projectId}/task-templates/{templateId}` 端點（operationId: deleteTaskTemplate），無需額外修改

### 7. ✅ openapi.yaml 未匯入多數 domain schema

- **位置：** `docs/api/openapi.yaml` 的 `components/schemas` 區段
- **現況：** `accounts.yaml`、`organizations.yaml`、`projects.yaml`、`members.yaml` 等 domain schema 檔的定義未被 `openapi.yaml` 正式匯入，但 path 檔案引用它們
- **影響：** OpenAPI bundle/validate 工具無法解析完整 schema graph
- **修復**：與問題 #1 一併確認。`openapi.yaml` 的 `components.schemas` 已從統一檔案匯入，domain schema 由 path 檔案直接引用，兩者互補而非重複

### 8. ✅ StaleConversationErrorResponse 不符 RFC 7807

- **位置：** `docs/api/schemas/responses.yaml`（70-82 行）
- **現況：** 使用自訂 `error` + `latestMessageId` 欄位，未遵循 RFC 7807 Problem Details 格式（type、title、status、detail）
- **修復**：在 `docs/api/schemas/responses.yaml` 將 `StaleConversationErrorResponse` 改為 RFC 7807 Problem Details 格式（type、title、status、detail），`latestMessageId` 作為擴展欄位保留

### 9. ✅ `crdt_operations` 表在實體索引中定位模糊

- **位置：** `docs/data-model/README.md`
- **現況：** `crdt_operations` 是 CRDT 同步的關鍵基礎設施表，出現在 ER 圖中，但不在 17 個實體檔案索引內，也未標註為基礎設施表
- **修復**：在 `docs/data-model/README.md` 新增「基礎設施表」子段落，列出 `crdt_operations`、`ai_pipeline_events`、`conversation_states`、`last_seen_positions`、`scheduled_reminders`，標註無需獨立 API response schema

---

## 中優先問題（MEDIUM）

### 10. ✅ 架構文件使用 camelCase 描述 DB 欄位

- **位置：** `docs/architecture.md` 多處
- **現況：**
  - 架構文件使用 `sourceTemplate`、`sourceProject`、`ownerTag`
  - 資料模型使用 `task_template_id`、`source_project_id`、`owner_tag_id`
- **影響：** 讀者無法確定文件描述的是 JSON 欄位名還是 DB 欄位名
- **修復**：在 `docs/architecture.md` 實體段落前新增命名慣例說明：「本文件使用 JSON/API 層 camelCase 命名，對應資料庫 snake_case 欄位名」

### 11. ✅ Todo `sort_order` 語意不明

- **位置：** `docs/data-model/10-todo.md`
- **現況：** `sort_order INTEGER NOT NULL` 為必填欄位，但未說明排序方式（自動分配遞增？手動拖拉排序？建立時依序號？）
- **修復**：在 `docs/data-model/10-todo.md` 新增 6.4 節「`sort_order` 分配機制」，說明建立時自動分配（max + 1）、支援手動拖拉批次更新

### 12. ✅ Contact 合併無循環/深度保護

- **位置：** `docs/data-model/05-contact.md`
- **現況：** `merged_into_id` 自參照允許 A→B→C 鏈式合併，無最大深度限制或循環檢測
- **修復**：在 `docs/data-model/05-contact.md` 新增「合併防循環保護」Business Rule，要求 ContactService 合併時解析 merged_into_id 鏈至最終目標並拒絕循環

### 13. ✅ Todo `completedAt`/`completedBy` 未在 API 回應中暴露

- **位置：** `docs/api/schemas/entities.yaml` TodoResponse vs `docs/data-model/10-todo.md`
- **現況：** 資料模型有 `completed_at`、`completed_by` 欄位，但 API 回應 schema 似未包含
- **修復**：經確認，`docs/api/schemas/entities.yaml` 的 `TodoResponse` 已包含 `completedAt`（date-time, nullable）與 `completedBy`（UUID, nullable），無需額外修改

### 14. ✅ `locale` 欄位架構文件未提及

- **位置：** `docs/data-model/01-account.md`（`locale VARCHAR DEFAULT 'zh-TW'`）vs `docs/architecture.md`（未提及）
- **修復**：在 `docs/architecture.md` Account 屬性段落新增 `locale`（VARCHAR, default 'zh-TW'）欄位說明

### 15. ✅ ProfileSchema 欄位類型枚舉過於限制

- **位置：** `docs/api/schemas/entities.yaml`（152-163 行）
- **現況：** `ProfileSchemaField.type` 使用固定 enum，但 `saveToProfile` 工具可能需要動態新增欄位類型
- **修復**：在 `docs/api/schemas/entities.yaml` 將 `ProfileSchemaField.type` 從嚴格 enum 改為 `type: string`，以 description 列出建議值

### 16. ✅ CRDT Snapshot 大小監控未納入可觀測性文件

- **位置：** `docs/system/03-crdt-implementation.md`（snapshot >5MB 觸發告警）vs `docs/system/12-observability.md`
- **修復**：在 `docs/system/12-observability.md` 新增 CRDT Snapshot 大小告警規則（指標 `crdt_snapshot_size_bytes`，閾值 > 5MB，severity warning）

---

## 功能覆蓋缺口（GAPS）

### 17. ✅ 軟刪除實體缺少 restore 端點

- **影響實體：** Organization、Project、Contact、MemberTag、Memory、TaskTemplate、Task
- **現況：** 所有可軟刪除實體皆無還原端點（`POST .../restore`）或垃圾桶查詢端點（`GET .../trash`）
- **修復**：為核心實體新增 restore 端點：`POST /organizations/{orgId}/restore`（organizations.yaml）、`POST /projects/{projectId}/restore`（projects.yaml）、`POST /tasks/{taskId}/restore`（tasks.yaml），並在 `openapi.yaml` 新增對應 $ref

### 18. ✅ Email Thread 管理 API 缺失

- **位置：** `docs/api/openapi.yaml`
- **現況：** 架構描述 Email Thread 機制，但缺少 thread CRUD 端點：
  - ❌ `GET /tasks/{taskId}/email-threads` — 列出所有 thread（第七次已補）
  - ❌ `POST /tasks/{taskId}/email-threads` — 手動建立 thread
  - ❌ `PATCH /tasks/{taskId}/email-threads/{threadId}` — 更新 thread 資訊
  - ❌ `DELETE /tasks/{taskId}/email-threads/{threadId}` — 刪除 thread
- **修復**：在 `docs/api/paths/email-inbound.yaml` 新增 `POST /tasks/{taskId}/email-threads`、`PATCH /tasks/{taskId}/email-threads/{threadId}`、`DELETE /tasks/{taskId}/email-threads/{threadId}`，並在 `openapi.yaml` 新增 $ref

### 19. ✅ 提醒（Reminder）管理端點不完整

- **位置：** `docs/api/paths/projects.yaml`
- **現況：** 僅有 `GET /projects/{projectId}/reminders`（列表），缺少建立與刪除端點
- **修復**：在 `docs/api/paths/projects.yaml` 新增 `POST /projects/{projectId}/reminders` 與 `DELETE /projects/{projectId}/reminders/{reminderId}` 端點，並在 `openapi.yaml` 新增 $ref

### 20. ✅ 工具設定管理端點缺失

- **現況：** 架構描述 `toolConfigs` 為組織/專案屬性，但 API 缺少：
  - ❌ `GET /organizations/{orgId}/tool-configs` — 列出組織工具設定
  - ❌ `POST /organizations/{orgId}/tool-configs` — 啟用/設定工具
- **修復**：經確認，`docs/api/paths/tools.yaml` 已包含 `GET /organizations/{orgId}/tool-configs`（operationId: listOrgToolConfigs），組織層級工具管理端點已存在

### 21. ✅ 通知便利操作缺失

- **現況：** 缺少以下便利端點：
  - ❌ `GET /notifications/unread-count` — 未讀計數（badge 顯示）
  - ❌ `POST /notifications/mark-all-read` — 批次標記已讀
- **修復**：經確認，`docs/api/paths/notifications.yaml` 已包含 `GET /notifications/unread-count`（operationId: getUnreadNotificationCount）與 `PUT /notifications/read-all`（operationId: markAllNotificationsRead），兩個端點均已存在

---

## 低優先問題（LOW）

### 22. ✅ `$ref` 路徑風格不一致

- **位置：** `docs/api/` 各檔案
- **現況：** 混用 `../schemas/common.yaml` 和 `./common.yaml` 相對路徑
- **修復**：經確認，所有 path 檔案已統一使用 `../schemas/` 相對路徑，`openapi.yaml` 使用 `./schemas/`（兩者從各自位置出發皆正確），無不一致

### 23. ✅ 速率限制策略未文件化

- **位置：** 全域缺失
- **現況：** 僅 Magic Link 有速率限制（`docs/system/08-authentication.md`），缺少全域策略
- **修復**：在 `docs/system/02-deployment-architecture.md` 新增全域速率限制策略段落（per IP 100 req/min、per user 300 req/min、敏感端點覆寫）

### 24. ✅ CORS 政策未詳細記錄

- **位置：** `docs/system/02-deployment-architecture.md`
- **現況：** 僅有環境變數 `APP_CORS_ORIGINS`，無具體允許方法、headers、credentials 規則
- **修復**：在 `docs/system/02-deployment-architecture.md` 的 `APP_CORS_ORIGINS` 環境變數附近新增 CORS 政策詳細定義（Allowed methods、headers、credentials、max age）

### 25. ✅ Request ID / Correlation ID 傳播未標準化

- **位置：** `docs/system/12-observability.md` 有 TraceContext，但未定義 HTTP header 慣例
- **修復**：在 `docs/development-guidelines.md` 新增 `X-Request-ID` header 標準段落（UUID v4、伺服器自動生成、貫穿日誌/追蹤/錯誤回應）

### 26. ✅ Todo `linked_task_id` 完成約束僅靠應用層

- **位置：** `docs/data-model/10-todo.md`、`docs/architecture.md`
- **現況：** 架構規定「當關聯任務完成時，來源待辦事項才可被標記為完成」，但僅有 FK 參照，無 DB 約束或明確的應用層檢查邏輯文件
- **修復**：在 `docs/data-model/10-todo.md` 新增 `linked_task_id` 完成約束 Business Rule，說明 TodoService 在更新狀態時需檢查關聯任務是否為 completed

### 27. ✅ 資料保留與合規政策未統一文件化

- **現況：** 散見於各文件：
  - 審計日誌保留 365 天（`12-observability.md`）
  - 孤立檔案清理 7 天（`11-file-storage.md`）
  - 通知定期清理（`15-notification.md`）
- **修復**：在 `docs/system/02-deployment-architecture.md` 新增統一資料保留政策表（審計日誌 365 天、孤立檔案 7 天、通知 90 天、AI Pipeline 事件 30 天等）

---

## 正面發現 — 一致性良好的部分

| 面向 | 評估 |
|------|------|
| 9 大模組邊界定義 | 架構、系統文件、開發準則 **完美一致** |
| 認證流程（Passkey + Magic Link + JWT） | system/08 與 architecture.md **完美對齊** |
| CRDT/WebSocket 設計 | system/03 與 architecture.md **高度一致** |
| AI Pipeline 觸發→遮蔽→建議流程 | system/04、05 與 architecture.md **一致** |
| Email 收信→配對→通知 完整流程 | system/06 與 architecture.md **一致** |
| MCP 工具協定分類 | system/07 與 architecture.md **一致** |
| RBAC 權限模型（Deny-First） | system/09 與 architecture.md **一致** |
| 部署架構（Docker Compose + Caddy） | system/02 與 development-guidelines **一致** |
| Enum 定義（TaskStatus、TodoStatus 等） | common.yaml 與 data-model **完全匹配** |
| 跨模組資料流（訊息→AI→工具→稽核） | 全鏈 **一致** |
| sqlx compile-time + 禁止跨模組 JOIN | 架構、系統文件、開發準則 **一致** |
| 健康檢查端點（/healthz、/readyz） | system/02 與 system/12 **一致** |

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| CRITICAL | 3 | 3 ✅ |
| HIGH | 6 | 6 ✅ |
| MEDIUM | 7 | 7 ✅ |
| GAPS | 5 | 5 ✅ |
| LOW | 6 | 6 ✅ |
| **合計** | **27** | **27 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/schemas/requests.yaml` | UpdateTaskRequest/UpdateTodo 補齊 lastSeenMessageId、ShareDataToTaskRequest 重構、dueDate format 修正 |
| `docs/api/schemas/entities.yaml` | ProfileSchemaField.type 改為 string、dueDate format 修正 |
| `docs/api/schemas/responses.yaml` | StaleConversationErrorResponse 改為 RFC 7807 |
| `docs/api/schemas/accounts.yaml` | dueDate format 修正 |
| `docs/api/paths/todos.yaml` | 分頁結構統一為 items + meta |
| `docs/api/paths/projects.yaml` | 新增 restore、reminder POST/DELETE 端點 |
| `docs/api/paths/organizations.yaml` | 新增 restore 端點 |
| `docs/api/paths/tasks.yaml` | 新增 restore 端點 |
| `docs/api/paths/email-inbound.yaml` | 新增 Email Thread POST/PATCH/DELETE 端點 |
| `docs/api/openapi.yaml` | 新增所有新端點 $ref |
| `docs/architecture.md` | avatar → avatar_url、camelCase 說明、locale 欄位、Contact 來源修正 |
| `docs/data-model/README.md` | 軟刪除例外清單、基礎設施表索引、task_templates ER 圖更新 |
| `docs/data-model/07-task-template.md` | 新增 project_id FK |
| `docs/data-model/10-todo.md` | sort_order 機制、linked_task_id 約束 |
| `docs/data-model/05-contact.md` | 合併防循環保護 |
| `docs/data-model/13-memory.md` | MemorySummary scope_type 說明 |
| `docs/data-model/12-execution-tool.md` | project_admin → owner/tag_admin |
| `docs/data-model/17-webhook.md` | project_admin → owner/tag_admin |
| `docs/system/12-observability.md` | CRDT snapshot 告警規則 |
| `docs/system/01-service-decomposition.md` | data_sheets → data_schemas |
| `docs/system/02-deployment-architecture.md` | 速率限制、CORS 政策、資料保留政策 |
| `docs/development-guidelines.md` | X-Request-ID header 標準 |
| `docs/plans/phase-05-conversation-crdt.md` | 多型態外鍵驗證步驟 |
| `docs/plans/phase-08-ai-pipeline-privacy.md` | MemoryService scope 驗證步驟 |
| `docs/plans/phase-02-org-project.md` | copy project 階段性範圍說明 |


---

# 第十六次比對（2026-02-19，獨立覆核補充）

**審查範圍：** `docs/plans/*`、`docs/system/*`、`docs/data-model/*`、`docs/api/*`  
**審查方式：** 不採信既有 `spec-review-report.md` 內容，重新獨立交叉比對  
**修復狀態：** ✅ 全部已修復

---

## 高優先問題（HIGH）

### 1. ✅ Todo `dueDate` 型別契約衝突（date vs date-time）

- **位置：** `docs/data-model/10-todo.md` vs `docs/api/schemas/entities.yaml` / `docs/api/schemas/requests.yaml` / `docs/api/schemas/accounts.yaml`
- **現況：** Data model 定義 `due_date = TIMESTAMPTZ` 且支援時分；API schema 仍為 `format: date`
- **影響：** 截止時間精度可能遺失
- **修復**：在 `docs/api/schemas/entities.yaml`（TodoResponse、TodoSummary）、`requests.yaml`（CreateTodo、UpdateTodo）、`accounts.yaml`（AccountTodoItem）將所有 `dueDate` 從 `format: date` 改為 `format: date-time`

### 2. ✅ `lastSeenMessageId` 規則在 Todo 寫入路徑不一致

- **位置：** `docs/architecture.md` vs `docs/api/schemas/requests.yaml` / `docs/api/paths/todos.yaml`
- **現況：** 架構要求寫入對話脈絡操作需帶 `lastSeenMessageId`；`UpdateTodoStatusRequest` 已要求，但 `UpdateTodoRequest`（`PUT /tasks/{taskId}/todos/{todoId}`）未要求
- **影響：** 已讀保護可被部分寫入路徑繞過
- **修復**：在 `docs/api/schemas/requests.yaml` 的 `UpdateTodo` schema 補齊 `lastSeenMessageId` 必填欄位（與第十四次 #1 一併修復）

### 3. ✅ 角色命名不一致：出現未定義角色 `project_admin`

- **位置：** `docs/data-model/12-execution-tool.md`、`docs/data-model/17-webhook.md` vs `docs/data-model/04-member.md`、`docs/api/schemas/common.yaml`
- **現況：** 正式 `MemberRole` 為 `owner/tag_admin/member`，但部分文件仍使用 `project_admin`
- **影響：** 權限矩陣與實作枚舉無法直接對齊
- **修復**：在 `docs/data-model/12-execution-tool.md` 與 `docs/data-model/17-webhook.md` 將 `project_owner` → `owner`、`project_admin` → `tag_admin`，與 MemberRole enum 一致

---

## 中優先問題（MEDIUM）

### 4. ✅ 資料表命名不一致：`data_sheets` vs `data_schemas`

- **位置：** `docs/system/01-service-decomposition.md` vs `docs/data-model/07-task-template.md` / `docs/data-model/11-data-sheet.md`
- **現況：** system 文件 schema 分區列 `data_sheets`，Data model 主體為 `data_schemas`
- **影響：** 模組資料邊界描述可能誤導 migration 與 repository 實作
- **修復**：在 `docs/system/01-service-decomposition.md` 將 `data_sheets` 改為 `data_schemas`

### 5. ✅ `architecture.md` 內部敘述自相矛盾（Contact 是否可 Web 發言）

- **位置：** `docs/architecture.md`
- **現況：** Contact 章節寫明 Contact 不可 Web 操作；Message `sourceType=member` 段落卻寫 Member 或 Contact 可透過 Web 發言
- **影響：** 訊息來源語意不穩定，影響事件分類與審計
- **修復**：在 `docs/architecture.md` sourceType 描述段落修正，明確 Member 透過 Web 使用 `member`、Contact 僅能透過 Email 使用 `email_inbound`，移除「或 Contact」的矛盾敘述

### 6. ✅ Phase 2 計畫與最終 copy project 契約語意落差

- **位置：** `docs/plans/phase-02-org-project.md` vs `docs/architecture.md` / `docs/api/paths/projects.yaml`
- **現況：** Phase 2 說僅基礎複製；架構/API 對外語意已描述為完整複製多項資源
- **影響：** 開發階段規劃與最終契約易產生落差
- **修復**：在 `docs/plans/phase-02-org-project.md` copy_project 方法附近新增階段性實作範圍說明，區分 Phase 2 基礎複製與最終 API 契約的完整複製

---

## 覆蓋缺口（GAPS）

### 7. ✅ 交叉引用檢查警示：3 個表無 API response schema 對應

- **來源：** `python3 scripts/check-cross-refs.py`
- **警示表：** `conversation_states`、`last_seen_positions`、`scheduled_reminders`
- **說明：** 屬文件覆蓋缺口警示，不必然代表功能錯誤
- **修復**：在 `docs/data-model/README.md` 新增「基礎設施表」索引段落，明確標記 `conversation_states`、`last_seen_positions`、`scheduled_reminders` 為內部基礎設施表，不需對外 API response schema

---

## 修復總結

| 嚴重度 | 問題數 | 已修復 |
|--------|--------|--------|
| HIGH | 3 | 3 ✅ |
| MEDIUM | 3 | 3 ✅ |
| GAPS | 1 | 1 ✅ |
| **合計** | **7** | **7 ✅** |

### 修改的檔案清單

| 檔案 | 修改類型 |
|------|----------|
| `docs/api/schemas/entities.yaml` | dueDate format: date → date-time |
| `docs/api/schemas/requests.yaml` | dueDate format 修正、UpdateTodo 補齊 lastSeenMessageId |
| `docs/api/schemas/accounts.yaml` | dueDate format 修正 |
| `docs/data-model/12-execution-tool.md` | project_admin → tag_admin、project_owner → owner |
| `docs/data-model/17-webhook.md` | project_admin → tag_admin、project_owner → owner |
| `docs/system/01-service-decomposition.md` | data_sheets → data_schemas |
| `docs/architecture.md` | Contact 來源類型矛盾修正 |
| `docs/plans/phase-02-org-project.md` | copy project 階段性範圍說明 |
| `docs/data-model/README.md` | 基礎設施表索引（含跨引用警示表標註） |
