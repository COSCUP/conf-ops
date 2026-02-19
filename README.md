# Conf-Ops

AI 輔助的研討會/活動專案管理系統，專為重複性高、跨組協作密集的大型活動（如 [COSCUP](https://coscup.org/)）設計。

系統以「任務對話」為核心操作介面，結合 AI 建議與人類決策，讓團隊在累積組織記憶的同時高效完成工作。

## 特色

- **AI 輔助，人類決策** — AI 僅提供建議，所有寫入操作皆須人類確認
- **對話即記錄** — 任務對話同時作為操作介面與完整的審計追蹤
- **多層級記憶繼承** — 從帳號到任務，經驗沿繼承鏈層層傳遞，持續改善 AI 建議品質
- **即時多人協作** — 基於 CRDT（yrs）的無衝突即時協作編輯
- **隱私優先** — AI 僅接收欄位結構與佔位符，不接觸實際資料值
- **工具可擴充（MCP）** — 支援自行接入第三方 MCP Server，統一格式無縫擴充

## 技術棧

| 層級 | 技術 |
|------|------|
| 後端 | Rust（Tokio + Axum + sqlx） |
| 資料庫 | PostgreSQL 16 |
| 前端 | Vue 3 + TypeScript（strict mode） |
| 即時協作 | CRDT（yrs）+ WebSocket |
| AI | Google Gemini API |
| 部署 | Docker Compose + Caddy |

## 架構

Modular Monolith — 單一 Rust binary，內含 9 個模組：

```
auth · core · conversation · ai · tools · email · notifications · storage · audit
```

前後端透過 OpenAPI 3.1 spec 作為契約，自動生成雙端類型定義。

## 資料庫

### 設計原則

- **主鍵**：全表統一使用 UUID v7（應用層產生），具時間可排序性，對 B-tree 索引友善
- **時間戳**：`TIMESTAMPTZ` 一律存 UTC，標準欄位為 `created_at`、`updated_at`、`deleted_at`
- **軟刪除**：多數資料表採 `deleted_at IS NULL` 過濾模式；少數 append-only 表（如 `messages`、`audit_logs`）例外
- **JSONB**：彈性結構化欄位（個人資料、工具設定、資料表定義等）由應用層 Rust struct + serde 驗證，搭配 GIN 索引
- **CRDT**：`crdt_operations` 表儲存 yrs 二進位操作日誌，與主表雙寫確保即時協作的最終一致性
- **查詢安全**：使用 sqlx compile-time query checking，禁止跨模組 JOIN

### 資料表總覽

```
accounts              # 帳號與個人資料
organizations         # 組織
projects              # 專案（屬於組織）
members               # 成員（帳號 × 專案）
contacts              # 外部聯絡人
member_tags           # 成員標籤（角色/組別）
task_templates        # 任務模板
tasks                 # 任務實例
messages              # 任務對話訊息（append-only）
todos                 # 待辦事項
data_schemas          # 資料表定義
data_entries          # 資料列
tool_configs          # MCP 工具設定
memories              # 多層級記憶
library_documents     # 記憶庫文件
audit_logs            # 稽核日誌（write-once）
notifications         # 通知
email_threads         # Email 執行緒追蹤
webhooks              # Webhook 設定
api_keys              # 專案 API 金鑰
files                 # 檔案後設資料
crdt_operations       # CRDT 操作日誌
ai_pipeline_events    # AI Pipeline 事件佇列
```

完整 ER 圖與欄位定義請參閱 [資料模型文件](docs/data-model/)。

### Migration

Migration 檔案位於 `migrations/`，使用 sqlx 管理：

```bash
# 執行 migration
make migrate
```

## 環境需求

- Rust 1.75+
- Node.js 20.19+ 或 22.12+
- pnpm
- Docker & Docker Compose

## 快速開始

### 1. 啟動開發服務

```bash
# 啟動 PostgreSQL + MailHog
make dev-up

# 執行資料庫 migration
make migrate
```

開發服務：
- PostgreSQL：`localhost:5432`（DB: `confops_dev`、User: `confops`）
- MailHog Web UI：http://localhost:8025（SMTP: `localhost:1025`）

### 2. 設定環境變數

```bash
cp .env.example .env
# 編輯 .env 填入必要設定
```

### 3. 啟動後端

```bash
cargo run
```

### 4. 啟動前端

```bash
cd frontend
pnpm install
pnpm run dev
```

## 開發指令

```bash
make help          # 顯示所有可用指令

# Lint
make lint          # 執行所有 lint（後端 + 前端 + OpenAPI spec）
make lint-backend  # cargo clippy + fmt check
make lint-frontend # pnpm run lint

# 測試
make test          # 執行所有測試
make test-backend  # cargo test（DB 測試使用 postgresql_embedded，無需額外設定）
make test-frontend # pnpm run test（Vitest + Vue Test Utils）

# API 文件
make bundle        # 打包 OpenAPI spec 為單一檔案
make build-docs    # 產生 HTML API 文件
make check-refs    # 跨文件一致性檢查
```

## 專案結構

```
conf-ops/
├── src/                    # Rust 後端原始碼
│   ├── api/                # HTTP 路由與中介層
│   ├── modules/            # 業務模組（auth, core, conversation...）
│   ├── config.rs           # 設定管理
│   ├── db.rs               # 資料庫連線
│   └── main.rs             # 進入點
├── frontend/               # Vue 3 前端
│   └── src/
│       ├── api/            # API 客戶端與生成類型
│       ├── composables/    # Vue composables
│       ├── stores/         # Pinia stores
│       ├── views/          # 頁面元件
│       └── router/         # 路由設定
├── migrations/             # SQL migration 檔案
├── tests/                  # 後端整合測試
├── docs/
│   ├── architecture.md     # 功能架構規格
│   ├── api/                # OpenAPI 3.1 規格
│   ├── data-model/         # 資料模型定義（19 個實體）
│   └── system/             # 系統架構文件
├── docker-compose.dev.yml  # 開發環境容器
└── Makefile                # 開發指令集
```

## 文件

- [功能架構規格](docs/architecture.md)
- [開發準則](docs/development-guidelines.md)
- [API 規格](docs/api/openapi.yaml)
- [資料模型](docs/data-model/)
- [系統架構](docs/system/)

## 授權

此專案尚未選定授權條款。
