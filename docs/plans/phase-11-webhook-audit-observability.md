# Phase 11：Webhook、外部 API、審計、可觀測性

**階段目標：** 實作 Webhook 出站通知、資料表外部 REST API、完整審計日誌、可觀測性基礎設施（結構化日誌、分散式追蹤、Prometheus 指標），使系統具備生產級的可審計性與可觀測性。

**前置依賴：** Phase 10 完成

---

## 後端任務

### B-11.1 Webhook 系統

**範圍：** 建立 Webhook 註冊、事件觸發、出站 HTTP 通知、重試機制。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0021_webhooks.sql`：
   - `webhooks` 表：id, project_id (FK), name (VARCHAR — Webhook 名稱，與 API schema 一致), url (VARCHAR), secret (VARCHAR — HMAC 簽章密鑰), enabled (BOOLEAN), event_types (JSONB — 訂閱的事件類型陣列), created_by (FK), created_at, updated_at, deleted_at
   - `webhook_event_logs` 表（命名與 API schema 的 WebhookLog 一致）：id, webhook_id (FK), event_type (VARCHAR), payload (JSONB), status (VARCHAR(20) — pending/success/failed), response_status (INTEGER, nullable), response_body (TEXT, nullable), attempts (INTEGER, DEFAULT 0), max_attempts (INTEGER, DEFAULT 3), next_retry_at (TIMESTAMPTZ, nullable), created_at, completed_at (TIMESTAMPTZ, nullable)（與 `docs/data-model/17-webhook.md` 一致：不含 `error_message`、`response_time_ms`、`is_test`；使用 `attempts` 而非 `attempt_count`）
3. 建立 `WebhookService`：
   - `register_webhook(actor, project_id, url, secret, event_types)` → 註冊 Webhook
   - `update_webhook(actor, webhook_id, updates)` → 更新
   - `delete_webhook(actor, webhook_id)` → 刪除
   - `list_webhooks(project_id)` → 列表
   - `dispatch_event(project_id, event_type, payload)` → 觸發：
     - 查詢該專案訂閱此 event_type 的所有 webhooks
     - 組裝 payload + HMAC-SHA256 簽章（`X-Webhook-Signature` header）
     - HTTP POST 至 webhook URL
     - 記錄結果至 webhook_event_logs
   - `retry_failed_events()` → 重試失敗事件（指數退避，最多 3 次，與 `docs/data-model/17-webhook.md` 一致）
4. **支援的 Webhook 事件類型**（與 `docs/api/schemas/webhooks.yaml` 一致）：
   - `task.created`, `task.updated`, `task.completed`, `task.deleted`
   - `todo.created`, `todo.completed`, `todo.updated`
   - `data_entry.created`, `data_entry.updated`, `data_entry.deleted`
   - `message.created`（不含對話內容，僅 metadata）
   - `member.added`, `member.removed`
   - `test`（Webhook 測試事件，用於驗證 Webhook 連線與簽章是否正確）
5. **安全**：
   - HMAC-SHA256 簽章驗證
   - 超時控制（10 秒）
   - 只允許 project owner 或 org_owner 管理
6. API 路由：
   - `GET /api/v1/projects/{projectId}/webhooks`
   - `POST /api/v1/projects/{projectId}/webhooks`
   - `GET /api/v1/projects/{projectId}/webhooks/{webhookId}` — 取得 Webhook 詳情
   - `PUT /api/v1/projects/{projectId}/webhooks/{webhookId}`
   - `DELETE /api/v1/projects/{projectId}/webhooks/{webhookId}`
   - `POST /api/v1/projects/{projectId}/webhooks/{webhookId}/test` — 發送測試事件
   - `GET /api/v1/projects/{projectId}/webhooks/{webhookId}/logs`
   - `POST /api/v1/webhooks/inbound/{endpointId}` — 接收外部系統的 Inbound Webhook 事件
   - （與 `docs/api/paths/webhooks.yaml` 一致）

**涉及檔案：**
- `migrations/0021_webhooks.sql`
- `src/modules/core/webhook/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/webhooks.rs`

**測試要求：**
- 整合測試：Webhook CRUD
- 整合測試：事件觸發 → HTTP POST → 簽章驗證
- 整合測試：重試機制
- 整合測試：權限控制
- API 測試：所有 Webhook 端點

**驗收標準：**
- [ ] Webhook 註冊與管理完整
- [ ] 事件觸發與 HTTP 通知正確
- [ ] HMAC 簽章正確
- [ ] 重試機制運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-11.2 資料表外部 REST API

**範圍：** 為資料表提供外部可存取的 REST API，供外部系統查詢與寫入結構化資料。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. API 路由（使用 API Key 認證，非 JWT）：
   > **注意：** Phase 11 完善 Phase 4 建立的 external data endpoint
   - `GET /external/v1/projects/{projectId}/task-templates` — 列出任務模板
   - `GET /external/v1/projects/{projectId}/task-templates/{templateId}/data` — 查詢聚合資料
   - `GET /external/v1/projects/{projectId}/tasks/{taskId}/data` — 查詢特定任務的資料
   - `PUT /external/v1/projects/{projectId}/tasks/{taskId}/data` — 外部系統更新資料
3. **API Key 管理**：
   - 建立 migration `0022_api_keys.sql`：
     - `api_keys` 表：id, project_id (FK), name, key_hash (VARCHAR), permissions (JSONB), created_by (FK), last_used_at, created_at, deleted_at
   - permissions JSONB 結構需明確定義：`{ "scopes": ["read", "write"], "data_access": { "template_ids": ["..."], "schema_ids": ["..."] } }`（至少區分 read/write scope 與資料表範圍限制）
   - `POST /api/v1/projects/{projectId}/api-keys` — 產生 API Key
   - `GET /api/v1/projects/{projectId}/api-keys` — 列出 API Keys
   - `DELETE /api/v1/projects/{projectId}/api-keys/{keyId}` — 撤銷
4. 外部 API 認證透過 `X-API-Key` header（`apiKeyAuth` security scheme）
5. 更新資料時觸發 DomainEvent：`SourceDataChanged`（觸發 AI 建議）

**涉及檔案：**
- `migrations/0022_api_keys.sql`
- `src/api/routes/data_external.rs`
- `src/api/middleware/api_key_auth.rs`
- `src/modules/core/api_key/mod.rs`, `service.rs`

**測試要求：**
- 整合測試：API Key 產生與驗證
- 整合測試：外部資料查詢與更新
- 整合測試：更新觸發 SourceDataChanged 事件
- API 測試：外部 API 端點

**驗收標準：**
- [ ] 外部 REST API 可查詢資料表聚合資料
- [ ] 外部系統可更新資料
- [ ] API Key 認證正確
- [ ] 更新觸發 AI 建議
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-11.3 完整審計日誌系統

**範圍：** 建立完整的審計日誌記錄，涵蓋所有寫入操作與 AI 互動。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0023_audit_logs.sql`：
   - `audit_logs` 表（按月分區）：id, actor_type (ENUM: account/system/ai/api_key), actor_id (UUID, nullable), action (VARCHAR), resource_type (VARCHAR), resource_id (UUID), context_type (VARCHAR, nullable — organization/project/task), context_id (UUID, nullable), details (JSONB — 含變更前後值), ip_address (INET, nullable), user_agent (TEXT, nullable), created_at
   - （api_key 指透過 API Key 進行的外部系統操作）
   - 建立當月 + 未來 3 個月的分區
3. 建立 `AuditService`（完善 Phase 0 的 audit 模組）：
   - 訂閱所有 DomainEvent，自動記錄審計日誌
   - `record_event(event: AuditEvent)` → 寫入審計日誌
   - `query_logs(filters) -> PaginatedAuditLogs` → 查詢審計日誌
   - **AI 審計**：記錄 prompt/response（敏感資料遮罩）
4. **審計覆蓋範圍**：
   - 帳號：登入、登出、Profile 更新
   - 組織/專案：CRUD、成員變更、設定變更
   - 任務：建立、狀態變更、刪除
   - AI：觸發、建議生成、人類決策、工具執行
   - 工具：執行記錄、外部工具呼叫
   - Email：收發記錄
   - 權限：角色變更、權限設定變更
5. **分區管理**：
   - Migration 建立當月 + 未來 3 個月的分區
   - 建立 Tokio scheduled task（cron-like），每月 1 日自動建立未來 3 個月的分區
   - **保留策略**：預設保留 12 個月的審計日誌，超過保留期的分區由管理員手動 `DROP PARTITION`（不自動刪除，避免法規問題）
   - 保留天數從環境變數 `AUDIT_RETENTION_DAYS` 讀取（預設 365）
6. API 路由：
   - `GET /api/v1/organizations/{orgId}/audit-logs` — 組織審計日誌
   - `GET /api/v1/projects/{projectId}/audit-logs` — 專案審計日誌

**涉及檔案：**
- `migrations/0023_audit_logs.sql`
- `src/modules/audit/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/audit.rs`

**測試要求：**
- 整合測試：各類操作自動產生審計日誌
- 整合測試：AI 審計（prompt/response 遮罩）
- 整合測試：審計日誌查詢（按時間、actor、action 過濾）
- 整合測試：分區正確路由

**驗收標準：**
- [ ] 所有寫入操作都有審計記錄
- [ ] AI prompt/response 正確遮罩
- [ ] 按月分區正確運作
- [ ] 審計日誌可查詢與過濾
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-11.4 可觀測性基礎設施

**範圍：** 建立結構化日誌、分散式追蹤（OpenTelemetry）、Prometheus 指標端點。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. **結構化日誌**：
   - 配置 `tracing-subscriber`：JSON 格式（生產）/ pretty 格式（開發）
   - 自動注入 TraceContext：request_id, account_id, project_id, task_id
   - 日誌層級規範：ERROR/WARN/INFO/DEBUG/TRACE
3. **分散式追蹤**：
   - 配置 `tracing-opentelemetry` + `opentelemetry-otlp`
   - HTTP middleware 自動建立根 span
   - 子 span 覆蓋：db.query, llm.call, tool.execute, email.send
   - 取樣策略：錯誤 100%、正常 10%（可配置）
4. **Prometheus 指標**：
   - 配置 `metrics` + `metrics-exporter-prometheus`
   - `/metrics` 端點
   - 指標：HTTP 請求（延遲、計數、錯誤率）、DB 連線池、WebSocket 連線數、AI pipeline 延遲、工具執行延遲
5. 環境變數配置：`LOG_FORMAT`, `OTEL_SERVICE_NAME`, `OTEL_EXPORTER_ENDPOINT`, `OTEL_TRACE_SAMPLE_RATE`, `METRICS_ENABLED`, `AUDIT_RETENTION_DAYS`, `AUDIT_AI_LOGGING_ENABLED`, `HEALTH_CHECK_DB_TIMEOUT`（與 `docs/system/12-observability.md` 一致）

**涉及檔案：**
- `src/observability/mod.rs`
- `src/observability/logging.rs`
- `src/observability/tracing.rs`
- `src/observability/metrics.rs`
- `src/api/middleware/tracing.rs`（擴展）

**測試要求：**
- 單元測試：TraceContext 注入
- 整合測試：結構化日誌格式正確
- 整合測試：`/metrics` 端點回傳 Prometheus 格式
- 整合測試：HTTP 請求自動記錄指標

**驗收標準：**
- [ ] 結構化 JSON 日誌含 request_id
- [ ] `/metrics` 端點可用
- [ ] OpenTelemetry 追蹤可輸出（OTLP）
- [ ] 關鍵操作都有 span 與指標
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-11.1 Webhook 與審計管理 UI

**範圍：** Webhook 管理介面、審計日誌檢視、API Key 管理。

**說明：**
1. `WebhookManagementView.vue`（專案設定內）：
   - Webhook 列表（含啟用/停用）
   - 新增 Webhook（URL、事件類型選擇、secret 設定）
   - Webhook 事件日誌檢視
2. `AuditLogView.vue`：
   - 審計日誌時間線
   - 按操作者、動作、時間過濾
   - 詳情展開（變更前後值比對）
3. `ApiKeyManagementView.vue`（專案設定內）：
   - API Key 列表
   - 產生新 Key（顯示一次後不再顯示）
   - 撤銷 Key

**涉及檔案：**
- `frontend/src/views/projects/WebhookManagementView.vue`
- `frontend/src/views/projects/AuditLogView.vue`
- `frontend/src/views/projects/ApiKeyManagementView.vue`

**測試要求：**
- 元件測試：各管理頁面渲染
- 元件測試：API Key 一次顯示邏輯

**驗收標準：**
- [ ] Webhook 管理完整
- [ ] 審計日誌可檢視與過濾
- [ ] API Key 管理完整
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 11 後，以下端到端流程可驗證：

1. **Webhook**：註冊 Webhook → 建立任務 → 接收到 task.created 事件通知 → 簽章驗證正確
2. **外部 API**：產生 API Key → 外部系統查詢資料表 → 外部系統更新資料 → 觸發 AI 建議
3. **審計日誌**：執行各種操作 → 審計日誌完整記錄 → 可按時間與操作者查詢
4. **可觀測性**：`/healthz` 正常 → `/metrics` 顯示指標 → 結構化日誌含 request_id
