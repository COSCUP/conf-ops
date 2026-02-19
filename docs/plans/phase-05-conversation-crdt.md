# Phase 5：對話系統與 CRDT 即時協作

**階段目標：** 實作任務對話系統（訊息 CRUD）、基於 yrs (Yjs) 的 CRDT 即時同步、WebSocket 連線管理與 awareness protocol，使多人可同時在任務對話中即時協作。

**前置依賴：** Phase 4 完成

---

## 後端任務

### B-5.1 Message 資料表與對話 CRUD

**範圍：** 建立 messages、conversation_states、last_seen_positions 資料表，實作對話訊息 CRUD。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0013_conversations.sql`：
   - `messages` 表：id, task_id (FK), source_type (ENUM: member/ai_suggestion/tool_execution/system/email_inbound), source_id (UUID, nullable — 依 source_type 指向 members.id 或 contacts.id，**禁止加設資料庫層級 FOREIGN KEY 約束**，因多型態關聯無法指向單一表，改由應用層 ConversationService 保證參照完整性), content (JSONB), attachments (JSONB, nullable), action_result (JSONB, nullable), last_seen_message_id (UUID, nullable — 寫入操作時客戶端附帶的已讀訊息 ID), created_at
   - `conversation_states` 表（與 `docs/data-model/09-task-conversation.md` 2.2 節一致）：id, task_id (FK), account_id (FK), last_read_message_id (UUID, nullable), unread_count (INTEGER, DEFAULT 0), created_at, updated_at — UNIQUE(task_id, account_id)。追蹤每位使用者在每個任務對話中的已讀狀態
   - `last_seen_positions` 表（與 `docs/data-model/09-task-conversation.md` 2.3 節一致）：id, task_id (FK), account_id (FK), y_clock (BIGINT), updated_at — UNIQUE(task_id, account_id)。追蹤 CRDT 的 Yjs clock 位置
   - `crdt_operations` 表：已在 Phase 4 migration 中建立（id, entity_type, entity_id, operation, created_by, created_at），本 Phase 驗證該表已存在並可正常使用
3. 建立 `ConversationService`（trait + impl）：
   - `send_message(actor_member_id, task_id, content, attachments?) -> Message` — 發送成員訊息
   - `add_system_message(task_id, content) -> Message` — 新增系統訊息
   - `get_conversation(task_id, cursor, limit) -> PaginatedMessages` — 游標分頁查詢對話
   - `update_last_seen(member_id, task_id, message_id)` — 更新已讀位置
   - `get_last_seen(member_id, task_id) -> Option<MessageId>` — 取得已讀位置
4. **lastSeenMessageId 保護機制**：
   - 寫入操作（send_message、apply_suggestion）時驗證 actor 的 lastSeenMessageId 是否為最新
   - 若落後則回傳錯誤（回傳 409 Conflict，含 latest_message_id 供前端更新已讀位置），要求先更新已讀位置
5. **content JSONB 結構**（source_type = 'member' 時）：`{ "text": "string", "mentions": [{ "type": "member | tag", "id": "UUID" }] }`（與 `docs/data-model/09-task-conversation.md` 一致，不含程式碼區塊）
6. **多型態外鍵驗證**：`messages.source_id` 寫入時，`ConversationService` 必須根據 `source_type` 呼叫對應模組 Service 驗證 ID 存在性（如 `source_type = 'member'` → `MemberService.exists(source_id)`、`source_type = 'email_inbound'` → `EmailService.exists(source_id)`）
7. 新增 DomainEvent：`MessageSent`

**涉及檔案：**
- `migrations/0013_conversations.sql`
- `src/modules/conversation/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/conversations.rs`

**測試要求：**
- 整合測試：Message CRUD（各 source_type）
- 整合測試：游標分頁正確
- 整合測試：lastSeenMessageId 保護（落後時拒絕寫入）
- 整合測試：last_seen_positions 追蹤
- API 測試：所有對話端點

**驗收標準：**
- [ ] 訊息 CRUD 完整
- [ ] 5 種 source_type 正確處理（member, ai_suggestion, tool_execution, system, email_inbound）
- [ ] lastSeenMessageId 保護機制正確
- [ ] 游標分頁可運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-5.2 CRDT 同步引擎（yrs 整合）

**範圍：** 整合 yrs (Yjs Rust port)，建立 CRDT document 管理、操作合併、狀態同步。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/modules/conversation/crdt.rs`：
   - `CrdtManager` 結構體：管理每個 task 的 yrs Document
   - `apply_update(task_id, update: Vec<u8>) -> Vec<u8>` — 應用 CRDT 更新，返回合併後的狀態
   - `get_state_vector(task_id) -> Vec<u8>` — 取得狀態向量
   - `encode_state_as_update(task_id, state_vector) -> Vec<u8>` — 編碼差異更新
3. **CRDT 持久化**：
   - CRDT document 狀態儲存於 `crdt_operations` 表（操作日誌，真相來源）
   - 每次 CRDT 更新寫入 `crdt_operations`（entity_type = 'conversation', entity_id = task_id）
   - 從 DB 重建 document 時，依序重放 `crdt_operations` 中的所有操作
   - 定期 compaction：將多筆操作合併為一筆快照操作，減少重建時間
   - 注意：`conversation_states` 表用於**使用者已讀狀態追蹤**（非 CRDT 狀態儲存），與 CRDT 持久化是不同職責
4. **記憶體中 Document 管理**：
   - 使用 `Arc<RwLock<HashMap<TaskId, YDoc>>>` 快取活躍的 CRDT document
   - 閒置超時（如 5 分鐘無更新）從記憶體卸載，下次存取時從 DB 重建
5. Messages 作為 CRDT document 中的 Y.Array 元素
6. **整合 Phase 4 的 CRDT 實體**（與 `docs/system/03-crdt-implementation.md` 一致）：
   - Phase 4 已建立 todos (Y.Map) 和 data_entries (Y.Map) 的 CRDT 寫入邏輯
   - 本 Phase 將其整合至 WebSocket 即時同步，使多人可同時編輯 todos 欄位與 data_entries 欄位
   - 記憶 (memories / library_documents) 使用 Y.Text，字元級別即時協同編輯，於 Phase 8 建立 CRDT 寫入，本 Phase 提供同步基礎設施

**涉及檔案：**
- `src/modules/conversation/crdt.rs`
- `src/modules/conversation/service.rs`（擴展）

**測試要求：**
- 單元測試：CRDT 更新合併（模擬兩個客戶端並發）
- 單元測試：狀態向量與差異編碼
- 整合測試：雙寫模式（DB 狀態與操作日誌一致）
- 整合測試：記憶體卸載後從 DB 重建

**驗收標準：**
- [ ] yrs 整合可正常運作
- [ ] CRDT 並發更新合併正確
- [ ] 雙寫模式一致
- [ ] 記憶體管理（快取 + 卸載 + 重建）
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-5.3 WebSocket 連線管理與 Awareness Protocol

**範圍：** 建立 WebSocket 端點、連線生命週期管理、awareness protocol（上線/離線/游標位置）。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/api/routes/ws.rs`：
   - `POST /api/v1/tasks/{taskId}/conversation/ws-token` → 產生一次性 WebSocket token（30 秒有效，使用後即失效）
   - `GET /api/v1/tasks/{taskId}/conversation/ws?token={token}` → WebSocket Upgrade（認證透過 query parameter 傳遞一次性 token，不使用 Authorization header，與 `docs/api/paths/conversations.yaml` 一致）
3. 建立 `src/modules/conversation/ws_manager.rs`：
   - `WsManager` 結構體：管理所有 WebSocket 連線
   - `ConnectionRegistry`：`HashMap<TaskId, Vec<(MemberId, WsSender)>>`
   - 處理 WebSocket 訊息類型（與 `docs/system/03-crdt-implementation.md` 一致）：
     - `SyncStep1 { state_vector }` — 同步步驟 1
     - `SyncStep2 { update }` — 同步步驟 2（差異更新）
     - `AwarenessUpdate { state }` — awareness 狀態更新
     - `WriteOperation { update, last_seen_message_id }` — 寫入操作（含已讀位置驗證）
4. **Awareness Protocol**：
   - 每個連線的 awareness state：member_id, cursor_position, is_typing, last_active
   - 廣播 awareness 變更至同一 task 的所有連線
5. **心跳機制**：每 30 秒 ping/pong，超時斷開
6. **環境變數配置**（與 `docs/system/03-crdt-implementation.md` 一致）：
   - `CRDT_WS_MAX_CONNECTIONS_PER_TASK`：每個任務最大 WebSocket 連線數（預設 50）
   - `CRDT_WS_HEARTBEAT_INTERVAL_SECS`：WebSocket 心跳間隔（預設 30 秒）
   - `CRDT_WS_IDLE_TIMEOUT_SECS`：WebSocket 閒置斷線時間（預設 300 秒）

**涉及檔案：**
- `src/api/routes/ws.rs`
- `src/modules/conversation/ws_manager.rs`
- `src/modules/conversation/awareness.rs`

**測試要求：**
- 整合測試：WebSocket 連線建立與認證
- 整合測試：訂閱/取消訂閱
- 整合測試：CRDT 同步（兩個客戶端模擬）
- 整合測試：awareness 廣播
- 整合測試：心跳超時斷開

**驗收標準：**
- [ ] WebSocket 連線可建立與認證
- [ ] CRDT 同步在多個客戶端間正確運作
- [ ] Awareness 即時廣播
- [ ] 心跳與超時機制正確
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-5.1 對話 UI 與即時同步

**範圍：** 任務對話時間線 UI、WebSocket 即時同步、訊息發送、@mention。

**說明：**
1. `ConversationPanel.vue` 元件（嵌入 TaskDetailView）：
   - 訊息時間線（支援多種 source_type 的不同樣式）
   - 訊息輸入框（支援文字、@mention 自動完成）
   - 即時接收新訊息
   - 未讀標記（基於 lastSeenMessageId）
   - 自動捲動至最新訊息
2. 建立 `useWebSocket` composable：
   - WebSocket 連線管理（自動重連、指數退避）
     - 重連策略：最大重連次數 10 次、初始延遲 1 秒、指數增長係數 2、最大延遲 30 秒（參考 `docs/system/03-crdt-implementation.md` 錯誤處理策略）
   - 訂閱/取消訂閱任務
   - CRDT 同步更新收發
   - Awareness 狀態管理
3. 建立 `useConversation` composable：
   - 訊息載入（分頁）
   - 訊息發送
   - lastSeen 更新
4. 建立 `conversationStore`

**涉及檔案：**
- `frontend/src/components/conversation/ConversationPanel.vue`
- `frontend/src/components/conversation/MessageItem.vue`
- `frontend/src/components/conversation/MessageInput.vue`
- `frontend/src/composables/useWebSocket.ts`
- `frontend/src/composables/useConversation.ts`
- `frontend/src/stores/conversation.ts`

**測試要求：**
- 元件測試：ConversationPanel 訊息渲染
- 元件測試：MessageInput @mention
- 單元測試：useWebSocket 重連邏輯
- 單元測試：conversationStore

**驗收標準：**
- [ ] 對話訊息即時顯示
- [ ] @mention 自動完成
- [ ] WebSocket 自動重連
- [ ] 未讀標記正確
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-5.2 Awareness UI（即時協作者狀態）

**範圍：** 顯示誰正在查看任務、正在輸入、游標位置等即時狀態。

**說明：**
1. `AwarenessBar.vue` 元件：
   - 顯示當前在線的協作者頭像
   - 「正在輸入」指示器
2. 在 `MessageInput.vue` 中發送 typing awareness 狀態
3. 整合至 `TaskDetailView.vue`

**涉及檔案：**
- `frontend/src/components/conversation/AwarenessBar.vue`
- `frontend/src/components/conversation/MessageInput.vue`（擴展）
- `frontend/src/views/projects/TaskDetailView.vue`（擴展）

**測試要求：**
- 元件測試：AwarenessBar 渲染
- 元件測試：typing 狀態發送

**驗收標準：**
- [ ] 即時顯示在線協作者
- [ ] 「正在輸入」指示正確
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 5 後，以下端到端流程可驗證：

1. **對話發送**：在任務詳情中輸入文字 → @mention 成員 → 發送訊息 → 對話時間線即時更新
2. **即時同步**：兩個瀏覽器分頁同時開啟同一任務 → 一方發送訊息 → 另一方即時看到
3. **Awareness**：兩人同時在任務中 → 看到彼此的頭像 → 一方輸入時另一方看到「正在輸入」
4. **未讀追蹤**：離開任務 → 其他人發送訊息 → 回來時顯示未讀標記
5. **CRDT 衝突解決**：兩人同時編輯 → 內容自動合併，無衝突
