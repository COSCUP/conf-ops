# Phase 9：MCP 工具執行引擎

**階段目標：** 實作統一的 MCP (Model Context Protocol) 工具執行引擎，含核心內建工具（Rust 直接呼叫）、可配置內建工具（SMTP/HackMD/Google Meet）、外部 MCP Server 整合（STDIO/SSE），建立工具設定繼承與權限控制。

**前置依賴：** Phase 8 完成

---

## 後端任務

### B-9.1 MCP 工具執行框架與核心內建工具

**範圍：** 建立統一的 MCP 工具定義、執行引擎、核心工具（8 種）。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0018_tools.sql`：
   - `tool_configs` 表：id, scope_type (VARCHAR(20) NOT NULL — 'organization'/'project'), scope_id (UUID NOT NULL), tool_type (VARCHAR(20) NOT NULL — builtin/external), tool_name (VARCHAR(255) NOT NULL), display_name (VARCHAR(255), nullable), description (TEXT, nullable), enabled (BOOLEAN, DEFAULT false), config (JSONB, DEFAULT '{}'), mcp_server_config (JSONB, nullable), created_at, updated_at, deleted_at, UNIQUE(scope_type, scope_id, tool_name)（與 `docs/data-model/12-execution-tool.md` 一致）
   - （工具權限透過 Project 的 `permission_settings.tool_permissions` 統一管理，不使用獨立的 member_tag_permissions 欄位）
   - `tool_executions` 表：id, task_id (FK), suggestion_id (FK, nullable), tool_name (VARCHAR), parameters (JSONB), result (JSONB), status (ENUM: success/error), executed_by (FK), executed_at, duration_ms (INT)
3. 建立 `ToolService`（trait + impl）：
   - `list_available_tools(project_id, member_tag_id?) -> Vec<ToolDefinition>`：
     - 列出可用工具（核心 + 已啟用的可配置 + 外部）
     - 根據 permission_settings.tool_permissions 過濾
   - `execute_tool(actor, task_id, tool_name, parameters, context) -> ToolResult`：
     - Placeholder 解析（呼叫 PrivacyEngine.resolve_placeholders）
     - 權限檢查
     - 路由至正確的執行器（internal / external）
     - 記錄執行結果至 tool_executions
     - 執行結果以 tool_execution 類型 message 加入對話
   - `resolve_placeholders(params, context) -> ResolvedParams`
4. **ToolDefinition 格式**（MCP 協定）：
   ```rust
   pub struct ToolDefinition {
       pub name: String,
       pub description: String,
       pub input_schema: serde_json::Value, // JSON Schema
   }
   ```
5. **核心內建工具**（永遠可用，Rust 直接呼叫）：
   - `createTask(template_id, owner_tag_id, name?)` → 建立任務（跨組協作）
   - `createTodo(task_id, title, parent_id?, assignee_ids?)` → 建立臨時待辦
   - `updateTodo(todo_id, status?, title?, assignee_ids?)` → 更新待辦
   - `upsertDataEntry(task_id, schema_id, values)` → 更新資料表
   - `shareDataToTask(source_task_id, target_task_id, field_mappings)` → 跨任務資料分享
   - `saveToProfile(key, value)` → 儲存至個人資料表
   - `upsertMemory(scope_type, scope_id, content)` → 更新記憶（memories 表無 title 欄位，與 `docs/data-model/13-memory.md` 一致）
   - `queryMemories(scope_type, scope_id, search?)` → 查詢記憶
   - （注意：不包含 sendNotification — 通知系統為確定性規則引擎，獨立於 AI 建議流程，所有通知由明確的動作或規則觸發，參考 `docs/architecture.md`）
6. 新增 DomainEvent：`ToolExecuted`, `ToolError`

**涉及檔案：**
- `migrations/0018_tools.sql`
- `src/modules/tools/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/modules/tools/executor.rs`
- `src/modules/tools/builtin/mod.rs`
- `src/modules/tools/builtin/create_task.rs`, `create_todo.rs`, `update_todo.rs`, `upsert_data_entry.rs`, `share_data.rs`, `save_to_profile.rs`, `upsert_memory.rs`, `query_memories.rs`
- `src/api/routes/tools.rs`

**測試要求：**
- 單元測試：每個核心工具的邏輯
- 整合測試：Placeholder 解析 → 工具執行 → 結果記錄
- 整合測試：權限檢查（無權限標籤無法使用受限工具）
- 整合測試：執行結果自動加入對話
- API 測試：工具列表與手動執行端點

**驗收標準：**
- [ ] 8 個核心工具全部可用（createTask, createTodo, updateTodo, upsertDataEntry, shareDataToTask, saveToProfile, upsertMemory, queryMemories）
- [ ] Placeholder 解析正確
- [ ] 權限控制正確
- [ ] 執行結果記錄完整
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-9.2 可配置內建工具（SMTP / HackMD / Google Meet）

**範圍：** 實作需要組織/專案層級設定的內建工具。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. **smtp/sendEmail 工具**：
   - 使用 Phase 7 的 EmailService
   - 參數：to, subject, body, reply_to_thread_id?
   - 需要 SMTP 設定（從 tool_configs 讀取，或使用全域 SMTP 設定）
   - 支援多信件串管理
3. **hackmd/createDocument 工具**：
   - 透過 HackMD API 建立共筆文件
   - 參數：title, content, permission
   - 需要 HackMD API Key（tool_configs）
4. **googleMeet/createMeeting 工具**：
   - 透過 Google Calendar API 建立會議
   - 參數：title, start_time, end_time, attendees
   - 需要 Google OAuth credentials（tool_configs）
5. **工具設定繼承**：
   - Organization 設定為預設值
   - Project 可覆寫 Organization 的設定
   - `get_effective_config(project_id, tool_name)` → 計算有效設定
6. API 路由：
   - `GET /api/v1/projects/{projectId}/tools` — 可用工具列表
   - `GET /api/v1/organizations/{orgId}/tool-configs` — 組織工具設定
   - `GET /api/v1/projects/{projectId}/tool-configs` — 專案工具設定
   - `POST /api/v1/projects/{projectId}/tool-configs` — 建立專案工具設定
   - `PUT /api/v1/projects/{projectId}/tool-configs/{configId}` — 更新專案工具設定
   - `DELETE /api/v1/projects/{projectId}/tool-configs/{configId}` — 刪除專案工具設定
   - （使用 configId (UUID) 而非 toolName 作為路徑參數，與 `docs/api/paths/tools.yaml` 一致）

**涉及檔案：**
- `src/modules/tools/builtin/send_email.rs`
- `src/modules/tools/builtin/hackmd.rs`
- `src/modules/tools/builtin/google_meet.rs`
- `src/modules/tools/config.rs`
- `src/api/routes/tools.rs`（擴展）

**測試要求：**
- 整合測試：sendEmail 工具（MailHog 驗證）
- 單元測試：設定繼承邏輯（Organization → Project 覆寫）
- 整合測試：工具設定 CRUD
- API 測試：工具設定端點

**驗收標準：**
- [ ] smtp/sendEmail 可寄信
- [ ] 設定繼承機制（Organization → Project）正確
- [ ] 工具設定 CRUD 完整
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-9.3 外部 MCP Server 整合（STDIO / SSE）

**範圍：** 支援連接外部 MCP Server，透過 STDIO 或 SSE 協定執行第三方工具。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/modules/tools/external/mod.rs`：
   - `ExternalToolExecutor` 結構體
   - **STDIO 通訊**：
     - 啟動外部 MCP Server process
     - 透過 stdin/stdout JSON-RPC 通訊
     - 管理 process 生命週期
   - **SSE 通訊**：
     - 連接遠端 MCP Server SSE 端點
     - 發送 HTTP POST 請求執行工具
     - 接收 SSE 串流結果
3. **MCP 協定實作**：
   - `initialize` — 初始化握手
   - `tools/list` — 取得工具列表
   - `tools/call` — 執行工具
4. **安全控制**：
   - 外部工具只能由 org_owner / project owner 設定
   - 執行時需要人類確認（透過 AI 建議流程）
   - 超時控制（預設 30 秒）
   - 執行結果的 sanitization
   - **沙箱隔離**（參考 `docs/system/07-mcp-tool-runtime.md`）：
     - L1：進程資源限制（CPU、記憶體、執行時間）
     - L2：網路存取限制（可選）
     - L3：檔案系統隔離（可選）
5. **工具設定**（tool_configs.config JSONB）：
   ```json
   {
     "type": "stdio",
     "command": "/path/to/mcp-server",
     "args": ["--flag"],
     "env": { "API_KEY": "..." }
   }
   // 或
   {
     "type": "sse",
     "url": "https://mcp.example.com/sse",
     "auth_header": "Bearer ..."
   }
   ```

**涉及檔案：**
- `src/modules/tools/external/mod.rs`
- `src/modules/tools/external/stdio.rs`
- `src/modules/tools/external/sse.rs`
- `src/modules/tools/external/mcp_protocol.rs`

**測試要求：**
- 單元測試：MCP 協定訊息序列化/反序列化
- 整合測試：STDIO 通訊（使用 mock MCP server）
- 整合測試：SSE 通訊（使用 mock HTTP server）
- 整合測試：超時控制
- 整合測試：安全控制（非 owner 無法設定外部工具）

**驗收標準：**
- [ ] STDIO MCP Server 可連接並執行工具
- [ ] SSE MCP Server 可連接並執行工具
- [ ] 超時與安全控制正確
- [ ] MCP 協定相容
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-9.1 工具管理與手動執行 UI

**範圍：** 工具設定管理、可用工具列表、手動工具執行介面。

**說明：**
1. `ToolConfigView.vue`（組織/專案設定內）：
   - 可用工具列表（含啟用/停用切換）
   - 工具設定表單（API Key、端點等）
   - 外部 MCP Server 設定（STDIO / SSE）
   - 標籤權限設定（哪些標籤可使用）
2. `ManualToolDialog.vue`：
   - 在任務對話中手動選擇工具執行
   - 根據工具的 input_schema 動態生成參數表單
   - Placeholder 預覽
   - 確認後執行
3. 整合至 AI 建議流程（F-8.1 的 SuggestionCard 中的工具執行）

**涉及檔案：**
- `frontend/src/views/settings/ToolConfigView.vue`
- `frontend/src/components/tools/ManualToolDialog.vue`
- `frontend/src/components/tools/ToolParameterForm.vue`

**測試要求：**
- 元件測試：ToolConfigView 渲染與設定
- 元件測試：ManualToolDialog 參數表單動態生成
- 元件測試：ToolParameterForm 各類型參數

**驗收標準：**
- [ ] 工具設定管理 UI 完整
- [ ] 手動工具執行可操作
- [ ] 動態參數表單正確生成
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 9 後，以下端到端流程可驗證：

1. **核心工具**：AI 建議 createTodo → 人類 accept → 待辦自動建立 → 結果顯示在對話
2. **跨組協作**：AI 建議 createTask (另一組) + shareDataToTask → 人類確認 → 跨組任務建立
3. **Email 工具**：AI 建議 sendEmail → 人類修改收件人 → 確認 → MailHog 收到信
4. **外部工具**：設定 MCP Server → 工具出現在可用列表 → AI 可建議使用 → 人類確認執行
5. **手動執行**：人類主動選擇工具 → 填寫參數 → 執行 → 結果記錄在對話
