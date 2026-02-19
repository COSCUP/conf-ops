# 部署架構

## 1. 問題描述

Conf-Ops 採用 Modular Monolith 架構，編譯為單一 Rust 二進位檔（參見 [01-service-decomposition.md](./01-service-decomposition.md)）。系統依賴 PostgreSQL、SMTP 郵件服務等外部元件。

我們的目標是建立**最簡單、最容易維護的部署方式**，適合小型團隊日常營運。避免過度工程化——先以單機部署為主，等真正需要時再考慮擴展。

## 2. 設計決策

### 核心原則

**最小化維運複雜度**。單機部署，全部使用 Docker Compose 管理。

### 環境策略

- **開發環境**：Docker Compose 啟動依賴服務（PostgreSQL、Mailhog），Rust 應用程式直接在本機以 `cargo run` 或 `cargo watch` 執行，獲得最快的編譯與除錯回饋。
- **生產環境**：單台 VPS/伺服器上以 Docker Compose 部署所有服務，包含 Rust 應用程式本身。
- **檔案儲存**：使用本機檔案系統（掛載 Docker Volume），不使用 S3。介面層抽象化，未來可無痛遷移至 S3。
- **資料庫**：單一 PostgreSQL 實例、單一資料庫。透過 `project_id` / `organization_id` 欄位進行列級（row-level）資料隔離。

### 考慮過的替代方案

| 方案 | 評估 | 結論 |
|------|------|------|
| **Kubernetes（生產）** | 自動擴展、滾動更新等能力強大，但學習曲線高、小規模部署過重 | 列為未來選項 |
| **S3 物件儲存** | 適合大規模檔案儲存，但小規模下增加不必要的維運開銷 | 列為未來選項，先用本機檔案系統 |
| **每專案獨立資料庫** | 資料隔離更徹底，但增加連線管理複雜度與維運負擔 | 列為未來考量，先用單一資料庫 |

## 3. 開發環境

### 3.1 服務組成

| 服務 | 映像檔 | 用途 | 連接埠 |
|------|--------|------|--------|
| **postgres** | `postgres:16` | 關聯式資料庫 | 5432 |
| **mailhog** | `mailhog/mailhog:latest` | SMTP 擷取與測試用 Web UI | 1025 (SMTP) / 8025 (UI) |

不需要 MinIO — 開發環境直接使用本機檔案系統儲存檔案。

### 3.2 Docker Compose 設定

```yaml
# docker-compose.dev.yml
services:
  postgres:
    image: postgres:16
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: confops_dev
      POSTGRES_USER: confops
      POSTGRES_PASSWORD: devpassword
    volumes:
      - pgdata:/var/lib/postgresql/data

  mailhog:
    image: mailhog/mailhog:latest
    ports:
      - "1025:1025"   # SMTP
      - "8025:8025"   # Web UI

volumes:
  pgdata:
```

### 3.3 開發環境架構圖

```mermaid
graph LR
    dev["Rust 應用程式<br/>cargo run / cargo watch"]
    pg["PostgreSQL 16<br/>:5432"]
    mailhog["MailHog<br/>SMTP :1025 / UI :8025"]
    fs["本機檔案系統<br/>./data/files"]

    dev --> pg
    dev --> mailhog
    dev --> fs

    browser["瀏覽器"] --> dev
    browser --> mailhog
```

### 3.4 開發環境設定

開發時使用 `.env` 檔案提供環境變數：

```env
# Database
DATABASE_URL=postgres://confops:devpassword@localhost:5432/confops_dev
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=2

# Auth
AUTH_JWT_SECRET=dev-secret-key
AUTH_JWT_ACCESS_EXPIRY=900
AUTH_REFRESH_EXPIRY=604800
AUTH_WEBAUTHN_RP_ID=localhost
AUTH_WEBAUTHN_RP_ORIGIN=http://localhost:3000

# Storage
STORAGE_BACKEND=local
STORAGE_BASE_PATH=./data/files

# Email
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USERNAME=
SMTP_PASSWORD=
SMTP_FROM=noreply@localhost
EMAIL_INBOUND_API_KEY=inbound-secret-key

# AI (Google Gemini)
LLM_API_URL=https://generativelanguage.googleapis.com/v1beta
LLM_API_KEY=AIza-dev-key
LLM_MODEL=gemini-2.0-flash

# App
APP_HOST=0.0.0.0
APP_PORT=8080
APP_LOG_LEVEL=debug,confops=trace
APP_CORS_ORIGINS=http://localhost:3000

# WebSocket
WS_HEARTBEAT_INTERVAL=30
WS_MAX_CONNECTIONS=1000
```

啟動方式：

```bash
# 啟動依賴服務
docker compose -f docker-compose.dev.yml up -d

# 啟動 Rust 應用程式（擇一）
cargo run
cargo watch -x run
```

## 4. 生產環境 — 單機 Docker Compose

### 4.1 架構總覽

```mermaid
graph TB
    subgraph Internet["外部網路"]
        users["使用者"]
        ext_mail["外部郵件伺服器"]
        llm["LLM API<br/>(Google Gemini)"]
    end

    subgraph Server["單機伺服器 (VPS)"]
        caddy["Caddy<br/>自動 HTTPS + WSS<br/>反向代理"]

        subgraph DockerCompose["Docker Compose"]
            app["confops-app<br/>Rust Binary<br/>:8080"]
            worker["confops-worker<br/>同一 Binary --worker<br/>背景任務"]
            pg["PostgreSQL 16<br/>:5432"]
        end

        fs["本機檔案儲存<br/>/data/confops/files"]
    end

    users -->|"HTTPS / WSS"| caddy
    caddy --> app
    app --> pg
    app --> fs
    worker --> pg
    worker --> fs
    worker --> llm
    app --> llm
    app --> ext_mail
    ext_mail --> app
```

### 4.2 元件說明

| 元件 | 說明 |
|------|------|
| **confops-app** | 主應用程式容器，提供 HTTP API、WebSocket、Email Webhook 收信等功能 |
| **confops-worker** | 同一二進位檔以 `--worker` 旗標啟動，負責背景任務（提醒排程、郵件重試、過期資料清理等） |
| **postgres** | PostgreSQL 16，資料持久化至 Docker Volume |
| **caddy** | 反向代理伺服器，自動透過 Let's Encrypt 取得與續期 HTTPS 憑證，支援 WebSocket 代理 |
| **本機檔案儲存** | 以 Docker Volume 掛載的本機目錄 `/data/confops/files`，存放使用者上傳的檔案 |

### 4.3 Docker Compose 設定

```yaml
# docker-compose.prod.yml
services:
  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
      - "443:443/udp"  # HTTP/3
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      confops-app:
        condition: service_healthy
    deploy:
      resources:
        limits:
          memory: 128M

  confops-app:
    image: ghcr.io/org/confops:latest
    restart: unless-stopped
    env_file: .env.prod
    ports:
      - "127.0.0.1:8080:8080"
    volumes:
      - file_storage:/data/files
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "1.0"

  # 重要：confops-worker 必須維持單一 replica。
  # 提醒排程器（Reminder Scheduler）依賴單一實例運行以避免重複觸發通知。
  # 若未來需要多 replica，須先啟用 PostgreSQL Advisory Lock 或
  # SELECT ... FOR UPDATE SKIP LOCKED 防重機制（參見 docs/system/10-notification-system.md）。
  confops-worker:
    image: ghcr.io/org/confops:latest
    restart: unless-stopped
    command: ["confops", "--worker"]
    env_file: .env.prod
    volumes:
      - file_storage:/data/files
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/healthz"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    deploy:
      replicas: 1  # 禁止調高，參見上方說明
      resources:
        limits:
          memory: 512M
          cpus: "0.5"

  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_DB: confops
      POSTGRES_USER: confops
      POSTGRES_PASSWORD_FILE: /run/secrets/db_password
    volumes:
      - pgdata:/var/lib/postgresql/data
    secrets:
      - db_password
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U confops"]
      interval: 10s
      timeout: 5s
      retries: 5
    deploy:
      resources:
        limits:
          memory: 1G

volumes:
  caddy_data:
  caddy_config:
  pgdata:
  file_storage:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /data/confops/files

secrets:
  db_password:
    file: ./secrets/db_password.txt
```

### 4.4 Caddy 反向代理設定

```caddyfile
# Caddyfile
api.confops.dev {
    # HTTP API 與 WebSocket 代理
    reverse_proxy confops-app:8080 {
        # WebSocket 支援（自動偵測 Upgrade header）
        flush_interval -1
    }

    # 存取日誌
    log {
        output stdout
        format json
    }
}
```

Caddy 會自動處理：
- Let's Encrypt 憑證申請與續期
- HTTP 自動重導向至 HTTPS
- WebSocket 連線代理（WSS）
- HTTP/2 與 HTTP/3 支援

## 5. CI/CD 流水線

### 5.1 Pipeline 總覽

```mermaid
graph LR
    push["Git Push / PR"]
    lint["Lint<br/>clippy + fmt"]
    test["Test<br/>cargo test"]
    build["Build Docker Image<br/>多階段建置"]
    push_img["Push to Registry<br/>ghcr.io"]
    deploy["Deploy<br/>SSH + docker compose"]

    push --> lint
    push --> test
    lint --> build
    test --> build
    build --> push_img
    push_img --> deploy
```

### 5.2 各階段說明

| 階段 | 觸發條件 | 說明 |
|------|----------|------|
| **Lint** | 所有 push / PR | `cargo clippy -- -D warnings` + `cargo fmt -- --check` |
| **Test** | 所有 push / PR | `cargo test`，包含單元測試與整合測試 |
| **Build** | lint + test 通過 (main branch) | 多階段 Docker 建置，產出最佳化映像檔 |
| **Push** | build 通過 | 推送映像檔至容器倉庫（ghcr.io 或 Docker Hub） |
| **Deploy** | push 完成 (main branch) | SSH 至伺服器執行 `docker compose pull && docker compose up -d` |
| **DB Migration** | 應用程式啟動時自動執行（sqlx migrate），或手動 `docker compose exec confops-app confops migrate` |

#### 資料庫遷移策略

**版本控制：**
- 遷移檔案儲存於 `migrations/` 目錄，隨原始碼版控
- 檔名格式：`{timestamp}_{description}.sql`（如 `20260101120000_add_api_keys.sql`）
- `sqlx` 使用 `_sqlx_migrations` 表追蹤已執行的遷移版本與雜湊值
- 禁止修改已部署的遷移檔案（雜湊值不符會導致啟動失敗）

**多 Replica 安全：**
- `sqlx migrate` 內部使用 PostgreSQL advisory lock（`pg_advisory_lock`）確保同一時間只有一個實例執行遷移
- 即使 `confops-app` 未來擴展為多 replica，各實例啟動時嘗試取得鎖，僅一個成功執行遷移，其餘等待完成後繼續啟動
- `confops-worker` 不執行遷移，僅依賴 `confops-app` 完成遷移

**Rollback 策略：**
- 每個遷移檔案應撰寫對應的 `down` 遷移（`migrations/{timestamp}_{description}.down.sql`）
- Rollback 操作須手動執行：`docker compose exec confops-app confops migrate revert`
- 破壞性遷移（刪欄位、改型別）須分兩階段部署：
  1. 第一版：新增欄位 / 停止讀取舊欄位（向後相容）
  2. 第二版：移除舊欄位（確認無回滾需求後）

**失敗處理：**
- 遷移失敗時，`confops-app` 容器啟動失敗，Docker healthcheck 偵測後不會接收流量
- 遷移以交易包裹（每個遷移檔案在單一 transaction 中執行），失敗時自動 rollback 該檔案的所有變更
- 遷移失敗觸發告警，維運人員須手動排查並修復後重新部署

### 5.3 Dockerfile

```dockerfile
# Build stage
FROM rust:1.82-bookworm AS builder
WORKDIR /app

# 先複製 Cargo 檔案，利用 Docker layer cache
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# 複製原始碼並建置
COPY src/ src/
COPY migrations/ migrations/
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/confops /usr/local/bin/confops
COPY --from=builder /app/migrations /app/migrations

# 建立檔案儲存目錄
RUN mkdir -p /data/files

EXPOSE 8080
ENTRYPOINT ["confops"]
```

### 5.4 部署腳本

```bash
#!/bin/bash
# deploy.sh — 在伺服器上執行
set -euo pipefail

cd /opt/confops

# 拉取最新映像檔
docker compose -f docker-compose.prod.yml pull

# 重啟服務（零停機：先啟動新容器再停舊容器）
docker compose -f docker-compose.prod.yml up -d --remove-orphans

# 執行資料庫遷移（若未設定自動遷移）
# docker compose -f docker-compose.prod.yml exec confops-app confops migrate

# 清理舊映像檔
docker image prune -f

echo "部署完成"
```

## 6. 環境設定

### 6.1 環境變數一覽

所有設定透過環境變數管理，以下按模組分組。

#### Database

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `DATABASE_URL` | PostgreSQL 連線字串 | `postgres://confops:password@postgres:5432/confops` |
| `DATABASE_MAX_CONNECTIONS` | 連線池最大連線數 | `20` |
| `DATABASE_MIN_CONNECTIONS` | 連線池最小連線數 | `5` |

#### Auth

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `AUTH_JWT_SECRET` | JWT 簽發密鑰 | `your-secret-key` |
| `AUTH_JWT_ACCESS_EXPIRY` | Access Token 過期秒數 | `900` |
| `AUTH_REFRESH_EXPIRY` | Refresh Token 過期秒數 | `604800` |
| `AUTH_WEBAUTHN_RP_ID` | WebAuthn Relying Party ID | `confops.dev` |
| `AUTH_WEBAUTHN_RP_ORIGIN` | WebAuthn Relying Party Origin | `https://confops.dev` |
| `AUTH_MAGIC_LINK_EXPIRY` | Magic Link Token 有效期（秒） | `900` |
| `AUTH_MAGIC_LINK_BASE_URL` | Magic Link 前端回呼 URL | `https://confops.dev/auth/verify` |
| `AUTH_RATE_LIMIT_MAGIC_LINK` | Magic Link 頻率限制 | `3/hour` |

#### Storage

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `STORAGE_BACKEND` | 儲存後端類型（`local`，未來支援 `s3`） | `local` |
| `STORAGE_BASE_PATH` | 本機檔案儲存根目錄 | `/data/files` |

> **未來擴展**：當 `STORAGE_BACKEND=s3` 時，將額外讀取 `S3_ENDPOINT`、`S3_BUCKET`、`S3_ACCESS_KEY`、`S3_SECRET_KEY`、`S3_REGION` 等環境變數。程式碼中的儲存介面（Storage trait）已抽象化，切換後端無需修改業務邏輯。

#### Email

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `SMTP_HOST` | SMTP 伺服器位址 | `smtp.example.com` |
| `SMTP_PORT` | SMTP 埠號 | `587` |
| `SMTP_USERNAME` | SMTP 帳號 | `noreply@confops.dev` |
| `SMTP_PASSWORD` | SMTP 密碼 | `password` |
| `SMTP_FROM` | 寄件人地址 | `noreply@confops.dev` |
| `EMAIL_INBOUND_API_KEY` | 收信轉發函式認證 token | （必填） |

#### AI

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `LLM_API_URL` | Gemini API 端點 | `https://generativelanguage.googleapis.com/v1beta` |
| `LLM_API_KEY` | Gemini API 金鑰 | `AIza...` |
| `LLM_MODEL` | 預設 LLM 模型名稱 | `gemini-2.0-flash` |

#### App

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `APP_HOST` | HTTP 伺服器監聽位址 | `0.0.0.0` |
| `APP_PORT` | HTTP 伺服器監聽埠號 | `8080` |
| `APP_LOG_LEVEL` | 日誌等級 | `info,confops=debug` |
| `APP_CORS_ORIGINS` | 允許的 CORS 來源（CORS 詳細策略見下方說明） | `https://confops.dev` |

**CORS 策略詳細設定：**

| 項目 | 設定 |
|------|------|
| Allowed Origins | 由 `APP_CORS_ORIGINS` 環境變數指定 |
| Allowed Methods | `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `OPTIONS` |
| Allowed Headers | `Content-Type`, `Authorization`, `X-API-Key`, `X-Request-ID` |
| Allow Credentials | `true` |
| Max Age | `86400`（24 小時） |

#### WebSocket (CRDT)

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `CRDT_WS_HEARTBEAT_INTERVAL_SECS` | WebSocket 心跳間隔（秒） | `30` |
| `CRDT_WS_IDLE_TIMEOUT_SECS` | WebSocket 閒置超時（秒） | `300` |
| `CRDT_WS_MAX_CONNECTIONS_PER_TASK` | 每個任務最大 WebSocket 連線數 | `50` |
| `CRDT_AWARENESS_TIMEOUT_SECS` | Awareness 狀態超時（秒） | `60` |

#### Web Push

| 環境變數 | 說明 | 範例 |
|----------|------|------|
| `WEB_PUSH_VAPID_PRIVATE_KEY` | VAPID 私鑰 | `base64-encoded-key` |
| `WEB_PUSH_VAPID_PUBLIC_KEY` | VAPID 公鑰 | `base64-encoded-key` |

### 6.2 環境區分

| 設定項 | 開發環境 | 生產環境 |
|--------|----------|----------|
| `DATABASE_URL` | localhost:5432 | postgres:5432（Docker 內部網路） |
| `STORAGE_BACKEND` | `local` | `local` |
| `STORAGE_BASE_PATH` | `./data/files` | `/data/files` |
| `SMTP_HOST` | localhost:1025 (MailHog) | smtp.example.com |
| `APP_LOG_LEVEL` | `debug,confops=trace` | `info,confops=debug` |

## 7. 全域速率限制策略

系統對 API 請求實施多層速率限制，防止濫用與過載：

| 限制層級 | 限制值 | 環境變數 |
|---------|--------|---------|
| Per IP | 100 req/min | `RATE_LIMIT_PER_IP` |
| Per User（已認證） | 300 req/min | `RATE_LIMIT_PER_USER` |

**敏感端點個別限制（Per endpoint overrides）：**
- 認證相關端點（Magic Link、Passkey 驗證）：更嚴格的頻率限制（參考 `AUTH_RATE_LIMIT_MAGIC_LINK`）
- AI Pipeline 端點（手動觸發建議）：受限於 LLM API 呼叫成本，額外限制每任務每分鐘觸發次數

速率限制回應遵循 RFC 7807 Problem Details 格式，HTTP 狀態碼 429，回應中包含 `Retry-After` header。

---

## 8. 資料備份策略

### 8.1 PostgreSQL 備份

使用 cron job 定期執行 `pg_dump`，將備份檔傳送至獨立磁碟或遠端備份位置：

```bash
#!/bin/bash
# backup-db.sh — 加入 crontab，例如每日凌晨 3:00 執行
set -euo pipefail

BACKUP_DIR="/data/backups/postgres"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="${BACKUP_DIR}/confops_${TIMESTAMP}.sql.gz"

mkdir -p "${BACKUP_DIR}"

# 使用 docker compose exec 執行 pg_dump
docker compose -f /opt/confops/docker-compose.prod.yml exec -T postgres \
    pg_dump -U confops confops | gzip > "${BACKUP_FILE}"

# 保留最近 30 天的備份
find "${BACKUP_DIR}" -name "*.sql.gz" -mtime +30 -delete

# 同步至遠端備份位置（選用）
# rsync -az "${BACKUP_DIR}/" backup-server:/backups/confops/postgres/
# rclone sync "${BACKUP_DIR}" remote:confops-backups/postgres/

echo "資料庫備份完成: ${BACKUP_FILE}"
```

Crontab 設定：

```
# 每日凌晨 3:00 執行資料庫備份
0 3 * * * /opt/confops/scripts/backup-db.sh >> /var/log/confops-backup.log 2>&1
```

### 8.2 檔案儲存備份

使用 `rsync` 或 `rclone` 將本機檔案目錄同步至備份位置：

```bash
#!/bin/bash
# backup-files.sh
set -euo pipefail

# rsync 至備份伺服器
rsync -az --delete /data/confops/files/ backup-server:/backups/confops/files/

# 或使用 rclone 同步至雲端儲存
# rclone sync /data/confops/files remote:confops-backups/files/
```

### 8.3 備份驗證

定期（建議每月一次）從備份還原至測試環境，驗證備份的完整性與可還原性。

## 9. 監控與健康檢查

### 9.1 健康檢查端點

| 端點 | 用途 | 檢查內容 |
|------|------|----------|
| `GET /healthz` | Liveness 檢查 | 程序存活確認（固定回傳 200） |
| `GET /readyz` | Readiness 檢查 | 檢查資料庫連線是否正常 |
| `GET /metrics` | Prometheus 指標（選用） | 暴露應用程式指標供 Prometheus 抓取 |

Docker Compose 中已設定 healthcheck，Docker 會自動根據健康檢查結果重啟不健康的容器。

### 9.2 基本監控方案

採用簡單、低維護成本的監控方式：

- **Uptime 監控**：使用 [Uptime Kuma](https://github.com/louislam/uptime-kuma) 或類似工具，定期檢查 `/healthz` 端點，異常時發送通知（Email / Telegram / Slack）。
- **Docker 健康檢查**：利用 Docker Compose 內建的 healthcheck 機制，搭配 `restart: unless-stopped` 自動重啟故障容器。
- **資源監控**：透過 `docker stats` 或簡單的系統監控工具觀察 CPU / 記憶體 / 磁碟使用狀況。

### 9.3 日誌管理

- 使用結構化 JSON 日誌格式（`tracing` + `tracing-subscriber`），輸出至 stdout。
- 日誌等級透過 `APP_LOG_LEVEL` 環境變數控制。
- 由 Docker 的 logging driver 收集日誌。預設使用 `json-file` driver，可依需求切換至 `syslog` 或 `fluentd`。
- 建議設定 Docker logging driver 的日誌輪替，避免磁碟空間耗盡：

```yaml
# 在 docker-compose.prod.yml 的各服務中加入
logging:
  driver: json-file
  options:
    max-size: "50m"
    max-file: "5"
```

### 9.4 Prometheus 指標（選用）

若需要更詳細的指標監控，可啟用 `/metrics` 端點，搭配 Prometheus + Grafana：

- HTTP 請求速率、延遲、錯誤率
- WebSocket 連線數
- 資料庫連線池使用率
- 背景任務佇列深度
- LLM API 呼叫延遲與錯誤率

此為選用功能，初期可先以基本監控方案運行，待需求增長再加入。

## 10. 資料保留策略

系統對各類資料定義統一的保留與清理策略：

| 資料類型 | 保留策略 | 環境變數 |
|---------|---------|---------|
| 審計日誌 (audit_logs) | 預設 365 天，月分區 | `AUDIT_RETENTION_DAYS` |
| 孤立檔案 | 7 天寬限期後清理 | — |
| 通知 (notifications) | 定期硬刪除已讀超過 90 天的通知 | `NOTIFICATION_RETENTION_DAYS` |
| CRDT 操作日誌 | 快照後可壓縮歷史操作 | — |
| AI Pipeline 事件 | 完成後保留 30 天 | `AI_EVENT_RETENTION_DAYS` |

清理作業由 `confops-worker` 背景任務定期執行。

---

## 11. 擴展性考量與未來方向

### 11.1 當前架構的承載能力

目前的單機 Docker Compose 部署方式，足以應對中小規模的使用場景：

- 單一研討會（如 COSCUP 規模）的籌備團隊協作
- 數十至數百位活躍使用者同時操作
- 適度的檔案上傳與 API 請求量

### 11.2 何時需要考慮擴展

當出現以下情況時，應評估是否需要升級部署架構：

- 大量併發 WebSocket 連線導致單機效能瓶頸
- 多個大型組織同時使用，資料量與請求量顯著增長
- 需要高可用性（HA），不能接受單機故障導致的服務中斷
- 檔案儲存量超出單機磁碟容量

### 11.3 未來擴展選項

| 方向 | 說明 |
|------|------|
| **Kubernetes 部署** | 遷移至 K8s 叢集，利用 HPA 自動擴展、滾動更新、自我修復等能力 |
| **S3 檔案儲存** | 將 `STORAGE_BACKEND` 切換為 `s3`，使用 AWS S3 或 MinIO，不受單機磁碟限制 |
| **資料庫 Read Replica** | 加入 PostgreSQL 讀取副本，分擔查詢負載 |
| **每專案獨立資料庫** | 為大型組織提供獨立資料庫實例，實現更徹底的資料隔離 |
| **CDN 靜態資源** | 前端資源透過 CDN 分發，降低伺服器負載 |
| **PgBouncer 連線池** | 在應用與資料庫之間加入連線池代理，提升連線效率 |

Modular Monolith 架構搭配抽象化的介面（如 Storage trait），使得上述各項遷移都能相對平順地進行，無需大幅重構業務邏輯。
