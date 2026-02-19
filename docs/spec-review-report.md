# Conf-Ops 規格審查報告

> 審查日期：2026-02-17
> 審查範圍：`docs/api/`、`docs/data-model/`、`docs/system/`、`docs/architecture.md`
> 審查角色：OpenAPI 審查、資料模型審查、系統架構審查、功能覆蓋審查、前端工程師、後端工程師、PM

---

## 摘要

| 嚴重度 | OpenAPI | 資料模型 | 系統架構 | 功能覆蓋 | 前端 | 後端 | PM | 合計 |
|--------|---------|---------|---------|---------|------|------|-----|------|
| Critical | 4 | 3 | 2 | — | 4 | 0 | — | **13** |
| Major | 9 | 5 | 6 | — | 7 | 6 | — | **33** |
| Minor | 6 | 7 | 10 | — | 7 | 11 | — | **41** |
| Gap/建議 | 3 | — | — | 6 gaps | 8 | — | 12+ | **29+** |

**整體評價：架構設計品質高，功能覆蓋率 ~95%。主要問題集中在文件間一致性（環境變數命名、schema 格式、x-permissions 格式）和 API 端點遺漏（檔案儲存、審計日誌、WebSocket token）。無技術上不可行的 Critical 項目。**

---

## 一、Critical 問題（必須修復）

### 1.1 OpenAPI $ref 與結構問題

| # | 問題 | 檔案位置 |
|---|------|---------|
| C1 | `conversations.yaml` 的 WebSocket `/ws` 和 attachments 路徑未在 `openapi.yaml` 引用，端點不可達 | `openapi.yaml` |
| C2 | `conversations.yaml:348` 引用 `responses.yaml#/SuggestionGroupResponse` 不存在（應為 `entities.yaml#/...`） | `conversations.yaml:348` |
| C3 | `conversations.yaml:373` 引用 `responses.yaml#/NotificationResponse` 不存在（應為 `entities.yaml#/...`） | `conversations.yaml:373` |
| C4 | `updateNotificationPreferences` operationId 重複出現在 `accounts.yaml:299` 和 `notifications.yaml:236` | `accounts.yaml`, `notifications.yaml` |

### 1.2 資料模型 — VARCHAR vs ENUM 不一致

| # | 問題 | 檔案位置 |
|---|------|---------|
| C5 | Task status：DB 用 `VARCHAR`（無約束），API 用 `enum: [pending, in_progress, completed, cancelled]` | `data-model/08-task.md` vs `common.yaml:200-207` |
| C6 | Todo status/type：DB 用 `VARCHAR`，API 用 `enum` | `data-model/10-todo.md` vs `common.yaml:209-221` |
| C7 | Message source_type：DB 用 `VARCHAR`，API 用 `enum` | `data-model/09-task-conversation.md` vs `common.yaml:233-240` |

**建議：** 統一使用 PostgreSQL ENUM type（與 `org_role`、`member_role`、`project_status` 一致），提供資料庫層防禦。

### 1.3 系統架構 — 安全與 API 缺口

| # | 問題 | 檔案位置 |
|---|------|---------|
| C8 | Magic Link 行為矛盾：API 說「帳號不存在自動建立」，系統文件說「不論是否存在回傳相同訊息防止列舉」 | `auth.yaml:179` vs `08-authentication.md:181` |
| C9 | 檔案儲存 API 端點完全缺失：`11-file-storage.md` 定義了 upload/download/presign 端點，但 `openapi.yaml` 中沒有任何 `/files/*` 路徑 | `11-file-storage.md:157-225` vs `openapi.yaml` |

### 1.4 前端串接阻塞

| # | 問題 | 檔案位置 |
|---|------|---------|
| C10 | WebSocket 認證 token 無取得端點：WS 需要 query param `token`，但無 REST API 可取得此 token | `conversations.yaml:309-314` |
| C11 | Magic Link verify 是 GET 請求但前端收到 JSON 而非重導向，使用者從 Email 點擊後無法正常導流 | `auth.yaml:212-257` |
| C12 | 附件上傳流程不完整：JSON body 需 `storagePath`（前端不可能知道），`SendMessageMultipartRequest` schema 幾乎為空 | `conversations.yaml:128-211`, `requests.yaml:639-693` |
| C13 | CRDT sync 端點格式矛盾：API 定義 JSON（`application/json`），系統文件定義 binary（`application/octet-stream`） | `conversations.yaml:226-275` vs `03-crdt-implementation.md:371-382` |

---

## 二、Major 問題（應該修復）

### 2.1 OpenAPI 規格問題

| # | 問題 | 說明 |
|---|------|------|
| M1 | Tag 名稱不一致 | `ExternalAPI`（data-external.yaml）vs `ExternalData`（openapi.yaml）；`ToolConfigs` 未在 openapi.yaml 宣告 |
| M2 | 分頁參數遺漏 | `listOrganizations`、`listOrganizationMembers`、`listProjectMembers` 缺少 cursor/limit 參數 |
| M3 | TokenPair schema 缺 refreshToken | schema 只有 accessToken，但 examples 中包含 refreshToken |
| M4 | 錯誤回應範例格式不符 RFC 7807 | 多處用 `{ error: { code, message } }` 而非 ProblemDetails `{ type, title, status, detail }` |
| M5 | 通知偏好端點重複 | `/accounts/me/notification-preferences` 和 `/notifications/preferences` 功能相同 |
| M6 | DELETE with requestBody | `webPushUnsubscribe` 使用 DELETE + body，許多 HTTP client 不支援 |
| M7 | x-permissions 格式不一致 | 有的用 structured object、有的用 string array、有的用 role 名稱不在授權文件定義中 |

### 2.2 資料模型問題

| # | 問題 | 說明 |
|---|------|------|
| M8 | AuditLog ip_address 型別不一致 | DB 用 `INET`，API 用 `string`（缺 format） |
| M9 | EmailMessage 缺少 `rawHeaders` | DB 有 `raw_headers JSONB`，API response schema 未包含 |
| M10 | Webhook `secret` 寫入端不明 | 讀端正確用 `hasSecret: boolean`，但 create/update request schema 缺少 `secret` 欄位 |
| M11 | TaskTemplate 無直接 project_id FK | 需透過 tag 間接關聯，「查詢專案所有模板」效率待確認 |

### 2.3 系統架構一致性問題

| # | 問題 | 說明 |
|---|------|------|
| M12 | 環境變數命名不一致 | `JWT_SECRET` vs `AUTH_JWT_SECRET`、`LLM_API_BASE_URL` vs `LLM_API_URL` 等（跨 01、02、04 等文件） |
| M13 | CRDT sync WebSocket 訊息類型命名不一致 | 系統文件用 Rust enum `SyncStep1/SyncStep2`，API 用 JSON event `crdt_update/send_message` |
| M14 | Auth 表名不一致 | `08-authentication.md` 用 `refresh_tokens`，`01-service-decomposition.md` 用 `sessions` |
| M15 | `/healthz`、`/readyz`、`/metrics` 未在 OpenAPI 定義 | 系統文件和部署架構都引用這些端點 |

### 2.4 前端串接 Major 問題

| # | 問題 | 說明 |
|---|------|------|
| M16 | WebSocket text/binary frame 規範不明 | JSON events 和 CRDT binary updates 混合傳輸，缺明確 frame type 規範 |
| M17 | CrdtSyncRequest 的 entityType/entityId 語意不明 | URL 已有 taskId，body 又要 entityType + entityId |
| M18 | AI 建議 parameters 只有 `type: object` | 前端無法為 modify_and_accept 生成編輯表單 |
| M19 | MessageResponse content 只有 `type: object` | 不同 sourceType 對應不同結構但無 discriminated union |
| M20 | ProfileSchemaField.type 無 enum | 前端不知道支援哪些欄位類型 |

### 2.5 後端技術 Major 問題

| # | 問題 | 說明 |
|---|------|------|
| M21 | Refresh Token Rotation Race Condition | 並發 refresh 請求可能導致 token 被錯誤撤銷，需原子操作 + grace period |
| M22 | AI Pipeline In-Memory Queue 重啟丟事件 | Tokio mpsc channel 無持久化，服務重啟丟失排隊中的 AI 請求 |
| M23 | 記憶繼承鏈查詢 N+1 | 沿 scope chain（account → org → project → tag → template → task）需多次 DB 查詢 |
| M24 | crdt_operations 表 snapshot 大小控制 | 大型任務的 CRDT state snapshot 可能很大，需壓縮/分段策略 |
| M25 | ~~LMTP Server crate 成熟度~~ | ~~已移除 LMTP，改用 AWS SES HTTP Webhook 收信~~ |
| M26 | 外部 MCP Server STDIO 沙箱隔離 | 外部進程通訊需要沙箱方案（seccomp/namespace 或 container） |

---

## 三、功能覆蓋缺口

### 3.1 缺失的 API 端點

| 優先級 | 端點 | 用途 | 發現者 |
|--------|------|------|--------|
| **P0** | `POST /tasks/{taskId}/conversation/ws-token` | WebSocket 認證 token 取得 | 前端、PM |
| **P0** | `POST/GET /files/upload`, `GET /files/{fileId}/download` | 檔案儲存 API | 系統架構、前端 |
| **P0** | CRUD `/projects/{projectId}/api-keys` | 外部 API Key 管理 | 覆蓋、PM |
| **P1** | `GET /projects/{projectId}/audit-logs` | 審計日誌查詢 | 覆蓋、PM |
| **P1** | CRUD `/organizations/{orgId}/tool-configs` | 組織工具設定（目前只有 GET） | 覆蓋 |
| **P1** | `GET /projects/{projectId}/reminders` | 提醒系統查詢與設定 | PM |
| **P1** | `GET /projects/{projectId}/stats` | 專案統計儀表板 | PM |
| **P1** | `GET /tasks/{taskId}/email-threads` | Email Thread 查詢 | PM |
| **P1** | `GET /notifications/web-push/vapid-key` | VAPID public key | 前端 |
| **P2** | `GET /healthz`, `GET /readyz`, `GET /metrics` | 運維端點 | 系統架構 |
| **P2** | `POST /memories/{id}/versions/{v}/restore` | 記憶版本回滾 | PM |
| **P2** | `DELETE /projects/{projectId}/members/me` | 成員自行退出 | PM |
| **P2** | `GET/DELETE /auth/passkeys` | Passkey 列表與刪除 | 覆蓋 |
| **P2** | `DELETE /tasks/{taskId}/todos/{todoId}` | 待辦事項刪除 | 覆蓋 |
| **P3** | 批次指派成員到標籤 | 大量成員管理 | PM |
| **P3** | 資料表 CSV/Excel 匯出 | 資料匯出 | PM |

### 3.2 功能覆蓋率

- **核心業務流程（任務、對話、AI 建議、CRDT、工具）**：100% 覆蓋
- **認證/授權**：95%（缺 Passkey 管理、API Key 管理）
- **管理功能（審計、統計、提醒）**：60%（多個管理端點缺失）
- **整體**：~95%

---

## 四、跨審查共識問題（多個審查角色同時發現）

以下問題被 2 個以上審查角色獨立發現，優先處理：

1. **WebSocket token 取得端點缺失** — 前端 ×、PM ×
2. **檔案上傳/下載 API 缺失** — 系統架構 ×、前端 ×
3. **通知偏好端點重複** — OpenAPI ×、覆蓋 ×、PM ×
4. **API Key 管理端點缺失** — 覆蓋 ×、PM ×
5. **CRDT sync 格式矛盾（JSON vs binary）** — 系統架構 ×、前端 ×
6. **審計日誌查詢 API 缺失** — 覆蓋 ×、PM ×
7. **Magic Link 行為矛盾** — 系統架構 ×、前端 ×
8. **環境變數命名不一致** — 系統架構（跨 3+ 文件）

---

## 五、建議修復順序

### Phase 1：Critical（阻塞開發）
1. 修復 OpenAPI $ref 錯誤（C1-C4）— 純文件修正
2. 統一 Magic Link 行為決策（C8）— 安全設計決策
3. 新增檔案儲存 API 端點（C9, C12）— 新增 path file
4. 新增 WebSocket token 取得端點（C10）
5. 統一 CRDT sync 格式（C13）— 設計決策
6. 修正 Magic Link verify 前端導流（C11）

### Phase 2：Major（影響實作品質）
1. 補齊分頁參數（M2）
2. 修正 Tag 名稱、operationId 重複（M1, C4）
3. 統一錯誤回應格式為 RFC 7807（M4）
4. 為 MessageResponse content 加入 discriminated union（M19）
5. 統一環境變數命名（M12）
6. 決定 Refresh Token 並發安全方案（M21）

### Phase 3：Gap 補齊
1. 新增 API Key 管理端點
2. 新增審計日誌查詢端點
3. 合併重複的通知偏好端點（M5）
4. 新增運維端點（/healthz, /readyz, /metrics）

### Phase 4：Minor 修正與優化
- 資料模型 VARCHAR → ENUM（C5-C7）
- x-permissions 格式統一
- 缺失的 schema 欄位補齊
- 其他 Minor 項目

---

## 六、後端技術可行性評估

**整體結論：架構技術上完全可行，無不可行項目。**

需要在實作前決定的技術方案：
1. Refresh Token Rotation：採用 grace period（如 30 秒）+ DB advisory lock
2. AI Pipeline Queue：Tokio mpsc + PostgreSQL 持久化層（確保重啟不丟事件）
3. 記憶繼承鏈：建議物化 scope chain 或使用 CTE 遞迴查詢
4. Email 收信：使用 AWS SES + SNS HTTP Webhook，不再需要自建 LMTP Server

---

## 七、各審查角色原始報告索引

| 角色 | 問題數 | 摘要 |
|------|--------|------|
| **OpenAPI 審查** | 22（4C/9M/6m/3S） | $ref 錯誤、operationId 重複、分頁遺漏、Tag 不一致 |
| **資料模型審查** | 15（3C/5M/7m） | VARCHAR vs ENUM、型別不一致、缺失欄位 |
| **系統架構審查** | 18（2C/6M/10m） | Magic Link 矛盾、檔案 API 缺失、環境變數不一致 |
| **功能覆蓋審查** | 6 gaps + 3 partial | Passkey 管理、API Key、org tool config CUD、審計日誌 |
| **前端工程師** | 22（4C/7M/7m/8S） | WS token、附件上傳、CRDT 格式、discriminated union |
| **後端工程師** | 17（0C/6M/11m） | Refresh token race、AI queue 持久化、LMTP 成熟度 |
| **PM** | 19+ | 審計日誌、提醒、統計儀表板、批次操作、邊界案例 |

---

## 八、驗證與測試

### 驗證工具

| 工具 | 指令 | 檢查項目 |
|------|------|---------|
| 完整驗證 | `make validate` | 執行所有檢查 |
| OpenAPI Lint | `make lint` | 語法、結構、最佳實踐 |
| Bundle 驗證 | `make bundle` | 所有 $ref 是否解析成功 |
| 交叉引用 | `make check-refs` | 資料模型↔API schema、ENUM 一致性、環境變數命名 |
| HTML 文件 | `make build-docs` | 產出 Redocly HTML 文件 |

### 執行方式

```bash
# 安裝依賴
npm install -g @redocly/cli
pip install pyyaml  # for cross-ref checker

# 執行完整驗證
make validate

# 僅檢查 OpenAPI
make lint

# 重新產出文件
make build-docs
```

### 驗證腳本說明

#### 1. `scripts/validate-openapi.sh`

OpenAPI 規格完整性驗證：

- **[1/5] Linting**：使用 Redocly CLI 檢查 OpenAPI 規範符合性、最佳實踐、常見錯誤
- **[2/5] Bundling**：嘗試將所有分離的 YAML 檔案 bundle 成單一檔案，驗證所有 `$ref` 引用是否正確解析
- **[3/5] operationId 唯一性**：掃描所有 path 檔案，確保沒有重複的 operationId
- **[4/5] Tag 一致性**：檢查 path 檔案中使用的 tags 是否都在 `openapi.yaml` 中宣告
- **[5/5] 分頁檢查**：檢查所有 `list*` 端點是否包含 cursor 分頁參數

#### 2. `scripts/check-cross-refs.py`

跨文件交叉引用一致性檢查：

- **[1/3] Entity Coverage**：比對資料模型文件中的 CREATE TABLE 與 API schema 中的 Entity Response，確保主要實體都有對應的 API 定義
- **[2/3] Enum Consistency**：比對資料模型中的 `CREATE TYPE ... AS ENUM` 與 API `common.yaml` 中的 enum 定義，確保值完全一致
- **[3/3] Environment Variables**：掃描系統文件中的環境變數命名，檢查潛在的命名不一致（如 `JWT_SECRET` vs `AUTH_JWT_SECRET`）

#### 3. `scripts/validate-all.sh`

完整驗證流程包裝腳本，依序執行上述兩個驗證腳本並匯總結果。

### CI 整合建議

將 `make validate` 加入 CI pipeline，在每次 PR 時自動驗證規格文件的一致性：

```yaml
# .github/workflows/validate-specs.yml 範例
name: Validate Specifications

on:
  pull_request:
    paths:
      - 'docs/**'
  push:
    branches:
      - main
      - v2

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - name: Install dependencies
        run: |
          npm install -g @redocly/cli
          pip install pyyaml
      - name: Run validation
        run: make validate
```

### 已知驗證覆蓋的問題

以下審查報告中發現的問題會被驗證腳本自動檢查：

- ✓ **C1-C3**：`$ref` 解析錯誤 → `validate-openapi.sh` [2/5] Bundling
- ✓ **C4**：operationId 重複 → `validate-openapi.sh` [3/5] operationId uniqueness
- ✓ **C5-C7**：ENUM 一致性 → `check-cross-refs.py` [2/3] Enum Consistency
- ✓ **M1**：Tag 名稱不一致 → `validate-openapi.sh` [4/5] Tag consistency
- ✓ **M2**：分頁參數遺漏 → `validate-openapi.sh` [5/5] Pagination check（會產生警告）
- ✓ **M12**：環境變數命名不一致 → `check-cross-refs.py` [3/3] Environment Variables
- ✓ **功能覆蓋 3.1**：部分缺失端點 → `check-cross-refs.py` [1/3] Entity Coverage（會產生警告）

### 未來擴充方向

1. **Schema 型別一致性檢查**：自動比對資料模型 SQL 欄位型別與 API schema 型別（如 `INET` vs `string`）
2. **x-permissions 格式統一驗證**：檢查所有端點的 `x-permissions` 結構一致性
3. **Request/Response 對應檢查**：驗證 POST/PUT 的 request schema 與 GET 的 response schema 欄位對應
4. **範例資料合法性驗證**：使用 JSON Schema validator 檢查所有 example 是否符合對應 schema
