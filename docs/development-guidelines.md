# 開發準則

本文件定義 Conf-Ops 專案的開發流程、品質要求與技術規範，所有貢獻者須遵循。

---

## 1. 開發流程：Test-Driven Development (TDD)

所有功能開發與 bug 修復皆採用 TDD 流程：

1. **Red**：先撰寫失敗的測試，明確定義預期行為
2. **Green**：撰寫最少量的程式碼使測試通過
3. **Refactor**：在測試保護下重構程式碼，消除重複與改善可讀性

### 提交前檢查清單

每次提交前，必須確認以下所有條件皆滿足：

- [ ] `cargo clippy -- -D warnings` 無任何錯誤與警告（含 info 層級）
- [ ] `cargo fmt -- --check` 格式檢查通過
- [ ] `cargo test` 所有測試通過（單元測試 + 整合測試）
- [ ] 前端 `pnpm run lint` 無任何錯誤與警告
- [ ] 前端 `pnpm run typecheck` 類型檢查通過
- [ ] 前端 `pnpm run test` 所有測試通過

### Lint 修復原則

- **禁止以忽略 / suppress 方式修復 lint 問題**（如 `#[allow(...)]`、`// eslint-disable`、`@ts-ignore` 等），除非確實無法以正當方式修復
- 必須使用忽略時，需先詢問確認，並在註解中說明原因

### Clippy 設定

在 `Cargo.toml` 或 `.cargo/config.toml` 中啟用嚴格 lint：

```toml
# Cargo.toml
[lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
# 依專案需求調整個別規則
module_name_repetitions = "allow"
must_use_candidate = "allow"
```

---

## 2. 測試策略

### 2.1 測試分層

| 層級 | 範圍 | 工具 | 執行時機 |
|------|------|------|----------|
| **單元測試** | 單一函式 / struct / module | `#[cfg(test)]` 內嵌模組 | 每次編譯 |
| **API 測試** | HTTP 端點的請求 / 回應驗證 | `axum::test` 或 `reqwest` + test server | CI pipeline |
| **整合測試** | 跨模組的完整業務流程 | `tests/` 目錄 + 真實 DB | CI pipeline |

### 2.2 資料庫測試：使用 postgresql_embedded（無需手動安裝 PostgreSQL）

**禁止使用 mock 資料庫。** 所有涉及資料庫的測試必須使用真實 PostgreSQL 實例。

使用 [`postgresql_embedded`](https://crates.io/crates/postgresql_embedded) 管理測試用 PostgreSQL。`postgresql_embedded` 會在測試執行時自動下載並啟動嵌入式 PostgreSQL，**不需要預先安裝或手動啟動 PostgreSQL 服務**，可直接執行 `cargo test`：

```rust
use postgresql_embedded::PostgreSQL;

#[tokio::test]
async fn test_create_task() {
    let mut pg = PostgreSQL::default();
    pg.setup().await.unwrap();
    pg.start().await.unwrap();

    let db_name = "test_db";
    pg.create_database(db_name).await.unwrap();

    let settings = pg.settings();
    let url = format!(
        "postgres://{}:{}@{}:{}/{}",
        settings.username, settings.password, settings.host, settings.port, db_name,
    );
    let pool = PgPool::connect(&url).await.unwrap();

    // 執行 migration
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    // 執行測試邏輯（_pg 離開 scope 後自動清理）
}
```

#### 測試輔助工具

建立共用的測試 fixture 模組，避免重複的設定程式碼：

```rust
// tests/common/mod.rs
pub struct TestContext {
    pub pool: PgPool,
    pub app: TestApp,
    _pg: PostgreSQL,  // 保持存活，離開 scope 後自動停止並清理
}

impl TestContext {
    pub async fn new() -> Self {
        let mut pg = PostgreSQL::default();
        pg.setup().await.expect("Failed to setup PostgreSQL");
        pg.start().await.expect("Failed to start PostgreSQL");
        let db_name = format!("test_{}", Uuid::now_v7().simple());
        pg.create_database(&db_name).await.unwrap();
        let settings = pg.settings();
        let url = format!(
            "postgres://{}:{}@{}:{}/{}",
            settings.username, settings.password, settings.host, settings.port, db_name,
        );
        let pool = PgPool::connect(&url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let app = TestApp::new(pool.clone()).await;
        Self { pool, app, _pg: pg }
    }

    /// 建立測試用帳號，回傳 account_id 與 JWT token
    pub async fn create_authenticated_user(&self) -> (Uuid, String) { /* ... */ }

    /// 建立測試用專案（含組織與成員）
    pub async fn create_project_with_member(&self) -> TestProject { /* ... */ }
}
```

### 2.3 Bug 修復測試

每個 bug 修復必須附帶對應的迴歸測試：

1. 先撰寫能重現 bug 的測試（確認測試失敗）
2. 修復 bug
3. 確認測試通過
4. 在測試中加入註解，說明對應的 bug / issue 編號

```rust
#[tokio::test]
async fn test_duplicate_email_thread_matching_bug_42() {
    // Bug #42: 當同一封 Email 的 In-Reply-To 同時匹配多個 Thread 時，
    // 應選擇最近更新的 Thread，而非第一個匹配的。
    // ...
}
```

### 2.4 前端測試

| 層級 | 工具 | 範圍 |
|------|------|------|
| **單元測試** | Vitest | Composable、Utility、Store |
| **元件測試** | Vitest + Vue Test Utils | 單一元件的渲染與互動 |
| **E2E 測試** | Playwright（選用） | 關鍵使用者流程 |

---

## 3. Rust 最佳實踐

### 3.1 錯誤處理

- 使用 `thiserror` 定義模組專屬的錯誤類型
- 對外介面使用 `Result<T, ModuleError>`，禁止在業務邏輯中使用 `unwrap()` / `expect()`（測試程式碼除外）
- HTTP 層統一將錯誤轉換為 RFC 7807 Problem Details 格式

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

### 3.2 非同步與並行

- 使用 Tokio runtime，避免在 async 函式中使用阻塞操作
- CPU 密集型任務使用 `tokio::task::spawn_blocking`
- 共享狀態使用 `Arc<T>` + `tokio::sync::RwLock`（非 `std::sync::Mutex`）

### 3.3 專案結構

```
src/
  main.rs              # 入口點、啟動邏輯
  lib.rs               # 模組匯出
  config.rs            # 環境變數載入與驗證
  modules/
    auth/
      mod.rs           # 模組公開介面（trait）
      service.rs       # 業務邏輯實作
      repository.rs    # 資料庫存取
      error.rs         # 模組專屬錯誤
      tests.rs         # 單元測試
    core/
      ...
  api/
    routes/            # HTTP handler（thin layer）
    middleware/         # 認證、追蹤等中介層
    extractors/        # 自訂 Axum extractor
    error.rs           # HTTP 錯誤回應轉換
```

### 3.4 依賴注入

模組間依賴透過 trait 注入，便於測試與未來拆分：

```rust
pub struct AiModule {
    core: Arc<dyn CoreService>,
    conversation: Arc<dyn ConversationService>,
    tools: Arc<dyn ToolService>,
    llm: Arc<dyn LlmProvider>,
}
```

### 3.5 資料庫存取

- 使用 `sqlx` 搭配 compile-time query checking（`sqlx::query!` / `sqlx::query_as!`）
- Migration 使用 `sqlx-cli`，所有 schema 變更透過 migration 檔案管理
- 禁止跨模組 JOIN（遵循模組邊界）

### 3.6 sqlx 離線模式（Offline Mode）

專案預設啟用 `SQLX_OFFLINE=true`（設定於 `.cargo/config.toml`），編譯時不需要連線 PostgreSQL。sqlx proc macro 改為讀取 `.sqlx/` 目錄中的快取檔案進行類型驗證。

#### 日常開發

不需要 `DATABASE_URL` 即可執行 `cargo check`、`cargo clippy`、`cargo build`：

```bash
cargo clippy -- -D warnings   # 使用 .sqlx/ 快取，無需 DB
cargo build                    # 同上
```

#### 更新 `.sqlx/` 快取

當新增或修改 `sqlx::query!` / `sqlx::query_as!` 巨集中的 SQL 時，必須更新離線快取：

```bash
# 需要一個可連線的 PostgreSQL（Docker 或其他）
DATABASE_URL=postgres://confops:devpassword@localhost:5432/confops_dev \
  cargo xtask sqlx-prepare
```

此指令會自動執行 migration 並重新產生 `.sqlx/` 快取。更新後須將 `.sqlx/` 目錄的變更一併提交至 Git。

#### CI 驗證

使用 `--check` 旗標驗證快取是否與程式碼同步：

```bash
DATABASE_URL=... cargo xtask sqlx-prepare --check
```

#### 注意事項

- `.sqlx/` 目錄必須提交至 Git（已排除在 `.gitignore` 之外）
- `cargo test`（整合測試）使用 `postgresql_embedded` 自動管理 PostgreSQL，無需手動安裝或啟動 PostgreSQL 服務
- 若快取過期（SQL 有變更但未更新快取），編譯會失敗並提示類型不匹配

---

## 4. Vue / TypeScript 最佳實踐

### 4.1 TypeScript 嚴格模式

`tsconfig.json` 必須啟用 `strict` 模式：

```json
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "exactOptionalPropertyTypes": true
  }
}
```

所有程式碼修改必須通過 `vue-tsc --noEmit` 類型檢查。

### 4.2 元件規範

- 使用 `<script setup lang="ts">` Composition API
- Props 使用 `defineProps<T>()` 搭配 TypeScript 類型定義
- Emits 使用 `defineEmits<T>()` 搭配類型定義
- 複雜邏輯抽為 Composable（`use*.ts`）
- **區塊順序固定為 `<script>` → `<template>` → `<style>`**

```vue
<script setup lang="ts">
import type { Task } from '@/api/types'

const props = defineProps<{
  task: Task
  readonly: boolean
}>()

const emit = defineEmits<{
  update: [task: Task]
  delete: [taskId: string]
}>()
</script>

<template>
  <div>{{ props.task.title }}</div>
</template>

<style scoped>
/* ... */
</style>
```

### 4.3 狀態管理

- 使用 Pinia 管理全域狀態
- Store 中的非同步操作使用明確的 loading / error 狀態
- 避免在元件中直接操作 API，統一透過 Store 或 Composable

### 4.4 Lint 與格式化

- ESLint：使用 `@antfu/eslint-config` 或等效的嚴格配置
- Prettier（若未整合至 ESLint）：統一程式碼格式
- 所有 lint 規則須零警告

---

## 5. 前後端契約：Rust → OpenAPI → TypeScript

Rust 手寫類型（含 utoipa 註解）為 runtime source of truth，透過 utoipa 自動匯出 OpenAPI spec，前端再從 spec 生成 TypeScript 類型。

### 5.1 後端（Rust）

API request/response 類型直接定義在 route handler 檔案中，加上 utoipa 註解：

```rust
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    pub id: Uuid,
    pub title: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/tasks/{id}",
    tag = "tasks",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Task details", body = TaskResponse),
        (status = 404, description = "Not found", body = ProblemDetails)
    )
)]
pub async fn get_task(/* ... */) -> Result<Json<TaskResponse>, ProblemDetails> {
    // ...
}
```

所有 handler 須在 `src/api/openapi.rs` 的 `ApiDoc` 中註冊。

### 5.2 前端（TypeScript）

使用 `openapi-typescript` 從 utoipa 匯出的 spec 生成 TypeScript 類型：

```bash
# 從 utoipa 匯出的 OpenAPI spec 生成 TypeScript 類型
npx openapi-typescript ../docs/api/openapi-generated.yaml -o src/api/schema.d.ts
```

搭配 `openapi-fetch` 實現類型安全的 API 呼叫：

```typescript
import createClient from 'openapi-fetch'
import type { paths } from '@/api/schema'

const client = createClient<paths>({ baseUrl: '/api/v1' })

// 完全類型安全：路徑、參數、請求體、回應體皆由 spec 推導
const { data, error } = await client.GET('/projects/{projectId}/tasks', {
  params: { path: { projectId } },
})
```

### 5.3 契約同步流程

```
Rust 類型（utoipa 註解）
    ├── cargo xtask generate-openapi     → docs/api/openapi-generated.yaml
    └── npx openapi-typescript           → 前端 TypeScript 類型更新
```

- Rust 手寫類型（含 utoipa 註解）為唯一真相來源（Single Source of Truth）
- API request/response 類型直接在 route handler 檔案定義並加上 utoipa 註解
- CI 中驗證匯出的 spec 是否與程式碼同步

### 5.4 設計參考文件

`docs/api/openapi.yaml` 及 `docs/api/` 目錄下的原始設計稿保留為設計參考，不再用於程式碼生成。實際的 OpenAPI spec 由 utoipa 從 Rust 類型匯出至 `docs/api/openapi-generated.yaml`。

### 5.5 API 回應序列化注意事項

當資料庫 JSONB 欄位（如 `messages.content`）與 API 回應 Schema 使用 `discriminator`（辨別器）時，後端在序列化回應時必須根據資料庫的對應欄位（如 `source_type`）動態注入 `type` 值至 JSONB 內容中。例如 `MessageResponse.content` 的 `type` 欄位不儲存於資料庫，而是在 API handler 或 serializer 層根據 `messages.source_type` 映射注入（`member` → `"member"`、`email_inbound` → `"email_inbound"` 等），確保前端依據 OpenAPI discriminator 正確解析。

---

## 6. CI Pipeline

```yaml
# .github/workflows/ci.yml（概念）
jobs:
  backend:
    services:
      postgres:  # cargo test 和 sqlx-prepare --check 需要真實 DB
        image: postgres:16
        env:
          POSTGRES_DB: confops_dev
          POSTGRES_USER: confops
          POSTGRES_PASSWORD: devpassword
    steps:
      - cargo fmt -- --check
      - cargo clippy -- -D warnings    # 使用 .sqlx/ 離線快取，不需 DB
      - cargo test                      # 需要 DB（整合測試）
      # 驗證 .sqlx/ 離線快取是否與程式碼同步
      - DATABASE_URL=... cargo xtask sqlx-prepare --check
      # 驗證 utoipa 匯出的 OpenAPI spec 是否與程式碼同步
      - cargo xtask generate-openapi --check

  frontend:
    steps:
      - pnpm install
      - pnpm run lint
      - pnpm run typecheck
      - pnpm run test
      # 驗證生成的 API 類型是否與 spec 同步
      - npx openapi-typescript docs/api/openapi-generated.yaml -o src/api/schema.d.ts
      - git diff --exit-code src/api/schema.d.ts

  api-spec:
    steps:
      # 驗證 OpenAPI spec 設計稿格式正確
      - npx @redocly/cli lint docs/api/openapi.yaml
```

---

## 7. `X-Request-ID` 請求追蹤標準

所有 HTTP 請求應攜帶 `X-Request-ID` header（UUID v4），用於跨服務的請求關聯與追蹤：

- 若客戶端提供 `X-Request-ID`，伺服器沿用該值
- 若客戶端未提供，伺服器自動生成 UUID v4
- 該 ID 貫穿日誌（`tracing` span 的 `request_id` 欄位）、分散式追蹤（OpenTelemetry）、錯誤回應（RFC 7807 Problem Details 的 `instance` 或擴展欄位），用於跨模組的請求關聯與問題排查

---

## 8. Git 工作流程

### 分支策略

- `main`：穩定分支，所有合併需通過 CI 與 Code Review
- `feature/*`：功能分支，從 `main` 分出
- `fix/*`：Bug 修復分支

### Commit 訊息格式

採用 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

常用 type：`feat`, `fix`, `refactor`, `test`, `docs`, `chore`

常用 scope：`auth`, `core`, `conversation`, `ai`, `tools`, `email`, `notifications`, `storage`, `audit`, `frontend`

範例：

```
feat(ai): add Gemini Context Caching support

Implement GeminiCacheManager to cache system prompts and memory chains
across multiple AI calls for the same task, reducing latency and cost.
```

---

## 9. Polymorphic Foreign Key 驗證規則

以下資料表使用多態外鍵模式，無法在資料庫層級強制 FK 約束：

| 資料表 | 欄位組合 | 合法類型值 |
|--------|---------|-----------|
| `memories` | `(scope_type, scope_id)` | `account`, `organization`, `project`, `member_tag`, `task_template`, `task` |
| `library_documents` | `(scope_type, scope_id)` | `organization`, `project` |
| `tool_configs` | `(scope_type, scope_id)` | `organization`, `project` |
| `audit_logs` | `(resource_type, resource_id)` | `task`, `todo`, `message`, `memory`, `data_entry`, `tool_config`, ... |

### 驗證要求

1. **應用層驗證**：所有對 polymorphic FK 的寫入操作，必須在 repository 層驗證 `scope_id` / `resource_id` 指向的實體確實存在且未被軟刪除，並驗證類型值匹配
2. **CHECK 約束**：在資料庫層使用 `CHECK` 約束限制合法的類型值（如 `CHECK (scope_type IN ('account', 'organization', 'project', 'member_tag', 'task_template', 'task'))`）
3. **整合測試覆蓋**：每個使用 polymorphic FK 的 repository 必須包含以下測試案例：
   - 各合法類型值搭配有效 ID 的 CRUD 操作
   - 不存在的 `scope_id` / `resource_id` 應回傳錯誤
   - 不合法的類型值應回傳錯誤
   - scope 實體被軟刪除後，關聯資料的查詢行為符合預期
4. **查詢安全**：查詢時必須同時過濾類型與 ID 欄位，避免跨類型的資料洩漏

---

## 10. Soft Delete 統一規範

所有支援軟刪除的資料表遵循統一模式：

### 適用範圍

| 使用軟刪除的資料表 | 不使用軟刪除的資料表（原因） |
|-------------------|--------------------------|
| `accounts`, `organizations`, `projects`, `members`, `contacts`, `member_tags`, `task_templates`, `todo_templates`, `data_schemas`, `tasks`, `todos`, `data_entries`, `memories`, `library_documents`, `tool_configs`, `webhooks`, `api_keys` | `messages`（append-only）, `email_messages`（write-once）, `audit_logs`（write-once）, `notifications`（僅標記已讀）, `webhook_event_logs`（狀態更新但不刪除） |

### 資料庫層

- 使用 `deleted_at TIMESTAMPTZ` 欄位，`NULL` 表示未刪除
- 不使用 boolean `is_deleted` 欄位（`deleted_at` 同時記錄刪除時間）
- 建立索引以優化查詢：`CREATE INDEX idx_<table>_deleted_at ON <table> (deleted_at)`

### 應用層

- 所有預設查詢自動過濾 `WHERE deleted_at IS NULL`
- 需查詢已刪除資料時，使用明確的 `include_deleted` 參數
- 軟刪除操作：`UPDATE ... SET deleted_at = NOW() WHERE id = $1`
- 恢復操作（如需要）：`UPDATE ... SET deleted_at = NULL WHERE id = $1`

### 級聯行為

軟刪除**不自動級聯**至子記錄，須在 Service 層以明確的事務（transaction）處理：

| 父實體刪除 | 子記錄處理 |
|-----------|-----------|
| 組織刪除 | 應用層同時軟刪除該組織下的所有專案及相關記錄 |
| 專案刪除 | 應用層同時軟刪除專案下的成員標籤、任務模板、任務等 |
| 任務刪除 | 應用層同時軟刪除任務下的待辦事項、資料列 |
| 成員標籤刪除 | 任務的 `owner_tag_id` 保留原值，查詢時透過 JOIN 判斷標籤狀態 |

### 資料清理

已軟刪除的記錄依保留政策定期硬刪除。清理策略由管理員設定，預設不自動清理。清理前須確認無其他記錄仍參照該記錄。

### 測試要求

每個支援軟刪除的 repository 必須包含以下測試：
- 軟刪除後預設查詢不返回該記錄
- 使用 `include_deleted` 參數可查詢到已刪除記錄
- `deleted_at` 時間戳正確記錄
- 級聯軟刪除場景：父實體刪除後子記錄的查詢行為正確
