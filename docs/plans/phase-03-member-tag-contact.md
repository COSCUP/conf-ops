# Phase 3：成員、聯絡人與標籤

**階段目標：** 實作專案成員管理、組織層級聯絡人管理、成員標籤系統，完成完整的 deny-first RBAC 權限模型，使跨組協作的基礎結構就緒。

**前置依賴：** Phase 2 完成

---

## 後端任務

### B-3.1 Member 資料表與專案成員管理

**範圍：** 建立 members 資料表、專案成員 CRUD、角色管理。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0008_members.sql`：
   - `members` 表：id, account_id (FK), project_id (FK), role (member_role ENUM: owner/tag_admin/member), created_at, updated_at, deleted_at
   - UNIQUE 約束：`(account_id, project_id) WHERE deleted_at IS NULL`（部分索引，排除已軟刪除記錄，使帳號可重新加入專案）
3. 建立 `MemberService`：
   - `invite_member(project_id, account_id, role, tag_ids: Option<Vec<Uuid>>)` → 新增成員（可選同時指派初始標籤）
   - `list_members(project_id, filters)` → 列出專案成員（支援 role filter）
   - `update_member_role(member_id, new_role)` → 含 last-owner guard
   - `remove_member(member_id)` → 含 last-owner guard + soft delete
   - `get_member(member_id)` → 取得成員資訊
   - `get_by_project_and_account(project_id, account_id)` → 透過 Repository 取得特定帳號的成員身份
4. API 路由：
   - `GET /api/v1/projects/{projectId}/members`
   - `POST /api/v1/projects/{projectId}/members/invite`（與 `docs/api/paths/members.yaml` 一致）
   - `GET /api/v1/projects/{projectId}/members/{memberId}`（取得成員詳細資訊）
   - `PUT /api/v1/projects/{projectId}/members/{memberId}`
   - `DELETE /api/v1/projects/{projectId}/members/{memberId}`
5. 新增 DomainEvent：`ProjectMemberJoined`, `ProjectMemberRoleChanged`, `ProjectMemberRemoved`

**涉及檔案：**
- `migrations/0008_members.sql`
- `src/modules/core/member/mod.rs`, `models.rs`, `repository.rs`, `service.rs`, `error.rs`
- `src/api/routes/members.rs`

**測試要求：**
- 整合測試：Member CRUD
- 整合測試：角色變更權限檢查
- 整合測試：唯一性約束
- API 測試：所有成員端點

**驗收標準：**
- [x] 專案成員管理完整
- [x] 角色權限檢查正確
- [x] 成員 CRUD 通過測試
- [x] API 端點與 `docs/api/` 中的 OpenAPI spec 一致
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### B-3.2 Contact 資料表與聯絡人管理

**範圍：** 建立 contacts 資料表、組織層級聯絡人 CRUD、聯絡人合併功能。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0009_contacts.sql`：
   - `contacts` 表：id, organization_id (FK), name, email, merged_into_id (FK, nullable, self-reference), created_at, updated_at, deleted_at
   - 欄位需與 `docs/data-model/05-contact.md` 一致（注意：不含 notes 欄位）
3. 建立 `ContactService`：
   - `create_contact(org_id, name, email)` → 建立聯絡人
   - `list_contacts(org_id, search)` → 列出組織聯絡人（支援搜尋）
   - `update_contact(contact_id, name, email)`
   - `merge_contacts(org_id, source_ids: Vec<Uuid>, target_id)` → 合併重複聯絡人（含 SourceContainsTarget, CrossOrgMerge, AlreadyMerged 驗證）
   - `delete_contact(contact_id)` → 軟刪除
   - **查詢時 auto-follow 機制**：`get_contact` 在遇到 `merged_into_id IS NOT NULL` 時，自動跟隨至最終目標（含 cycle detection，max 10 hops）
4. API 路由：
   - `GET /api/v1/organizations/{orgId}/contacts`
   - `POST /api/v1/organizations/{orgId}/contacts`
   - `GET /api/v1/organizations/{orgId}/contacts/{contactId}`
   - `PUT /api/v1/organizations/{orgId}/contacts/{contactId}`
   - `DELETE /api/v1/organizations/{orgId}/contacts/{contactId}`
   - `POST /api/v1/organizations/{orgId}/contacts/merge`

**涉及檔案：**
- `migrations/0009_contacts.sql`
- `src/modules/core/contact/mod.rs`, `models.rs`, `repository.rs`, `service.rs`, `error.rs`
- `src/api/routes/contacts.rs`

**測試要求：**
- 整合測試：Contact CRUD
- 整合測試：聯絡人合併（驗證 merged_into_id 設定正確）
- 整合測試：合併後查詢自動導向目標聯絡人
- 整合測試：合併後 member_tag_assignments 中的引用正確更新
- 整合測試：合併後歷史 messages 的 source_id 正確處理（透過查詢時跟隨 merged_into_id）
- API 測試：所有聯絡人端點

**驗收標準：**
- [x] 聯絡人 CRUD 完整
- [x] 聯絡人合併功能正確
- [x] 合併後引用自動更新
- [x] API 端點與 `docs/api/` 中的 OpenAPI spec 一致
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### B-3.3 MemberTag 資料表與標籤系統

**範圍：** 建立 member_tags、member_tag_assignments 資料表、標籤 CRUD、成員/聯絡人標籤指派。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0010_member_tags.sql`：
   - `member_tags` 表：id, project_id (FK), name, description, external_task_creation (JSONB), created_at, updated_at, deleted_at
   - `member_tag_assignments` 表：id, tag_id (FK), member_id (FK, nullable), contact_id (FK, nullable), project_id (FK), created_at
   - `project_id` 為冗餘欄位（可從 tag_id → member_tags.project_id 推導），用於快速查詢。需在整合測試中驗證 project_id 與 tag_id 的一致性
   - CHECK 約束：`(member_id IS NOT NULL AND contact_id IS NULL) OR (member_id IS NULL AND contact_id IS NOT NULL)`（恰好指派給一方，XOR 語義）
3. 建立 `MemberTagService`：
   - `create_tag(project_id, name, description)` → 建立標籤
   - `update_tag(tag_id, name, description)` → 更新標籤
   - `delete_tag(tag_id)` → 軟刪除
   - `assign(tag_id, member_id, contact_id, project_id)` → 指派（含 XOR 驗證）
   - `unassign(tag_id, assignment_id, project_id)` → 取消指派
   - `list_tags(project_id)` → 列出專案所有標籤（含 member_count/contact_count）
   - `get_tag_detail(tag_id)` → 標籤詳情（含 assigned members + contacts）
   - `update_external_task_creation(tag_id, settings)` → 更新外部建立任務設定
4. API 路由：
   - `GET /api/v1/projects/{projectId}/member-tags`
   - `POST /api/v1/projects/{projectId}/member-tags`
   - `GET /api/v1/projects/{projectId}/member-tags/{tagId}`（取得標籤詳細資訊，含成員與聯絡人列表）
   - `PUT /api/v1/projects/{projectId}/member-tags/{tagId}`（與 `docs/api/paths/member-tags.yaml` 一致，完整替換語義）
   - `DELETE /api/v1/projects/{projectId}/member-tags/{tagId}`
   - `POST /api/v1/projects/{projectId}/member-tags/{tagId}/assign`（與 API Spec 一致）
   - `DELETE /api/v1/projects/{projectId}/member-tags/{tagId}/assignments/{assignmentId}`（取消指派）
   - `PUT /api/v1/projects/{projectId}/member-tags/{tagId}/external-task-creation`（更新外部建立任務設定）

**涉及檔案：**
- `migrations/0010_member_tags.sql`
- `src/modules/core/member_tag/mod.rs`, `models.rs`, `repository.rs`, `service.rs`, `error.rs`
- `src/api/routes/member_tags.rs`

**測試要求：**
- 整合測試：MemberTag CRUD
- 整合測試：成員/聯絡人指派與取消
- 整合測試：external_task_creation JSONB 讀寫
- API 測試：所有標籤端點

**驗收標準：**
- [x] 標籤 CRUD 完整
- [x] 成員與聯絡人可指派至標籤
- [x] external_task_creation 配置可讀寫
- [x] API 端點與 `docs/api/` 中的 OpenAPI spec 一致
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### B-3.4 完整 RBAC 權限模型（deny-first）

**範圍：** 擴展 Phase 2 的基礎 RBAC，實作完整的 deny-first 權限計算，涵蓋組織、專案、成員標籤三個層級。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 擴展 `check_permission` 支援：
   - 組織層級：org_owner / org_admin / org_member 權限矩陣
   - 專案層級：owner / tag_admin / member 權限矩陣
   - 標籤層級：標籤成員可操作該標籤下的任務與相關資源
3. **建立 `evaluate_deny_first` 函數**（參考 `docs/system/09-authorization.md`）：
   - 實作完整的 deny-first 權限計算邏輯
   - **多標籤聚合演算法**：
     1. 收集使用者所屬的所有標籤對應的 `PermissionRule`
     2. 聯集（union）所有標籤的 `deny` 清單 → 得到 `denied_set`
     3. 聯集（union）所有標籤的 `allow` 清單 → 得到 `allowed_set`
     4. 最終有效權限 = `allowed_set - denied_set`（任一標籤的 deny 優先於所有 allow）
     5. 若操作不在最終有效權限中 → 拒絕（預設拒絕）
   - 若使用者不屬於任何標籤，或所有標籤均無明確設定 → 預設拒絕
   - 支援 wildcard `"*"` 標籤和項目
4. 權限設定儲存於 `projects.permission_settings` JSONB 欄位（已在 Phase 2 建立），以 `serde_json::Value` 存取
5. ~~建立 migration `0008_permission_extensions.sql`~~ — 不需要，permission_settings JSONB 欄位已在 Phase 2 的 projects 表中建立
6. **權限矩陣實作於程式碼中**（`is_allowed` 和 `is_allowed_project_role` 函數），涵蓋：
   - 組織層級操作：成員管理、聯絡人管理
   - 專案層級操作：標籤管理、成員管理、權限設定
7. 使用 moka cache 快取權限計算結果（已實作：TTL 300s + EventBus 事件驅動 invalidation）
8. 專案複製時一同複製 permission_settings

**涉及檔案：**
- `src/modules/core/permission/service.rs`（大幅擴展：Action enum + Resource::ProjectScoped + check 方法 + 權限矩陣）
- `src/modules/core/permission/deny_first.rs`（新增）

**測試要求：**
- 單元測試：deny-first 策略（多標籤場景）
  - 場景 1：使用者屬於標籤 A（允許）和標籤 B（拒絕）→ 預期結果：拒絕 ✅
  - 場景 2：wildcard allow/deny 場景 ✅
  - 場景 3：無規則預設拒絕 ✅
- 單元測試：組織 + 專案權限矩陣（owner_can_all, admin_permissions, member_permissions, project role matrix）✅
- API 測試：無權限操作回傳 403

**驗收標準：**
- [x] deny-first 策略正確實作
- [x] 三層權限計算正確
- [x] 權限快取有效運作（moka in-memory cache + EventBus invalidation）
- [x] 專案複製包含 permission_settings
- [x] 權限矩陣文件完成（已新增至 `docs/system/09-authorization.md` 5.5 節）
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-3.1 成員、聯絡人與標籤管理頁面

**範圍：** 專案成員管理、組織聯絡人管理、標籤管理與指派 UI。

**說明：**
1. `ProjectMembersView.vue`：
   - 成員列表（角色、標籤標示）
   - 新增成員（從組織成員中選擇）
   - 角色變更、移除成員
2. `ContactsView.vue`：
   - 聯絡人列表（支援搜尋）
   - 新增/編輯聯絡人
   - 合併聯絡人 UI（選擇來源與目標）
3. `MemberTagsView.vue`：
   - 標籤列表
   - 新增/編輯標籤（含 external_task_creation 設定）
   - 指派成員/聯絡人至標籤
4. `ProjectContactsView.vue`：
   - 專案相關聯絡人列表（透過 tag assignments）
5. 建立 `memberStore`, `contactStore`, `memberTagStore`

**涉及檔案：**
- `frontend/src/views/projects/ProjectMembersView.vue`
- `frontend/src/views/organizations/ContactsView.vue`
- `frontend/src/views/projects/MemberTagsView.vue`
- `frontend/src/views/projects/ProjectContactsView.vue`
- `frontend/src/stores/member.ts`, `frontend/src/stores/contact.ts`, `frontend/src/stores/memberTag.ts`
- `frontend/src/router/index.ts`（新增 4 條路由）
- `frontend/src/api/schema.d.ts`（新增 23 個 schemas + 19 個 path endpoints）

**測試要求：**
- 元件測試：各列表渲染 ✅
- 元件測試：標籤指派互動 ✅
- 元件測試：聯絡人合併流程 ✅
- 單元測試：各 store ✅

**驗收標準：**
- [x] 成員管理 UI 完整
- [x] 聯絡人管理與合併 UI 可操作
- [x] 標籤管理與指派 UI 完整
- [x] `pnpm run lint` 零警告
- [x] `pnpm run typecheck` 通過
- [x] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 3 後，以下端到端流程可驗證：

1. **成員管理**：在專案中新增成員 → 設定角色 → 成員可存取專案
2. **聯絡人管理**：建立組織聯絡人 → 合併重複聯絡人 → 聯絡人可指派至專案標籤
3. **標籤系統**：建立標籤 → 指派成員/聯絡人 → 設定 externalTaskCreation
4. **RBAC 驗證**：不同角色的使用者看到不同的操作按鈕，無權限操作被阻擋

## 驗證結果

### 後端驗證（2026-02-20）
- `cargo fmt -- --check` ✅ 通過
- `cargo clippy -- -D warnings` ✅ 零警告
- `cargo test` ✅ 140 tests passed, 0 failed

### 前端驗證（2026-02-20）
- `pnpm run lint` ✅ 零錯誤
- `pnpm run typecheck` ✅ 通過
- `pnpm run test` ✅ 22 files, 116 tests passed
