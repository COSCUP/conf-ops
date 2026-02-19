# Phase 2：組織與專案管理

**階段目標：** 實作組織與專案的完整 CRUD，含組織角色管理（org_owner / org_admin / org_member）、組織成員邀請、專案建立（空白 + 複製），建立基礎 RBAC 權限框架。

**前置依賴：** Phase 1 完成

---

## 後端任務

### B-2.1 Organization 與 OrganizationMember 資料表與 CRUD

**範圍：** 建立 organizations、organization_members 資料表、Repository、Service 層。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0003_organizations.sql`：
   - `organizations` 表：id, name, description, logo_url (VARCHAR, nullable), created_by (FK → accounts), created_at, updated_at, deleted_at
   - `organization_members` 表：id, organization_id (FK), account_id (FK), org_role (ENUM: org_owner/org_admin/org_member), created_at, updated_at
   - 成員退出時直接刪除記錄（非軟刪除）
   - UNIQUE 約束：`(organization_id, account_id)` 確保不重複加入
3. 建立 Rust 結構體與 Repository CRUD
4. 建立 `OrganizationService`（trait + impl）：
   - `create_organization(actor, name, description)` → 建立者自動成為 org_owner
   - `update_organization(actor, org_id, updates)`
   - `list_my_organizations(account_id)` → 列出使用者所屬的組織
   - `invite_member(actor, org_id, email, role)` → 邀請成員（已有帳號直接加入，無帳號寄邀請信）
   - `remove_member(actor, org_id, member_account_id)`
   - `update_member_role(actor, org_id, member_account_id, new_role)`
5. API 路由：
   - `POST /api/v1/organizations`
   - `GET /api/v1/organizations`（我的組織列表）
   - `GET /api/v1/organizations/{orgId}`
   - `PUT /api/v1/organizations/{orgId}`（與 `docs/api/paths/organizations.yaml` 一致，完整替換語義）
   - `DELETE /api/v1/organizations/{orgId}`（軟刪除，僅 org_owner 可操作，需確認無進行中專案）
   - `GET /api/v1/organizations/{orgId}/members`
   - `POST /api/v1/organizations/{orgId}/members/invite`
   - `DELETE /api/v1/organizations/{orgId}/members/{memberId}`（與 `docs/api/paths/organizations.yaml` 一致，使用 memberId 而非 accountId）
   - `PUT /api/v1/organizations/{orgId}/members/{memberId}`（與 API Spec 一致）
6. 新增 DomainEvent：`OrganizationCreated`, `MemberInvited`, `MemberJoined`

**涉及檔案：**
- `migrations/0003_organizations.sql`
- `src/modules/core/organization/mod.rs`, `models.rs`, `repository.rs`, `service.rs`, `error.rs`, `tests.rs`
- `src/api/routes/organizations.rs`
- `src/events.rs`（擴展）

**測試要求：**
- 整合測試：Organization CRUD
- 整合測試：成員邀請、角色變更、移除
- API 測試：所有組織端點
- 整合測試：非 org_owner 無法邀請成員

**驗收標準：**
- [ ] 組織 CRUD 完整
- [ ] 組織成員管理（邀請、角色、移除）正確
- [ ] 組織角色權限基礎驗證
- [ ] 組織刪除時若仍有進行中的專案（status != archived），回傳 409 Conflict（與 `docs/api/paths/organizations.yaml` 一致）
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-2.2 Project 資料表與 CRUD（含空白建立與複製）

**範圍：** 建立 projects 資料表、CRUD、專案複製邏輯。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0004_projects.sql`：
   - `projects` 表：id, organization_id (FK), name, description, status (ENUM: preparing/active/completed/archived), source_project_id (FK, nullable), permission_settings (JSONB, DEFAULT '{}'), created_by (FK → accounts), created_at, updated_at, deleted_at
3. 建立 `ProjectService`：
   - `create_project(actor, org_id, name, description)` → 建立空白專案
   - `copy_project(actor, org_id, source_project_id, new_name)` → 複製專案基礎邏輯（Phase 2 僅複製專案基本資料並記錄 source_project_id，設定狀態為 preparing，建立者成為 owner；member_tags, task_templates, memories, tool_configs 等深複製邏輯於 Phase 12 完善）
     > **階段性實作範圍說明**：本 Phase 實作基礎複製（專案設定、任務範本），完整資源複製（成員、標籤、工具設定等）依 API 契約定義於後續 Phase 逐步擴展
   - `update_project(actor, project_id, updates)`
   - `update_project_status(actor, project_id, new_status)` → 狀態轉換驗證
   - `list_projects(actor, org_id)` → 列出組織內的專案
4. API 路由：
   - `POST /api/v1/organizations/{orgId}/projects`
   - `POST /api/v1/organizations/{orgId}/projects/copy`
   - `GET /api/v1/organizations/{orgId}/projects`
   - `GET /api/v1/projects/{projectId}`
   - `PUT /api/v1/projects/{projectId}`
   - `DELETE /api/v1/projects/{projectId}`（軟刪除，僅 project owner 可操作）
   - `PUT /api/v1/projects/{projectId}/status`
   - `GET /api/v1/projects/{projectId}/permission-settings`（取得專案權限設定，僅 owner）
   - `PUT /api/v1/projects/{projectId}/permission-settings`（更新專案權限設定，完整取代，僅 owner）
5. 專案狀態轉換規則：
      ```
      preparing → active → completed → archived
                       ↘              ↗
                         archived
      ```
      - `active → archived`（提前封存，如活動取消）為合法轉換
      - `preparing` 不可直接跳到 `completed`
      - `archived` 為終態，不可再變更
6. 新增 DomainEvent：`ProjectCreated`, `ProjectCopied`, `ProjectStatusChanged`

**涉及檔案：**
- `migrations/0004_projects.sql`
- `src/modules/core/project/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/api/routes/projects.rs`
- `src/events.rs`（擴展）

**測試要求：**
- 整合測試：Project CRUD
- 整合測試：專案狀態轉換（含非法轉換拒絕）
- 整合測試：專案複製（基礎邏輯）
- API 測試：所有專案端點

**驗收標準：**
- [ ] 專案 CRUD 完整
- [ ] 專案狀態轉換規則正確
- [ ] 專案複製基礎邏輯可運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-2.3 基礎 RBAC 權限框架

**範圍：** 建立 `check_permission` 權限檢查基礎架構，先支援組織角色檢查，後續階段逐步擴展。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/modules/core/permission/mod.rs`：
   - `Action` 枚舉：`CreateOrganization`, `UpdateOrganization`, `InviteMember`, `CreateProject`, `UpdateProject` 等
   - `Resource` 枚舉：Organization, Project 等
   - `check_permission(account_id: Uuid, project_id: Uuid, action: Action) -> Result<PermissionResult>`（PermissionResult 含 allowed、reason、denied_by_tag，提供豐富的錯誤資訊）
3. 初步實作組織層級權限：
   - org_owner：完整權限
   - org_admin：可管理專案、邀請成員
   - org_member：唯讀
4. 建立 Axum permission middleware / extractor：在 handler 中方便檢查權限
5. 權限快取延後至 Phase 3 再實作（Phase 2 先建立基礎權限檢查邏輯），屆時使用 moka in-memory cache + PostgreSQL LISTEN/NOTIFY 失效機制

**涉及檔案：**
- `src/modules/core/permission/mod.rs`, `permission/service.rs`
- `src/api/middleware/permission.rs`
- `src/api/extractors/permission.rs`

**測試要求：**
- 單元測試：各角色的權限矩陣
- 整合測試：org_member 無法建立專案
- 整合測試：org_owner 可邀請成員
- API 測試：無權限時回傳 403

**驗收標準：**
- [ ] 權限檢查基礎架構就緒
- [ ] 組織層級角色權限正確
- [ ] 權限檢查基礎架構就緒（Phase 3 再加入快取）
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-2.1 組織管理頁面

**範圍：** 組織列表、建立組織、組織設定、成員管理。

**說明：**
1. `OrganizationListView.vue`：
   - 我的組織列表
   - 建立組織按鈕 → 彈窗表單
2. `OrganizationSettingsView.vue`：
   - 組織資料編輯（name, description, logo）
   - 成員列表（含角色標示）
   - 邀請成員（email + 角色選擇）
   - 成員角色變更 / 移除
3. 建立 `organizationStore`（Pinia）
4. 建立 `useOrganization` composable

**涉及檔案：**
- `frontend/src/views/organizations/OrganizationListView.vue`
- `frontend/src/views/organizations/OrganizationSettingsView.vue`
- `frontend/src/stores/organization.ts`
- `frontend/src/composables/useOrganization.ts`

**測試要求：**
- 元件測試：組織列表渲染
- 元件測試：邀請成員表單
- 單元測試：organizationStore

**驗收標準：**
- [ ] 組織 CRUD 頁面完整
- [ ] 成員邀請與管理可操作
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-2.2 專案管理頁面

**範圍：** 專案列表、建立專案（空白 + 複製）、專案設定。

**說明：**
1. `ProjectListView.vue`：
   - 組織內專案列表（含狀態標示）
   - 建立專案按鈕 → 選擇空白或從現有專案複製
2. `ProjectSettingsView.vue`：
   - 專案資料編輯
   - 狀態變更按鈕
3. `ProjectDashboardView.vue`（骨架）：
   - 專案概覽頁面（後續階段填入內容）
4. 建立 `projectStore`（Pinia）

**涉及檔案：**
- `frontend/src/views/projects/ProjectListView.vue`
- `frontend/src/views/projects/ProjectSettingsView.vue`
- `frontend/src/views/projects/ProjectDashboardView.vue`
- `frontend/src/stores/project.ts`

**測試要求：**
- 元件測試：專案列表渲染
- 元件測試：建立專案表單（含複製選項）
- 單元測試：projectStore

**驗收標準：**
- [ ] 專案 CRUD 頁面完整
- [ ] 從現有專案複製的 UI 流程可操作
- [ ] 專案狀態變更可操作
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 2 後，以下端到端流程可驗證：

1. **組織管理**：建立組織 → 邀請成員 → 成員接受邀請 → 角色管理
2. **專案管理**：在組織內建立專案 → 編輯專案資料 → 變更專案狀態
3. **專案複製**：從現有專案複製（基礎邏輯，Phase 3+ 完善複製內容）
4. **權限控制**：org_member 無法建立專案或邀請成員
