# Phase 4：任務系統（模板 → 任務 → 待辦 → 資料表）

**階段目標：** 實作完整的任務系統，從任務模板定義到任務實例化、待辦事項管理、結構化資料表蒐集，使團隊可以正式使用任務流程進行工作。

**前置依賴：** Phase 3 完成

---

## 後端任務

### B-4.1 TaskTemplate、TodoTemplate、DataSchema 資料表與 CRUD

**範圍：** 建立任務模板相關資料表，實作模板 CRUD 含待辦模板與資料表定義。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0009_task_templates.sql`：
   - `task_templates` 表：id, name, description, created_by (FK → accounts), created_at, updated_at, deleted_at
   - （task_templates 透過 `task_template_tags` 多對多關聯到 `member_tags`，再透過 `member_tags.project_id` 關聯到專案，因此不需要直接的 project_id 欄位）
   - `todo_templates` 表：id, task_template_id (FK), parent_id (FK, nullable, self-reference, 最多 1 層), name, description, sort_order (INT), created_at, updated_at, deleted_at
   - `data_schemas` 表：id, task_template_id (FK), name, fields (JSONB), created_at, updated_at, deleted_at
   - `task_template_tags` 表：id, task_template_id (FK), member_tag_id (FK), created_at
3. 建立 `TaskTemplateService`：
   - `create_template(actor, name, description)` → 建立模板（project 關聯透過 task_template_tags 間接建立）
   - `update_template(actor, template_id, updates)`
   - `delete_template(actor, template_id)` → 軟刪除
   - `add_todo_template(actor, template_id, name, sort_order, parent_id?)` → 新增待辦模板（1 層巢狀）
   - `update_todo_template(actor, todo_template_id, updates)`
   - `reorder_todo_templates(actor, template_id, ordered_ids)` → 重排序
   - `add_data_schema(actor, template_id, name, fields)` → 新增資料表定義
   - `update_data_schema(actor, schema_id, updates)`
   - `link_tag(actor, template_id, tag_id)` → 關聯標籤
   - `unlink_tag(actor, template_id, tag_id)` → 取消關聯
4. **data_schema fields JSONB** 結構驗證（10 種欄位類型）：
   - single_line_text, multi_line_text, number, date, email, url, select, boolean, image, file
   - 每欄位含：key, label, description, field_type, required, constraints
5. API 路由：
   - `GET /api/v1/projects/{projectId}/task-templates`
   - `POST /api/v1/projects/{projectId}/task-templates`
   - `GET /api/v1/projects/{projectId}/task-templates/{templateId}`
   - `PUT /api/v1/projects/{projectId}/task-templates/{templateId}`
   - `DELETE /api/v1/projects/{projectId}/task-templates/{templateId}`
   - 待辦模板與資料表定義的子路由

**涉及檔案：**
- `migrations/0009_task_templates.sql`
- `src/modules/core/task_template/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/task_templates.rs`

**測試要求：**
- 整合測試：TaskTemplate CRUD
- 整合測試：TodoTemplate 巢狀（驗證最多 1 層）
  - 驗證：若指定了 parent_id，該父項的 parent_id 必須為 NULL，否則回傳錯誤
- 整合測試：DataSchema fields JSONB 驗證（10 種類型）
- 整合測試：task_template_tags 關聯
- API 測試：所有模板端點

**驗收標準：**
- [ ] 任務模板 CRUD 完整
- [ ] 待辦模板支援 1 層巢狀
- [ ] 資料表定義支援 10 種欄位類型
- [ ] 模板與標籤多對多關聯
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-4.2 Task 資料表與任務實例化

**範圍：** 建立 tasks 資料表，實作從模板實例化任務的邏輯。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0010_tasks.sql`：
   - `tasks` 表：id, project_id (FK), task_template_id (FK), owner_tag_id (FK), name, description, status (ENUM: pending/in_progress/completed/cancelled), created_by (FK), created_at, updated_at, deleted_at
   - （注意：task status 為 pending/in_progress/completed/cancelled）
3. 建立 `TaskService`：
   - `create_task(actor, template_id, owner_tag_id, name?)` → 從模板實例化：
     - 驗證 owner_tag_id 是此模板關聯的標籤之一（透過 task_template_tags）
     - 若建立者不屬於 owner_tag，需額外檢查 owner_tag 的 `externalTaskCreation` 設定是否允許建立者的標籤建立此模板的任務（跨組協作機制）
     - 自動從 todo_templates 建立對應的 todos
     - 自動從 data_schemas 建立對應的 data_entries
   - `update_task(actor, task_id, updates)` → 更新任務
   - `update_task_status(actor, task_id, new_status)` → 狀態轉換（含 completed 前驗證所有 todos 已完成）
   - `list_tasks(project_id, filters)` → 列出任務（支援按狀態、標籤過濾 + 游標分頁）
   - `get_task_with_participants(task_id)` → 取得任務含參與人計算
4. **參與人即時計算邏輯**（透過各模組 Service 層聚合，禁止跨模組 SQL JOIN）：
   - ownerTag 成員 ∪ @mentioned 成員 ∪ 建立者 ∪ todo_assignees
   - 分別透過 MemberTagService、ConversationService、TodoService 查詢後在應用層合併
   - **效能優化**：使用 `moka` cache 建立 `ParticipantCache`，key = `task_id`，TTL 1–2 分鐘。任何影響參與人組成的寫入操作（任務指派變更、訊息 @mention、todo assignee 變更）時主動 invalidate 對應 cache entry
5. 新增 DomainEvent：`TaskCreated`, `TaskStatusChanged`

**涉及檔案：**
- `migrations/0010_tasks.sql`
- `src/modules/core/task/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/tasks.rs`

**測試要求：**
- 整合測試：從模板實例化任務（自動建立 todos + data_entries）
- 整合測試：owner_tag_id 驗證（必須是關聯標籤）
- 整合測試：狀態轉換規則（含 completed 需所有 todos 完成）
- 整合測試：參與人計算
- API 測試：所有任務端點

**驗收標準：**
- [ ] 任務從模板正確實例化
- [ ] Todos 和 DataEntries 自動建立
- [ ] 狀態轉換規則正確
- [ ] 參與人計算正確
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-4.3 Todo 資料表與待辦事項管理

**範圍：** 建立 todos、todo_assignees 資料表，實作待辦 CRUD、指派、狀態管理。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0011_todos.sql`：
   - `todos` 表：id, task_id (FK), parent_id (FK, nullable), title, description, status (ENUM: open/completed), todo_type (ENUM: template/ad_hoc), source_template_id (UUID, FK → todo_templates, nullable), due_date (TIMESTAMPTZ, nullable), sort_order (INT), linked_task_id (FK, nullable), completed_at (TIMESTAMPTZ, nullable), completed_by (UUID, FK → accounts, nullable), created_at, updated_at, deleted_at
   - （與 `docs/data-model/10-todo.md` 一致，使用 `open/completed` 而非 `pending/in_progress/completed`）
   - 需在 migration 中明確建立 ENUM 類型：`CREATE TYPE todo_type AS ENUM ('template', 'ad_hoc'); CREATE TYPE todo_status AS ENUM ('open', 'completed');`
   - `todo_assignees` 表：id, todo_id (FK), member_id (FK), created_at
3. 建立 `TodoService`：
   - `create_ad_hoc_todo(actor, task_id, title, parent_id?)` → 建立臨時待辦
   - `update_todo(actor, todo_id, updates)` → 更新標題、描述、due_date
   - `update_todo_status(actor, todo_id, new_status)` → 更新狀態
   - `assign_todo(actor, todo_id, member_id)` → 指派成員
   - `unassign_todo(actor, todo_id, member_id)` → 取消指派
   - `link_task(actor, todo_id, linked_task_id)` → 關聯其他任務（跨組協作）
   - `list_my_todos(account_id, filters)` → 列出我的所有待辦（跨專案聚合）
     - 對應 API 端點：`GET /api/v1/accounts/me/todos`（跨專案個人待辦聚合）
4. 待辦完成時自動觸發 DomainEvent：`TodoCompleted`
5. **CRDT 操作支援**（與 `docs/system/03-crdt-implementation.md` 一致）：
   - 建立 `crdt_operations` 表（在 `0011_todos.sql` 中一併建立）：id, entity_type (VARCHAR), entity_id (UUID), operation (BYTEA), created_by (FK → accounts), created_at
   - todos 使用 Y.Map，每個欄位獨立更新（LWW Register）
   - 每次操作寫入 `crdt_operations` 表（entity_type = 'todo'）
   - CRDT 管理欄位：title, description, status, due_date, sort_order
   - 本 Phase 建立 todos 的 CRDT 寫入邏輯，完整的即時同步整合於 Phase 5

**涉及檔案：**
- `migrations/0011_todos.sql`
- `src/modules/core/todo/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/todos.rs`

**測試要求：**
- 整合測試：Todo CRUD（template + ad_hoc）
- 整合測試：1 層巢狀
- 整合測試：指派與取消
- 整合測試：linked_task 關聯
- 整合測試：my_todos 跨專案聚合
- API 測試：所有待辦端點

**驗收標準：**
- [ ] 待辦 CRUD 完整
- [ ] 指派管理正確
- [ ] 跨組 linked_task 關聯可運作
- [ ] 個人待辦跨專案聚合
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-4.4 DataSheet / DataEntry 資料表與結構化資料蒐集

**範圍：** 建立 data_entries 資料表，實作資料列 CRUD、欄位驗證、跨任務聚合查詢。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0012_data_entries.sql`：
   - `data_entries` 表：id, data_schema_id (FK), task_id (FK), values (JSONB), source_links (JSONB, nullable), created_at, updated_at, deleted_at
3. 建立 `DataSheetService`：
   - `upsert_data_entry(actor, task_id, schema_id, values)` → 新增或更新資料列（自動根據 data_schema 驗證欄位類型與約束）
   - `get_data_entry(task_id, schema_id)` → 取得任務的資料列
   - `get_aggregated_entries(template_id, schema_id, filters)` → 跨任務聚合查詢（同一模板下所有任務的資料）
   - `share_data_to_task(actor, source_task_id, target_task_id, field_mappings)` → 跨任務資料分享
4. **欄位驗證邏輯**：根據 data_schema 中的 field_type 與 constraints 驗證 values
5. 外部 API（Phase 11 完善）：`GET /external/v1/projects/{projectId}/task-templates/{templateId}/data` 供外部系統查詢
   > **注意：** Phase 4 定義路由骨架，Phase 11 完善實作
6. **CRDT 操作支援**（與 `docs/system/03-crdt-implementation.md` 一致）：
   - data_entries.values 使用 Y.Map，鍵值結構，欄位獨立合併
   - 每次操作寫入 `crdt_operations` 表（entity_type = 'data_entry'）
   - 本 Phase 建立 data_entries 的 CRDT 寫入邏輯，完整的即時同步整合於 Phase 5

**涉及檔案：**
- `migrations/0012_data_entries.sql`
- `src/modules/core/data_sheet/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/data_external.rs`

**測試要求：**
- 整合測試：DataEntry CRUD
- 整合測試：欄位驗證（10 種類型 × 有效/無效輸入）
  - 驗證項目：key 存在於 DataSchema fields、value 型別匹配 field type、required 欄位在完成時必須有值、constraints（maxLength, min, max 等）滿足
- 整合測試：跨任務聚合查詢
- 整合測試：share_data_to_task
- API 測試：資料表端點

**驗收標準：**
- [ ] 資料列 CRUD 與驗證完整
- [ ] 10 種欄位類型全部支援
- [ ] 跨任務聚合查詢可運作
- [ ] 跨任務資料分享可運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-4.1 任務模板管理頁面

**範圍：** 任務模板列表、建立/編輯模板、待辦模板管理、資料表欄位定義器。

**說明：**
1. `TaskTemplateListView.vue`：
   - 專案內所有任務模板列表
   - 建立模板按鈕
2. `TaskTemplateEditorView.vue`：
   - 模板基本資訊編輯
   - 待辦模板管理（拖拽排序、巢狀）
   - 資料表欄位定義器（支援 10 種欄位類型、約束設定）
   - 關聯標籤設定
3. `DataSchemaEditor.vue` 元件：
   - 欄位新增/編輯/刪除
   - 欄位類型選擇器（含約束設定 UI）
   - 欄位預覽
4. 建立 `taskTemplateStore`

**涉及檔案：**
- `frontend/src/views/projects/TaskTemplateListView.vue`
- `frontend/src/views/projects/TaskTemplateEditorView.vue`
- `frontend/src/components/data-schema/DataSchemaEditor.vue`
- `frontend/src/stores/taskTemplate.ts`

**測試要求：**
- 元件測試：模板編輯器
- 元件測試：DataSchemaEditor（10 種欄位類型）
- 單元測試：taskTemplateStore

**驗收標準：**
- [ ] 模板 CRUD UI 完整
- [ ] 資料表欄位定義器支援所有欄位類型
- [ ] 待辦模板拖拽排序
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-4.2 任務列表與詳情頁面

**範圍：** 任務列表（過濾、排序）、建立任務、任務詳情（待辦、資料表）。

**說明：**
1. `TaskListView.vue`：
   - 專案內任務列表（按狀態、標籤過濾）
   - 建立任務（選擇模板 + ownerTag）
   - 任務狀態標示
2. `TaskDetailView.vue`：
   - 任務資訊（名稱、狀態、參與人）
   - 待辦事項列表（勾選完成、指派成員、新增臨時待辦）
   - 資料表填寫表單（根據 data_schema 動態生成）
   - 狀態變更按鈕
3. `TodoList.vue` 元件：
   - 待辦列表（含巢狀顯示）
   - 勾選/指派/編輯互動
4. `DataEntryForm.vue` 元件：
   - 動態表單（根據欄位類型渲染不同輸入元件）
5. 建立 `taskStore`, `todoStore`

**涉及檔案：**
- `frontend/src/views/projects/TaskListView.vue`
- `frontend/src/views/projects/TaskDetailView.vue`
- `frontend/src/components/task/TodoList.vue`
- `frontend/src/components/data-schema/DataEntryForm.vue`
- `frontend/src/stores/task.ts`, `frontend/src/stores/todo.ts`

**測試要求：**
- 元件測試：任務列表過濾
- 元件測試：TodoList 互動（勾選、指派）
- 元件測試：DataEntryForm 動態渲染
- 單元測試：taskStore, todoStore

**驗收標準：**
- [ ] 任務列表含過濾排序
- [ ] 從模板建立任務流程完整
- [ ] 待辦事項互動完整
- [ ] 資料表填寫表單動態生成
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-4.3 個人待辦總覽

**範圍：** 跨專案個人待辦事項聚合檢視。

**說明：**
1. `MyTodosView.vue`：
   - 跨專案待辦列表
   - 按狀態、截止日期過濾與排序
   - 點擊跳轉至對應任務
2. 建立 `myTodosStore`

**涉及檔案：**
- `frontend/src/views/MyTodosView.vue`
- `frontend/src/stores/myTodos.ts`

**測試要求：**
- 元件測試：待辦列表渲染
- 單元測試：跨專案聚合邏輯

**驗收標準：**
- [ ] 個人待辦跨專案聚合顯示
- [ ] 過濾與排序可操作
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 4 後，以下端到端流程可驗證：

1. **模板管理**：建立任務模板 → 新增待辦模板 → 定義資料表欄位 → 關聯標籤
2. **任務建立**：選擇模板 + 標籤 → 自動建立待辦與資料欄位
3. **任務操作**：勾選待辦 → 填寫資料表 → 指派成員 → 變更狀態
4. **跨組協作**：A 組待辦關聯 B 組任務（linkedTask）
5. **個人待辦**：在首頁看到所有專案中指派給我的待辦
