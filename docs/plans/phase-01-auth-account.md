# Phase 1：認證與帳號管理

**階段目標：** 實作完整的認證流程（Passkey + Email Magic Link）與帳號管理，建立 JWT 簽發/驗證機制，使用者可註冊、登入並管理個人資料。

**前置依賴：** Phase 0 完成

---

## 後端任務

### B-1.1 Account 資料表與 Repository

**範圍：** 建立 accounts 相關資料表 migration、Rust 結構體、Repository CRUD。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0002_accounts.sql`：
   - `accounts` 表：id, name (VARCHAR, NOT NULL), email (VARCHAR, NOT NULL), avatar_url (VARCHAR, nullable), bio (TEXT, nullable), profile_data (JSONB, NOT NULL, DEFAULT '{}'), profile_schema (JSONB, NOT NULL, DEFAULT '[]'), notification_preferences (JSONB, NOT NULL, DEFAULT '{}' — Repository 層查詢時自動合併預設值結構，確保回應符合 API Schema 的 NotificationPreferences 定義), locale (VARCHAR, NOT NULL, DEFAULT 'zh-TW'), created_at (TIMESTAMPTZ, NOT NULL), updated_at (TIMESTAMPTZ, NOT NULL), deleted_at (TIMESTAMPTZ, nullable)
   - Email 唯一索引排除已刪除帳號：`CREATE UNIQUE INDEX uq_accounts_email ON accounts (email) WHERE deleted_at IS NULL`
   - `passkey_credentials` 表：id, account_id (FK), credential_id (BYTEA, UNIQUE), public_key (BYTEA), counter (BIGINT, DEFAULT 0), name (VARCHAR, nullable — 使用者自訂名稱如「MacBook Touch ID」), created_at, last_used_at (TIMESTAMPTZ, nullable)
   - `magic_link_tokens` 表：id, account_id (FK, NOT NULL — Magic Link 發送前先建立帳號), token_hash (BYTEA, UNIQUE — 儲存雜湊值而非明文), expires_at, used_at, created_at
   - `refresh_tokens` 表：id, account_id (FK), token_hash (BYTEA, UNIQUE), expires_at, revoked_at, rotated_at (TIMESTAMPTZ, nullable), replaced_by (FK, nullable — 指向新 token), created_at
   - 索引定義：
     - `idx_accounts_locale` — accounts(locale)，用於批次通知等查詢
     - `idx_passkey_credentials_account_id` — passkey_credentials(account_id)
     - `idx_magic_link_tokens_token_hash` — magic_link_tokens(token_hash)
     - `idx_magic_link_tokens_account_id` — magic_link_tokens(account_id)
     - `idx_refresh_tokens_token_hash` — refresh_tokens(token_hash)
     - `idx_refresh_tokens_account_id` — refresh_tokens(account_id)
3. 建立 Rust 結構體：`Account`, `PasskeyCredential`, `MagicLinkToken`, `RefreshToken`
4. 建立 `AccountRepository`：create, get_by_id, get_by_email, update, soft_delete
5. 使用 `sqlx::query_as!` compile-time checked queries

**涉及檔案：**
- `migrations/0002_accounts.sql`
- `src/modules/auth/mod.rs`, `src/modules/auth/models.rs`, `src/modules/auth/service.rs`, `src/modules/auth/repository.rs`, `src/modules/auth/error.rs`, `src/modules/auth/tests.rs`

**測試要求：**
- 整合測試：Account CRUD（建立、查詢、更新、軟刪除）
- 整合測試：Email 唯一性約束
- 整合測試：Profile data JSONB 讀寫

**驗收標準：**
- [ ] accounts 及相關表 migration 成功
- [ ] Repository CRUD 全部通過測試
- [ ] 軟刪除機制正確運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] `cargo xtask generate-api-types --check` 型別同步通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-1.2 JWT 簽發與驗證

**範圍：** 實作 JWT Access Token（15 分鐘）+ HTTP-only Refresh Cookie（7 天）機制。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/modules/auth/jwt.rs`：
   - `issue_access_token(account_id) -> String`：簽發 JWT access token，payload 含 account_id、exp
   - `validate_access_token(token) -> Claims`：驗證 JWT 並返回 Claims
   - `issue_refresh_token(account_id) -> RefreshToken`：產生 refresh token，存入 DB，回傳 HTTP-only cookie
   - `refresh_access_token(refresh_token) -> (AccessToken, RefreshToken)`：refresh 流程（token rotation）
3. 建立 Axum authentication middleware（`src/api/middleware/auth.rs`）：
   - 從 Authorization header 提取 Bearer token
   - 驗證 JWT → 注入 `AuthUser` extractor
4. 建立 `src/api/extractors/auth.rs`：`AuthUser` extractor
5. Refresh token rotation 含並發安全：使用 `SELECT ... FOR UPDATE` 原子操作，30 秒 grace period 機制避免並發請求導致的 token 失效

**涉及檔案：**
- `src/modules/auth/jwt.rs`
- `src/api/middleware/auth.rs`
- `src/api/extractors/auth.rs`

**測試要求：**
- 單元測試：JWT 簽發與驗證（含過期測試）
- 單元測試：Refresh token rotation
- 單元測試：Token 雜湊值驗證（確認不儲存明文）
- API 測試：受保護路由在無 token 時回傳 401
- API 測試：受保護路由在有效 token 時回傳 200
- API 測試：Token reuse attack 偵測（grace period 測試）
- 整合測試：並發 refresh 請求的原子性

**驗收標準：**
- [ ] JWT access token 15 分鐘過期
- [ ] Refresh token 7 天過期，使用 HTTP-only cookie
- [ ] Token rotation 正確運作
- [ ] 認證 middleware 可保護路由
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-1.3 Email Magic Link 認證

**範圍：** 實作 Email Magic Link 登入/註冊流程。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. `POST /api/v1/auth/magic-link/request`（與 `docs/api/paths/auth.yaml` 一致）：
   - 接收 email，產生一次性 token（32 bytes, URL-safe base64）
   - 存入 `magic_link_tokens`（expires_at = now + 15 min）
   - 透過 SMTP 寄送含 token 的連結（使用 lettre）
   - 無縫註冊/登入流程：若 email 不存在於 accounts，先自動建立帳號再發送登入信；所有合法 Email 請求皆發送登入信，回應訊息一致以確保使用者體驗統一
3. `GET /api/v1/auth/magic-link/verify`（與 `docs/api/paths/auth.yaml` 一致，透過 query param 傳遞 token）：
   - 驗證 token（未過期、未使用）
   - 標記 token 為已使用
   - 簽發 JWT access token + refresh token
4. 建立 email 寄送基礎設施（`src/modules/email/smtp.rs`）

**涉及檔案：**
- `src/modules/auth/service.rs`
- `src/modules/auth/magic_link.rs`
- `src/modules/email/smtp.rs`
- `src/api/routes/auth.rs`

**測試要求：**
- 整合測試：Magic link 建立 → 驗證完整流程
- 整合測試：過期 token 拒絕
- 整合測試：重複使用 token 拒絕
- API 測試：完整 HTTP 流程

**驗收標準：**
- [ ] Magic link 可成功寄出（開發環境透過 MailHog 確認）
- [ ] Token 驗證後可取得 JWT
- [ ] 新使用者自動建立帳號
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-1.4 Passkey (WebAuthn) 認證

**範圍：** 實作 WebAuthn 註冊與認證流程。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 使用 `webauthn-rs` crate
3. 註冊流程（已登入使用者新增 Passkey）：
   - `POST /api/v1/auth/passkey/register/begin` → 返回 PublicKeyCredentialCreationOptions
   - `POST /api/v1/auth/passkey/register/complete` → 儲存 credential 至 `passkey_credentials`
4. 認證流程：
   - `POST /api/v1/auth/passkey/login/begin` → 返回 PublicKeyCredentialRequestOptions
   - `POST /api/v1/auth/passkey/login/complete` → 驗證 credential → 簽發 JWT
5. 配置 WebAuthn RP ID 與 Origin（從環境變數讀取）
6. 註冊端點需要 `AuthUser` extractor 保護（已登入使用者新增 Passkey）

**涉及檔案：**
- `src/modules/auth/passkey.rs`
- `src/api/routes/auth.rs`（擴展）

**測試要求：**
- 單元測試：WebAuthn challenge 產生與驗證邏輯
- API 測試：註冊 begin/complete 完整流程
- API 測試：認證 begin/complete 完整流程

**驗收標準：**
- [ ] Passkey 註冊流程完整
- [ ] Passkey 認證流程完整
- [ ] Sign count 正確遞增
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-1.5 帳號管理 API

**範圍：** 帳號個人資料 CRUD API、密碼變更、Session 管理。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. `GET /api/v1/accounts/me` — 取得目前使用者資料
3. `PATCH /api/v1/accounts/me` — 更新 name, bio, avatar, locale
4. `GET /api/v1/accounts/me/profile` — 取得個人資料表（profile_data）
5. `PUT /api/v1/accounts/me/profile` — 更新個人資料表
6. `GET /api/v1/accounts/me/notification-preferences` — 查詢通知偏好（與 API Spec 一致）
7. `PUT /api/v1/accounts/me/notification-preferences` — 更新通知偏好
   > **注意：** Phase 1 僅建立基本 API stub，完整實作見 Phase 10
8. `POST /api/v1/auth/logout` — 撤銷 refresh token
9. `POST /api/v1/auth/refresh` — refresh access token
10. 新增 DomainEvent：`AccountCreated`, `AccountUpdated`

**涉及檔案：**
- `src/api/routes/accounts.rs`
- `src/modules/auth/service.rs`（擴展）
- `src/events.rs`（新增事件）

**測試要求：**
- API 測試：所有帳號 CRUD 端點
- API 測試：未認證存取回傳 401
- 整合測試：Logout 後 refresh token 失效

**驗收標準：**
- [ ] 帳號 CRUD API 完整
- [ ] 認證保護正確
- [ ] DomainEvent 正確發布
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-1.1 登入/註冊頁面

**範圍：** Email Magic Link 登入頁面、Passkey 登入支援、Magic Link 驗證頁面。

**說明：**
1. `LoginView.vue`：
   - Email 輸入欄位 + 「Send Magic Link」按鈕
   - Passkey 登入按鈕（呼叫 WebAuthn API）
   - 發送成功後顯示「請查看您的信箱」提示
2. `MagicLinkVerifyView.vue`：
   - 從 URL query 提取 token
   - 自動呼叫驗證 API
   - 成功後儲存 token → 跳轉至首頁
3. 建立 `useAuth` composable（完善）：
   - `login(email)`, `verifyMagicLink(token)`, `loginWithPasskey()`, `logout()`, `refreshToken()`
   - 使用 Pinia store 管理認證狀態
4. 建立 `authStore`（Pinia）：管理 accessToken, currentUser（Refresh Token 透過 HTTP-only cookie 自動管理，前端不直接操作）

**涉及檔案：**
- `frontend/src/views/auth/LoginView.vue`
- `frontend/src/views/auth/MagicLinkVerifyView.vue`
- `frontend/src/composables/useAuth.ts`
- `frontend/src/stores/auth.ts`

**測試要求：**
- 元件測試：LoginView 表單提交
- 元件測試：MagicLinkVerifyView token 處理
- 單元測試：authStore 狀態管理

**驗收標準：**
- [ ] Email Magic Link 登入流程可操作
- [ ] Passkey 登入按鈕可觸發 WebAuthn
- [ ] Token 管理正確（存儲、refresh、過期處理）
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### F-1.2 帳號設定頁面

**範圍：** 帳號個人資料編輯、Passkey 管理、通知偏好設定。

**說明：**
1. `AccountSettingsView.vue`：
   - 個人資料編輯（name, bio, avatar 上傳）
   - Passkey 管理（列表、新增、刪除）
   - 通知偏好設定（各頻道開關）
2. `ProfileDataView.vue`：
   - 個人資料表（profile_data）編輯
   - 動態表單根據 profile_schema 生成欄位
3. 建立 `useAccount` composable

**涉及檔案：**
- `frontend/src/views/settings/AccountSettingsView.vue`
- `frontend/src/views/settings/ProfileDataView.vue`
- `frontend/src/composables/useAccount.ts`

**測試要求：**
- 元件測試：表單渲染與提交
- 元件測試：Passkey 管理互動
- 單元測試：useAccount composable

**驗收標準：**
- [ ] 個人資料可編輯並儲存
- [ ] Passkey 可新增與管理
- [ ] 通知偏好可設定
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 1 後，以下端到端流程可驗證：

1. **註冊流程**：輸入 Email → 收到 Magic Link（MailHog 確認）→ 點擊連結 → 自動建立帳號並登入
2. **登入流程**：Email Magic Link 登入 + Passkey 登入均可使用
3. **Token 管理**：Access token 過期後自動 refresh；手動 logout 後 refresh token 失效
4. **帳號管理**：可編輯個人資料、管理 Passkey、設定通知偏好
5. **認證保護**：未登入時自動跳轉至登入頁面
