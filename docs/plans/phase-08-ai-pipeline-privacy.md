# Phase 8：AI 建議管線與隱私引擎

**階段目標：** 實作 AI-Human 協作迴圈（trigger → context collection → privacy masking → LLM call → suggestion → decision → execution → record）、隱私引擎（placeholder 系統）、Memory 多層級繼承鏈，使 AI 可根據上下文在任務中提出建議並由人類決策執行。

**前置依賴：** Phase 7 完成

> **工具執行引擎依賴說明：** AI 建議管線中的工具執行引擎（Tool Executor）在 Phase 9 完成。Phase 8 期間，AI 建議的生成與人類決策流程已完整實作，但工具實際執行部分以 mock tool executor 進行端對端驗證。Phase 9 完成後替換為正式的工具執行引擎。

---

## 後端任務

### B-8.1 Memory 資料表與多層級繼承鏈

**範圍：** 建立 memories、library_documents 資料表，實作 6 層記憶繼承鏈與查詢。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0016_memories.sql`：
   - `memories` 表：id, scope_type (VARCHAR(20): account/organization/project/member_tag/task_template/task), scope_id (UUID), content (TEXT), source (VARCHAR(20): manual/auto_extracted, DEFAULT 'manual'), library_ref (UUID, nullable — 引用 library_documents), created_by (FK → accounts), created_at, updated_at, deleted_at
     （注意：無 title 欄位，與 `docs/data-model/13-memory.md` 一致；使用 library_ref 而非 library_document_id）
   - `library_documents` 表：id, scope_type (VARCHAR(20)), scope_id (UUID), title (VARCHAR), content (TEXT), created_by (FK → accounts), created_at, updated_at, deleted_at（與 `docs/data-model/13-memory.md` 一致，不含 `organization_id` 欄位）
   - `memory_versions` 表：id, memory_id (FK), content (TEXT), changed_by (FK → accounts), created_at（與 `docs/data-model/13-memory.md` 一致）
   - `library_document_versions` 表：id, document_id (FK), title (VARCHAR, nullable), content (TEXT), changed_by (FK → accounts), created_at（與 `docs/data-model/13-memory.md` 一致）
     （版本控制使用獨立的 versions 表，而非主表的 version 欄位）
3. 建立 `MemoryService`：
   - `upsert_memory(actor, scope_type, scope_id, content, source?, library_ref?)`（無 title 參數） → 新增或更新記憶
   - `query_memories(scope_type, scope_id, search_text?) -> Vec<Memory>` → 查詢指定範圍的記憶
   - `collect_inherited_memories(task_id) -> InheritedMemories` → 蒐集完整繼承鏈：
     ```
     Account memories (個人偏好)
     + Organization memories (組織知識)
     → Project memories (專案經驗)
     → MemberTag memories (組別特定)
     → TaskTemplate memories (模板流程)
     → Task memories (任務特定)
     ```
   - `delete_memory(actor, memory_id)` → 軟刪除
4. **Library Document CRUD**：
   - 用於存放詳細的 SOP、範本、指南
   - Memory 的 content 為摘要，library_ref 引用完整文件
   - 支援版本控制（library_document_versions 表）
5. **多型態外鍵驗證**：`memories` 與 `library_documents` 的 `scope_type`/`scope_id` 寫入時，`MemoryService` 必須根據 `scope_type` 呼叫對應模組 Service 驗證 scope_id 存在性
6. 使用 moka cache 快取繼承鏈結果（key = task_id，TTL = 5 分鐘）
7. 繼承鏈查詢使用 PostgreSQL CTE 遞迴查詢優化，避免 N+1 問題（單一 SQL 查詢完成 6 層繼承鏈蒐集，效能要求 < 5ms，參考 `docs/system/04-ai-pipeline.md` 7.2）
   - **重要：** 繼承鏈嚴格依據任務的 `ownerTag`（歸屬成員標籤）決定，不受操作者個人標籤影響（與 `docs/architecture.md` §3 記憶繼承鏈一致）
8. 記憶支援 CRDT（多人同時編輯記憶內容）
9. API 路由：
   - `GET /api/v1/memories?scopeType=...&scopeId=...`
   - `POST /api/v1/memories`
   - `PUT /api/v1/memories/{memoryId}`
   - `DELETE /api/v1/memories/{memoryId}`
   - Library documents 子路由
10. 新增 DomainEvent：`MemoryUpserted`

**涉及檔案：**
- `migrations/0016_memories.sql`
- `src/modules/ai/memory/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/memories.rs`

**測試要求：**
- 整合測試：Memory CRUD（6 種 scope_type）
- 整合測試：繼承鏈蒐集（驗證 6 層合併正確）
- 整合測試：Library Document 版本管理
- 整合測試：繼承鏈快取命中與失效
- 整合測試：繼承鏈 CTE 查詢效能 < 5ms（在合理資料量下）
- API 測試：所有記憶端點

**驗收標準：**
- [ ] 6 種 scope 的記憶 CRUD 完整
- [ ] 繼承鏈正確蒐集與合併
- [ ] Library Document 版本控制
- [ ] 快取機制運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-8.2 隱私引擎（Privacy Engine）

**範圍：** 實作 placeholder 系統，確保 AI 永遠不會接觸實際資料值。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/modules/ai/privacy.rs`：
   - `PrivacyEngine` 結構體
   - **資料遮罩（送入 AI 前）**：
     - `mask_conversation(messages) -> MaskedMessages`：將對話中的實際資料替換為 placeholder
   - **雙策略遮罩機制**（參考 `docs/system/05-privacy-engine.md`）：
     1. **結構化遮罩**（tool_execution / system 類型訊息）：依欄位 key 精確替換
     2. **自由文字遮罩**（member 類型訊息）：使用 `aho-corasick` crate（最長匹配優先、區分大小寫）
   - **遮罩字典建構**：從任務相關的 data_entries、profile_data、contacts 蒐集所有敏感值，排除空值與過短值（< 2 字元），建立 Aho-Corasick automaton
   - **字典快取**：使用 `moka` cache 快取 Aho-Corasick 自動機，key = `(task_id, account_id)`，TTL = 10 分鐘，資料變更時重建
     - `mask_data_schema(schema) -> MaskedSchema`：資料表只傳結構，不傳值
     - `mask_profile(profile_schema) -> MaskedProfile`：個人資料只傳欄位結構
   - **Placeholder 解析（AI 建議確認後）**：
     - `resolve_placeholders(text, context) -> ResolvedText`：將 `{{profile.phone}}`, `{{data.companyName}}` 等 placeholder 替換為實際值
     - 支援的 placeholder 類型（與 `docs/system/05-privacy-engine.md` 5.4 節一致）：
       - `{{profile.<key>}}` — 個人資料表欄位
       - `{{data.<key>}}` — 任務資料表欄位
3. **遮罩規則**：
   - 成員的 profile_data 值 → 遮罩，保留 profile_schema
   - DataEntry 的 values → 遮罩，保留 data_schema.fields
   - 對話中出現的電話、Email、地址等 → 識別並遮罩
4. **Placeholder 解析上下文**：
   - 從 task → data_entries → 取得 data 值
   - 從 actor → accounts.profile_data → 取得 profile 值
   - 從 task → contacts（透過 member_tag_assignments）→ 取得 contact 值

**涉及檔案：**
- `src/modules/ai/privacy.rs`
- `src/modules/ai/placeholder.rs`

**測試要求：**
- 單元測試：各種資料的遮罩規則
- 單元測試：Placeholder 解析（所有類型）
- 單元測試：遮罩後不含任何實際值
- 整合測試：完整流程（遮罩 → AI → placeholder → 解析 → 確認值正確）

**驗收標準：**
- [ ] AI 接收的上下文不含任何實際資料值
- [ ] Placeholder 語法全部支援
- [ ] 解析後值正確替換
- [ ] 遮罩覆蓋率完整（profile, data, contact）
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-8.3 AI 建議管線（Trigger → Suggestion → Decision）

**範圍：** 實作完整的 AI-Human 協作迴圈，含觸發偵測、上下文蒐集、LLM 呼叫、建議生命週期管理。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0017_ai_pipeline.sql`：
   - AI Suggestion 作為 `messages` 表中的一種 `source_type = 'ai_suggestion'` 記錄，SuggestionGroup 結構儲存在 `messages.content` JSONB 欄位中（遵循「對話即記錄」原則，與 `docs/data-model/09-task-conversation.md` 一致）
   - **建議查詢優化**：由於建議儲存在 JSONB 中，需為高效檢索「待處理建議」建立 GIN 索引或物化查詢視圖。建議在 `messages` 表新增 partial GIN 索引加速查詢：
     ```sql
     CREATE INDEX idx_messages_pending_suggestions ON messages
         USING GIN ((content->'suggestionGroup'->'suggestions'))
         WHERE source_type = 'ai_suggestion';
     ```
     當 `/tasks/{taskId}/suggestions` API 查詢效能不足時（建議閾值：單任務訊息量超過 1000 筆），建立 `suggestion_lookup` 物化索引表（suggestion_id, message_id, task_id, decision, created_at），在 SuggestionGroup 寫入與決策更新時同步維護，避免 JSONB 深層掃描
   - `ai_contexts` 表：id, task_id (FK), message_id (FK — 對應的 ai_suggestion message), prompt (TEXT), response (TEXT), model (VARCHAR), input_tokens (INT), output_tokens (INT), duration_ms (INT), created_at
   - SuggestionGroup content JSONB 結構（與 `docs/data-model/09-task-conversation.md` 4.1 節一致）：
     ```json
     {
       "suggestionGroup": {
         "id": "UUID",
         "trigger": "task_created|todo_completed|message_sent|tool_error|source_data_changed|manual_request",
         "suggestions": [{
           "id": "UUID",
           "summary": "string",
           "tool": "string",
           "parameters": {},
           "reasoning": "string",
           "contextUsed": [{ "scopeType": "string", "memoryId": "UUID", "content": "string" }],
           "decision": "pending|accept|modify_and_accept|reject|re_suggest",
           "decidedBy": "UUID?",
           "decidedAt": "datetime?",
           "modifiedParameters": {},
           "executionResult": {}
         }],
         "createdAt": "datetime"
       }
     }
     ```
3. 建立 `AiPipelineService`：
   - **觸發偵測**：訂閱 DomainEvent，偵測 6 種觸發：
     - `task_created` → 建議初始操作
     - `todo_completed` → 建議下一步
     - `message_sent` → 分析訊息並建議回應（涵蓋三種來源：member 直接發送、email_inbound Email 來信轉入、webhook 外部系統轉入，與 `docs/architecture.md` 一致）
     - `tool_error` → 建議錯誤修復
     - `source_data_changed` → 建議更新相關資料
     - `manual_request` → 使用者透過 `POST /api/v1/tasks/{taskId}/suggestions/request` 手動觸發建議（與 API Spec 一致）
   - **上下文蒐集**：
     - 記憶繼承鏈（B-8.1）
     - 遮罩後的對話歷史（最近 N 條）
     - 遮罩後的資料表結構
     - 可用工具列表（Phase 9）
   - **LLM 呼叫**：
     - 使用 Google Gemini API（`reqwest` HTTP client）
     - 構建 system prompt（含記憶、工具定義）
     - 構建 user prompt（含觸發上下文、對話歷史）
     - 使用 Gemini Context Caching 機制，建立 `GeminiCacheManager` 結構體（參考 `docs/system/04-ai-pipeline.md` 5.3）：
       - 使用 `moka::future::Cache<Uuid, String>` 管理 task_id → cached_content_name 對應
       - `get_or_create(task_id, system_prompt, memory_context, tool_definitions)` → 取得或建立 cached content
       - `invalidate(task_id)` → 任務上下文變更時清除快取
     - 快取失效策略：記憶變更、工具定義變更時清除對應 task 快取
     - 使用 Gemini `streamGenerateContent` 端點串流接收回應，降低首 token 延遲
     - 解析回應為 SuggestionGroup
   - **SuggestionGroup 生命週期**：
     - pending → 等待人類決策
     - 每個 suggestion 可獨立決策：accept / modify_and_accept / reject / re_suggest
     - re_suggest 時附帶人類指示重新呼叫 LLM
4. **事件佇列與持久化**（參考 `docs/system/04-ai-pipeline.md` 7.3 佇列持久化）：
   - 建立 `ai_pipeline_events` 表（在 migration `0017_ai_pipeline.sql` 中）：id, task_id (FK), trigger_type (VARCHAR), payload (JSONB), status (VARCHAR: pending/processing/completed/failed), attempts (INT), max_attempts (INT, DEFAULT 3), scheduled_at, started_at, completed_at, error_message, created_at
   - 觸發事件時先寫入 `ai_pipeline_events` 表，`PipelineWorker` 透過訂閱 `EventBus` 接收通知後喚醒處理（不使用 PostgreSQL `NOTIFY`/`LISTEN`，避免額外的 DB 連線與未來擴展限制）
   - Tokio worker 使用 `SELECT ... FOR UPDATE SKIP LOCKED` 取得待處理事件，避免多 worker 競爭
   - 重試策略：失敗事件使用指數退避重試（30s, 2m, 10m），超過 max_attempts 標記為 failed
   - 超時處理：processing 超過 5 分鐘的事件自動重置為 pending
   - 服務啟動時掃描 pending 和超時的 processing 事件，重新排入處理佇列
5. 建議以 ai_suggestion 類型的 message 加入對話時間線
6. 新增 DomainEvent：`SuggestionGenerated`, `SuggestionDecided`

**涉及檔案：**
- `migrations/0017_ai_pipeline.sql`
- `src/modules/ai/mod.rs`, `models.rs`, `repository.rs`
- `src/modules/ai/pipeline.rs`
- `src/modules/ai/trigger.rs`
- `src/modules/ai/context.rs`
- `src/modules/ai/llm_client.rs`
- `src/api/routes/ai_suggestions.rs`

**測試要求：**
- 單元測試：5 種觸發偵測
- 單元測試：上下文蒐集（記憶 + 對話 + schema）
- 單元測試：LLM 回應解析
- 整合測試：完整觸發 → 建議 → 決策流程
- 整合測試：re_suggest 流程
- API 測試：建議相關端點

**驗收標準：**
- [ ] 6 種觸發正確偵測
- [ ] 上下文蒐集含記憶繼承鏈
- [ ] LLM 呼叫與回應解析正確
- [ ] 建議生命週期管理完整
- [ ] 人類可 accept/modify/reject/re_suggest
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-8.4 AI 建議 API 端點

**範圍：** AI 建議相關的完整 REST API。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. API 路由：
   - `GET /api/v1/tasks/{taskId}/suggestions` — 取得任務的建議列表
   - `GET /api/v1/tasks/{taskId}/suggestions/{groupId}` — 取得建議詳情
   - `POST /api/v1/tasks/{taskId}/suggestions/{groupId}/suggestions/{suggestionId}/decide` — 對個別建議做出決策（與 `docs/api/paths/ai-suggestions.yaml` 一致）
     ```json
     {
       "decision": "accept | modify_and_accept | reject | re_suggest",
       "lastSeenMessageId": "...",
       "modifiedParameters": {},
       "additionalInstructions": "（re_suggest 時使用）"
     }
     ```
3. **lastSeenMessageId 驗證**：所有決策操作必須帶 lastSeenMessageId，確保使用者看到最新上下文（伺服器驗證：若 lastSeenMessageId 不等於對話中實際的最後一則訊息 ID，拒絕操作並回傳 409 Conflict，含最新訊息資訊）
4. **決策執行**：
   - accept → 呼叫 tools 模組執行工具（Phase 9 完善）
   - modify_and_accept → 先解析 placeholder → 確認 → 執行
   - reject → 記錄拒絕原因
   - re_suggest → 帶人類指示重新呼叫 LLM

**涉及檔案：**
- `src/api/routes/ai_suggestions.rs`
- `src/modules/ai/decision.rs`

**測試要求：**
- API 測試：建議列表與詳情
- API 測試：4 種決策類型
- API 測試：lastSeenMessageId 驗證
- 整合測試：決策 → 執行完整流程

**驗收標準：**
- [ ] 建議 API 完整
- [ ] 4 種決策正確處理
- [ ] lastSeenMessageId 保護機制
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-8.1 AI 建議 UI

**範圍：** 對話中的 AI 建議卡片、決策互動（accept/modify/reject/re-suggest）。

**說明：**
1. `SuggestionCard.vue` 元件：
   - 建議摘要顯示
   - 工具名稱與參數預覽
   - AI 推理說明
   - 使用的記憶/上下文
   - Accept / Modify / Reject 按鈕
   - Re-suggest 按鈕（附帶指示輸入框）
2. `SuggestionModifyDialog.vue`：
   - 修改工具參數的對話框
   - Placeholder 顯示（如 `{{profile.phone}}` 預覽實際值）
   - 確認修改後執行
3. 整合至 `ConversationPanel.vue`：
   - ai_suggestion 類型訊息渲染為 SuggestionCard
   - 執行結果顯示為 tool_execution 類型訊息
4. 建立 `useSuggestion` composable

**涉及檔案：**
- `frontend/src/components/ai/SuggestionCard.vue`
- `frontend/src/components/ai/SuggestionModifyDialog.vue`
- `frontend/src/composables/useSuggestion.ts`
- `frontend/src/components/conversation/ConversationPanel.vue`（擴展）

**測試要求：**
- 元件測試：SuggestionCard 渲染與互動
- 元件測試：SuggestionModifyDialog 參數編輯
- 單元測試：useSuggestion composable

**驗收標準：**
- [ ] AI 建議卡片正確顯示
- [ ] 4 種決策操作可執行
- [ ] 修改參數 UI 可使用
- [ ] Placeholder 預覽正確
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-8.2 Memory 管理 UI

**範圍：** 記憶檢視與編輯（各層級）、Library Document 管理。

**說明：**
1. `MemoryPanel.vue`（側邊面板）：
   - 按層級分組顯示記憶（Account / Organization / Project / Tag / Template / Task）
   - 記憶新增/編輯/刪除
   - Library Document 連結
2. `LibraryDocumentView.vue`：
   - 組織層級的知識庫列表
   - 文件 CRUD（Markdown 編輯器）
   - 版本歷史
3. 建立 `memoryStore`

**涉及檔案：**
- `frontend/src/components/memory/MemoryPanel.vue`
- `frontend/src/views/organizations/LibraryDocumentView.vue`
- `frontend/src/stores/memory.ts`

**測試要求：**
- 元件測試：MemoryPanel 分層顯示
- 元件測試：記憶 CRUD 互動
- 單元測試：memoryStore

**驗收標準：**
- [ ] 記憶按層級分組顯示
- [ ] 記憶 CRUD 可操作
- [ ] Library Document 管理完整
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 8 後，以下端到端流程可驗證：

1. **AI 觸發**：在任務中發送訊息 → AI 自動偵測觸發 → 產生建議
2. **隱私保護**：檢查 AI 上下文（ai_contexts 表）→ 不含任何實際資料值
3. **人類決策**：查看 AI 建議 → Accept/Modify/Reject → 執行結果記錄在對話中
4. **Re-suggest**：對建議不滿意 → 提供指示 → AI 重新產生建議
5. **記憶繼承**：設定組織記憶 → 建立任務 → AI 建議中可看到引用了組織記憶
6. **Placeholder**：AI 建議中含 `{{data.sponsorName}}` → 確認時顯示實際值 → 執行時使用實際值
