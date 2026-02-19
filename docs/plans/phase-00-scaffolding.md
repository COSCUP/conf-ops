# Phase 0：專案腳手架與基礎設施 ✅ COMPLETED

**階段目標：** 建立完整的 Rust 後端與 Vue 前端專案骨架，含 CI、Docker Compose 開發環境、共用基礎模組，使後續所有階段能直接在此基礎上開發。

**完成日期：** 2026-02-19
**實作 commits：** `4f0c23d` (feat: scaffold Phase 0 project infrastructure)

---

## 共享任務

### S-0.1 Rust 專案初始化與 Workspace 設定

**範圍：** 初始化 Cargo workspace，設定所有 9 個模組的 crate 結構、共用 lints、依賴版本管理。

**說明：**
1. 初始化 Cargo workspace，建立 `Cargo.toml`（workspace 層級）
2. 建立 `src/` 目錄結構，按 `docs/development-guidelines.md` 中的專案結構：
   ```
   src/
     main.rs
     lib.rs
     config.rs
     modules/
       auth/mod.rs
       core/mod.rs
       conversation/mod.rs
       ai/mod.rs
       tools/mod.rs
       email/mod.rs
       notifications/mod.rs
       storage/mod.rs
       audit/mod.rs
     api/
       routes/mod.rs
       middleware/mod.rs
       extractors/mod.rs
       error.rs
   ```
3. 設定 `Cargo.toml` lints（clippy all deny、pedantic warn、nursery warn）
4. 新增共用依賴：`tokio`, `axum`, `sqlx`, `serde`, `uuid`, `chrono`, `thiserror`, `tracing`, `tracing-subscriber`
5. 建立 `.env.example` 參考 `docs/system/02-deployment-architecture.md` 中的環境變數
6. 建立 `xtask` 子專案（`cargo xtask generate-openapi`），參考 `docs/development-guidelines.md` 第 5 節，供後續 Phase 從 OpenAPI spec 生成 Rust API 類型

**涉及檔案：**
- `Cargo.toml`, `Cargo.lock`
- `src/main.rs`, `src/lib.rs`, `src/config.rs`
- `src/modules/*/mod.rs`（9 個模組空殼）
- `src/api/routes/mod.rs`, `src/api/middleware/mod.rs`, `src/api/extractors/mod.rs`, `src/api/error.rs`
- `.env.example`

**測試要求：**
- `cargo build` 編譯通過
- `cargo clippy -- -D warnings` 零警告
- `cargo fmt -- --check` 通過

**驗收標準：**
- [x] Workspace 結構完整，9 個模組空殼可編譯
- [x] Clippy 配置為 strict（all deny + pedantic warn + nursery warn）
- [x] 環境變數配置檔案完整
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### S-0.2 Vue 前端專案初始化

**範圍：** 使用 Vite + Vue 3 建立前端專案，配置 TypeScript strict mode、ESLint、Vitest、Pinia。

**說明：**
1. `npm create vue@latest frontend` — 選擇 TypeScript、Vue Router、Pinia、Vitest
2. 配置 `tsconfig.json`：`strict: true`, `noUncheckedIndexedAccess: true`, `noUnusedLocals: true`, `noUnusedParameters: true`, `exactOptionalPropertyTypes: true`
3. 配置 ESLint（`@antfu/eslint-config` 或等效嚴格配置），確保 `--max-warnings 0`
4. 安裝 `openapi-typescript` + `openapi-fetch`，配置型別生成腳本
5. 建立基礎目錄結構：`src/api/`, `src/components/`, `src/composables/`, `src/stores/`, `src/views/`, `src/layouts/`
6. 建立 API 客戶端骨架（`src/api/client.ts`）使用 `openapi-fetch`

**涉及檔案：**
- `frontend/package.json`, `frontend/tsconfig.json`
- `frontend/eslint.config.js`
- `frontend/vitest.config.ts`
- `frontend/src/api/client.ts`
- `frontend/src/stores/`, `frontend/src/composables/`

**測試要求：**
- `npm run lint -- --max-warnings 0` 零警告
- `npm run typecheck` 通過
- `npm run test` 通過（含一個 smoke test）

**驗收標準：**
- [x] TypeScript strict mode 完整啟用
- [x] ESLint 零警告
- [x] openapi-typescript 型別生成腳本可執行
- [x] 基礎目錄結構與 API 客戶端骨架就緒
- [x] `npm run lint -- --max-warnings 0` 零警告
- [x] `npm run typecheck` 通過
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### S-0.3 Docker Compose 開發環境

**範圍：** 建立 `docker-compose.dev.yml`，啟動 PostgreSQL 16 + MailHog 開發環境。

**說明：**
1. 按 `docs/system/02-deployment-architecture.md` 第 3 節建立 `docker-compose.dev.yml`
2. PostgreSQL 16（port 5432），資料庫名 `confops_dev`
3. MailHog（SMTP port 1025，Web UI port 8025）
4. 建立 `Makefile` 統一開發指令：`make dev-up`, `make dev-down`, `make migrate`, `make test`, `make lint`
5. Makefile 指令包含分離的前後端操作：
   - `make lint-backend`：`cargo clippy -- -D warnings && cargo fmt -- --check`
   - `make lint-frontend`：`cd frontend && npm run lint -- --max-warnings 0`
   - `make test-backend`：`cargo test`
   - `make test-frontend`：`cd frontend && npm run test`
   - `make lint`：lint-backend + lint-frontend
   - `make test`：test-backend + test-frontend

**涉及檔案：**
- `docker-compose.dev.yml`
- `Makefile`
- `.env.example`（補充 Docker 相關變數）

**測試要求：**
- `docker compose -f docker-compose.dev.yml up -d` 成功啟動所有服務
- PostgreSQL 可連線並查詢

**驗收標準：**
- [x] `make dev-up` 一鍵啟動所有開發依賴
- [x] PostgreSQL 16 可存取
- [x] MailHog Web UI 可開啟
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### S-0.4 CI Pipeline 與品質閘門

**範圍：** 建立 GitHub Actions CI，涵蓋後端（clippy + fmt + test）與前端（lint + typecheck + test）。

**說明：**
1. 建立 `.github/workflows/ci.yml`，參考 `docs/development-guidelines.md` 第 6 節
2. 後端 job：`cargo fmt -- --check` → `cargo clippy -- -D warnings` → `cargo test` → `cargo xtask generate-openapi --check`（驗證生成的 API 類型與 spec 同步）
3. 前端 job：`npm ci` → `npm run lint -- --max-warnings 0` → `npm run typecheck` → `npm run test` → `npx openapi-typescript` 型別同步驗證
4. OpenAPI spec 驗證 job：`npx @redocly/cli lint docs/api/openapi.yaml`
5. 後端測試使用 `pg_lite` 管理測試用 PostgreSQL（無需 CI service container）

**涉及檔案：**
- `.github/workflows/ci.yml`

**測試要求：**
- CI 在空白專案上全綠

**驗收標準：**
- [x] Push / PR 自動觸發 CI
- [x] 後端 clippy + fmt + test 全部檢查
- [x] 前端 lint + typecheck + test 全部檢查
- [x] OpenAPI spec 格式驗證
- [x] 所有 commit 遵循 Conventional Commits 格式

---

## 後端任務

### B-0.1 資料庫連線池與 Migration 基礎設施

**範圍：** 建立 sqlx 連線池初始化、migration 基礎設施、TestContext 測試輔助工具。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 在 `src/config.rs` 實作環境變數載入與驗證（`DATABASE_URL`, `DATABASE_MAX_CONNECTIONS` 等）
3. 建立 `src/db.rs`：sqlx PgPool 初始化，連線池配置
4. 安裝 `sqlx-cli`，建立 `migrations/` 目錄
5. 建立第一個 migration `0001_init.sql`：啟用 `uuid-ossp` 與 `pgcrypto` 擴充功能（UUID v7 由應用層透過 `uuid` crate v7 feature 生成）
6. 建立 `tests/common/mod.rs`：TestContext 結構體，使用 `pg_lite` 管理測試用 PostgreSQL
7. TestContext 提供 `new()` → 自動建立獨立 DB + 執行 migration
8. 建立共用的 `generate_id()` 函式，使用 UUID v7 生成主鍵

**涉及檔案：**
- `src/config.rs`
- `src/db.rs`
- `migrations/0001_init.sql`
- `tests/common/mod.rs`

**測試要求：**
- 單元測試：config 載入與驗證
- 整合測試：TestContext 可成功建立並連線
- Migration 可成功執行

**驗收標準：**
- [x] sqlx 連線池正常運作
- [x] Migration 可成功執行
- [x] TestContext 可用於測試，每次測試使用獨立資料庫
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### B-0.2 Axum HTTP Server 與健康檢查端點

**範圍：** 建立 Axum HTTP server 啟動邏輯、路由掛載基礎、`/healthz` 與 `/readyz` 端點。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 在 `src/main.rs` 建立 Tokio runtime + Axum server 啟動邏輯
3. 建立 AppState 結構體，持有 PgPool、EventBus 等共用資源
4. 建立路由掛載框架（`/api/v1/` prefix）
5. 實作 `GET /healthz`（固定 200）和 `GET /readyz`（檢查 DB 連線）
   > **注意**：`/readyz` 回應包含 `checks` 物件，其中 `database` 欄位回傳 `ok` 或 `error`，詳見 `docs/api/paths/health.yaml`。
   > **注意**：`/metrics` 端點與完整可觀測性基礎設施（tracing、Prometheus 指標）延遲至 Phase 11 (B-11.4) 實作。Phase 0 僅建立健康檢查端點。
6. 建立 `src/api/error.rs`：RFC 7807 Problem Details 錯誤回應結構體（含 type, title, status, detail, instance 欄位），參考 `docs/api/schemas/common.yaml` 中的 ProblemDetails schema

**涉及檔案：**
- `src/main.rs`
- `src/api/routes/mod.rs`, `src/api/routes/health.rs`
- `src/api/error.rs`

**測試要求：**
- API 測試：`/healthz` 回傳 200
- API 測試：`/readyz` 在 DB 正常時回傳 200，異常時回傳 503
- 單元測試：RFC 7807 錯誤格式正確

**驗收標準：**
- [x] `cargo run` 啟動 HTTP server
- [x] `/healthz` 與 `/readyz` 正常回應
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 所有 commit 遵循 Conventional Commits 格式

---

### B-0.3 Event Bus（領域事件匯流排）

**範圍：** 建立 Tokio channel 為基礎的 Event Bus，定義 DomainEvent 枚舉，供後續各階段擴展。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 `src/events.rs`：
   - `DomainEvent` 枚舉（初始含基礎事件 `SystemStarted`、`SystemHealthCheck`，後續階段逐步新增變體）
   - `EventBus` 結構體，基於 `tokio::sync::broadcast` channel
   - `publish()` 發布事件、`subscribe()` 訂閱事件
3. 將 EventBus 注入 AppState
4. 建立基礎的事件訂閱者框架（audit 模組用），為後續階段的事件驅動架構做準備

**涉及檔案：**
- `src/events.rs`
- `src/main.rs`（AppState 更新）

**測試要求：**
- 單元測試：事件發布與訂閱正確運作
- 單元測試：多個訂閱者可同時接收同一事件

**驗收標準：**
- [x] EventBus 可發布和訂閱事件
- [x] 事件傳遞非阻塞
- [x] DomainEvent 枚舉可方便擴展
- [x] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [x] `cargo fmt -- --check` 通過
- [x] 錯誤回應符合 RFC 7807 Problem Details 格式
- [x] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-0.1 基礎 Layout 與路由框架

**範圍：** 建立應用程式 Shell（頂部導航、側邊欄框架）、Vue Router 路由結構、認證守衛框架。

**說明：**
1. 建立 `AppLayout.vue`（認證後使用的主 layout）與 `AuthLayout.vue`（登入/註冊用 layout）
2. 配置 Vue Router：
   - `/login`, `/register` → AuthLayout
   - `/`, `/projects/*`, `/settings/*` → AppLayout
3. 建立路由守衛框架（`router.beforeEach`），檢查認證狀態（Phase 1 再實作真正邏輯）
4. 建立 `useAuth` composable 骨架
5. 建立基礎 UI 元件：`BaseButton`, `BaseInput`, `BaseCard`

**涉及檔案：**
- `frontend/src/layouts/AppLayout.vue`, `frontend/src/layouts/AuthLayout.vue`
- `frontend/src/router/index.ts`
- `frontend/src/composables/useAuth.ts`
- `frontend/src/components/base/BaseButton.vue`, `BaseInput.vue`, `BaseCard.vue`

**測試要求：**
- 元件測試：Layout 元件正確渲染
- 單元測試：路由守衛在未認證時重導向至 `/login`

**驗收標準：**
- [x] 基礎 Layout 渲染正確
- [x] 路由切換正常
- [x] 認證守衛框架就緒（骨架）
- [x] `npm run lint -- --max-warnings 0` 零警告
- [x] `npm run typecheck` 通過
- [x] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 0 後，以下端到端流程可驗證：

1. **開發環境啟動**：`make dev-up` 啟動 PostgreSQL + MailHog → `cargo run` 啟動後端 → `npm run dev` 啟動前端
2. **健康檢查**：`curl http://localhost:8080/healthz` 回傳 `{"status":"ok"}`
3. **CI 全綠**：Push 到 GitHub 觸發 CI，所有 job 通過
4. **前端頁面**：瀏覽器開啟 `http://localhost:3000`，顯示基礎 Layout
5. **測試全通過**：`cargo test` + `npm run test` 全部綠燈
