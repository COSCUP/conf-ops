# Conf-Ops 全面規格審查報告 v2

跨 55+ 文件比對，涵蓋架構規格、資料模型、API 規格、系統文件、實作計劃。

---

## 一、嚴重矛盾（CRITICAL — 需優先解決）

### 1. AI Pipeline 事件持久化架構矛盾
- **`architecture.md`**：描述使用 "Tokio in-memory queue"，重啟後從對話掃描未處理事件恢復
- **`system/04-ai-pipeline.md` §7.3**：實作完整的 PostgreSQL `ai_pipeline_events` 表，含 worker polling、retry、timeout
- **問題**：這是兩套完全不同的架構，一個是 best-effort 恢復，一個是持久化保證。需要統一決定

### 2. CRDT 同步協定格式矛盾（二進位 vs JSON）
- **`paths/conversations.yaml`**：定義 `/sync` 端點接收與回傳 `application/octet-stream`（yrs 二進位格式）
- **`schemas/requests.yaml`**：定義了基於 JSON 的 `CrdtSyncRequest` 與 `CrdtSyncResponse`
- **問題**：API 規格在同步協定層級存在直接衝突，需明確統一採用原生二進位流還是 JSON 封裝

### 3. `lastSeenMessageId` 在多個 API 端點遺漏
- **`architecture.md` §9**：明確要求所有對話寫入操作都須附帶 `lastSeenMessageId`，列舉了 `SendMessage`、`UpsertDataEntry`、`UpdateTodo`、`UpdateTaskStatus`、`SuggestionDecision`、`ToolExecute`
- **API 現狀**：
  - `UpdateTaskStatusRequest`（`schemas/requests.yaml`）**缺少** `lastSeenMessageId` 必填欄位
  - `UpsertDataEntry`、`SuggestionDecision` 等端點也需逐一確認
- **問題**：核心的「已讀保護」機制在 API 層未完整實作

### 4. API Key 實體定義遺漏
- **`architecture.md`**：提到 `/external/v1/` 使用「專案層級 API Key（`X-API-Key` header）認證」
- **資料模型**：17 個實體文件中**沒有任何 API Key 表的定義**
- **問題**：認證機制引用了不存在的實體

### 5. 提醒系統（ReminderSystem）架構文件與配置 API 缺口
- **架構現狀**：`architecture.md` 僅一句話帶過，缺乏流程描述
- **API 現狀**：缺乏配置提醒規則（如：設定「即將到期」的提前天數）的 CRUD 端點
- **資料模型**：`data-model/15-notification.md` 已完整定義表結構，但規格與 API 均未對齊
- **問題**：該關鍵子系統在實作層次上有嚴重脫節

### 6. Worker 部署重複觸發風險
- **`system/10-notification-system.md`**：聲稱「Worker 為單一 replica 避免重複」
- **`system/02-deployment-architecture.md`**：Docker Compose 顯示 `confops-app` 和 `confops-worker` 兩個容器
- **問題**：如果兩者都能觸發提醒，會造成重複。缺少分散式鎖或 leader election 機制

### 7. Magic Link 自動註冊行為未寫入架構
- **`system/08-authentication.md`**：「帳號不存在時自動建立」
- **`architecture.md`**：Account 節完全沒提到自動註冊，暗示只有已有帳號才能登入
- **問題**：關鍵的使用者入口行為在架構規格中缺失

---

## 二、中度落差（MEDIUM — 應盡快修正）

### 8. `POST /api/v1/ai/resolve-placeholders` 端點遺漏
- **架構要求**：`architecture.md` §6 規定前端需透過此端點預覽佔位符解析結果
- **規格現狀**：OpenAPI spec (`paths/ai-suggestions.yaml`) 與 Phase 8 計劃中完全找不到此端點
- **問題**：隱私引擎的「人為確認」環節在 API 層面無法落地

### 9. AI 管線與工具執行環境的實作順序落差
- **Phase 8**：實作建議與決策邏輯，但真正的工具執行引擎在 Phase 9 才完成
- **問題**：這會導致 Phase 8 期間無法進行端對端的執行驗證（Execution Verification），增加 Phase 9 的整合風險

### 10. Contact 是否算入 Task Participants
- **`architecture.md` §8 參與人計算**：只列出 Member 相關來源（ownerTag 成員、@mention 成員、建立者、assignees）
- **但同時描述**：Contact 可透過 `email_inbound` 參與任務對話
- **問題**：Contact 在對話中發言，但不算 participant？邏輯上不完整

### 11. CRDT Position Tracking 架構文件缺失
- **`data-model/09-task-conversation.md`**：定義了 `last_seen_positions` 表追蹤 Yjs clock positions
- **`architecture.md`**：完全沒有提到此 CRDT 關鍵機制
- **問題**：CRDT 狀態恢復的核心表在架構中無描述

### 12. 記憶繼承鏈查詢方案未定
- **`system/04-ai-pipeline.md`**：同時提出兩種方案：PostgreSQL CTE 遞迴查詢 vs `moka` cache
- **問題**：兩個競爭方案共存，未明確在實作指南中選定最終採用哪一個

### 13. 外部 API Base URL 未在 OpenAPI servers 中定義
- **`openapi.yaml` servers 區塊**：只有 `/api/v1` base URL
- **但存在 `/external/v1/*` 路徑**
- **問題**：外部 API 路由缺少 server 定義，客戶端無法正確解析

### 14. `StaleConversationErrorResponse` Schema 未正式定義
- **`paths/conversations.yaml`**：409 錯誤回應引用了 `StaleConversationErrorResponse`
- **`schemas/responses.yaml`**：此 Schema **未正式定義**
- **問題**：OpenAPI 引用了不存在的定義

### 15. `conversation_states` 讀取追蹤架構缺失
- **`data-model/09-task-conversation.md`**：定義了 `conversation_states` 表含 `last_read_message_id`
- **`architecture.md`**：僅提到 `lastSeenMessageId` 的操作保護機制
- **問題**：`lastSeenMessageId`（操作保護）vs `last_read_message_id`（讀取追蹤）職責混淆

---

## 三、輕度落差（LOW — 建議修正）

### 16. `saveToProfile` 工具動態欄位定義不明確
- **問題**：當 AI 建議建立新個人資料欄位時，API 如何傳遞 Metadata（Label, Type, Description）的定義在 `requests.yaml` 中不夠具體
- **建議**：在工具執行參數中定義統一的 `metadata` 傳遞標準

### 17. 專案複製跨 Phase 未說明
- **架構**：描述為單一操作的完整複製；**Plans**：分成 Phase 2 (基本) 與 Phase 12 (深複製)
- **建議**：架構文件加註複製功能分階段實作

### 18. 工具權限矩陣未文件化
- **建議**：將 `toolPermissions` 的權限矩陣定義明確寫入系統文件

### 19. S3 儲存後端狀態不明
- **建議**：明確標註 S3 支援是 Day 1 功能還是未來規劃

### 20. Notification `delivered_channels` 欄位遺漏
- **Phase 10 計劃**：包含 `delivered_channels (JSONB)`；**資料模型**：無此欄位
- **建議**：同步更新資料模型

### 21. 資料表變更無版本歷史
- **問題**：`data_entries` 只有更新時間，沒有欄位級別的變更追蹤
- **建議**：考慮透過 audit log 或 CRDT operation log 追蹤

### 22. 記憶自動提取（auto_extracted）確認流程未展開
- **問題**：Phase 8 提到欄位但沒有描述 UI 確認流程
- **建議**：補充自動提取 → 審核 → 存入的 UI 規格

---

## 四、本次補充審查（2026-02-19，排除所有 `spec-review-report.md`）

> 以下為新增發現，尚未併入上方統計表。

### A1. OpenAPI `servers` 與外部路徑前綴不一致（CRITICAL）
- **`docs/api/openapi.yaml`**：`servers` 僅定義 `/api/v1`
- **同檔 paths**：同時宣告 `/external/v1/projects/...`
- **問題**：在單一 server base 下混入外部前綴，會造成 client 端 URL 組合與 SDK 生成歧義

### A2. Reminder 類型列舉在 API path / schema / 架構文件三方不一致（CRITICAL）
- **`docs/architecture.md`**：`due_date_approaching` / `due_date_overdue` / `todo_stale`
- **`docs/api/schemas/entities.yaml#/ReminderResponse`**：同上列舉
- **`docs/api/paths/projects.yaml`（建立提醒 request）**：`due_date` / `overdue` / `custom`
- **問題**：同一領域概念在 request 與 response、架構規格使用不同 enum，會導致 API 契約不可預期

### A3. `scheduled_reminders` 資料模型定義互相矛盾（CRITICAL）
- **`docs/data-model/README.md` ER 區塊**：`task_id`, `remind_at`, `sent`
- **`docs/data-model/15-notification.md` 正式 DDL**：`todo_id`, `trigger_at`, `fired`（並含 `notification_id`, `config`）
- **問題**：總覽與細部定義衝突，會直接影響 migration 與 repository 欄位命名

### A4. Contact 專案視角查詢端點在架構有、OpenAPI 無（MEDIUM）
- **`docs/architecture.md`**：提到 `GET /api/v1/projects/{projectId}/contacts?includeTags=true`
- **OpenAPI (`docs/api/paths/contacts.yaml`)**：僅有 `/organizations/{orgId}/contacts...`
- **問題**：架構文件宣告的查詢能力無對應 API 契約

### A5. 權限命名體系混用（MEDIUM）
- **角色 enum / 系統規格**：`owner` / `tag_admin` / `member`（`docs/api/schemas/common.yaml`, `docs/system/09-authorization.md`）
- **多個 path `x-permissions`**：使用 `project_owner` / `project_member`
- **問題**：權限詞彙混用，會使授權中介層對應規則與文件理解產生落差

---

## 統計摘要

| 嚴重程度 | 數量 | 性質 |
|---------|------|------|
| **CRITICAL** | 7 | 架構矛盾、關鍵遺漏、協定衝突 |
| **MEDIUM** | 8 | 實作落差、不一致 |
| **LOW** | 7 | 文件不完整、參數定義不明 |
| **總計** | **22** | |

## 建議優先處理順序

1. **統一 CRDT 同步協定格式**（二進位 vs JSON）
2. **統一 AI Pipeline 事件持久化方案**（in-memory vs PostgreSQL）
3. **補齊 `lastSeenMessageId`** 到所有需要的 API request schema
4. **補齊 `/api/v1/ai/resolve-placeholders` 端點到 OpenAPI**
5. **新增 API Key 資料模型定義**
6. **解決 AI 管線與工具執行引擎（Phase 8 vs 9）的整合驗證間隔**
7. **解決 Worker 重複觸發風險**（加入分散式鎖）
