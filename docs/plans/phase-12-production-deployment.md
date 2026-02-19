# Phase 12：生產部署與最佳化

**階段目標：** 完成生產部署準備，含 Docker 多階段建置、Caddy 反向代理、資料庫備份策略、效能最佳化、專案複製完善、端到端整合測試，使系統可安全上線運行。

**前置依賴：** Phase 11 完成

---

## 後端任務

### B-12.1 專案複製完善與資料遷移

**範圍：** 完善 Phase 2 的專案複製功能，實作完整的深複製邏輯。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 完善 `ProjectService.copy_project()`：
   - **複製項目**：
     - member_tags（含 external_task_creation 設定）
     - task_templates（含 todo_templates + data_schemas）
     - task_template_tags（標籤與模板的關聯）
     - memories（project / member_tag / task_template scope）
     - tool_configs（專案層級設定）
     - permission_settings
   - **不複製項目**：
     - members（重新邀請）
     - tasks（全新開始）
     - contacts（組織層級已共用）
     - data_entries（全新開始）
     - messages / todos / conversations
   - **ID 對應表**：複製時維護 old_id → new_id 對應，確保 task_template_tags 中的 tag_id 和 template_id 指向新建立的 member_tags 和 task_templates
3. 建立複製進度追蹤（大專案可能需時較長）
4. 複製後的專案狀態為 `preparing`

**涉及檔案：**
- `src/modules/core/project/copy.rs`
- `src/modules/core/project/service.rs`（擴展）

**測試要求：**
- 整合測試：完整專案複製（所有項目正確複製）
- 整合測試：ID 對應正確（內部引用指向新 ID）
- 整合測試：不複製項目確實未被複製
- 整合測試：複製後專案獨立（修改不影響來源專案）

**驗收標準：**
- [ ] 完整深複製邏輯正確
- [ ] 所有內部引用正確映射
- [ ] 來源與複製專案完全獨立
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-12.2 效能最佳化與安全加固

**範圍：** 資料庫查詢最佳化、API 限流、安全 headers、CORS 配置。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. **資料庫最佳化**：
   - 審查所有查詢計畫（EXPLAIN ANALYZE），確保使用正確索引
   - 新增缺失的複合索引
   - 連線池參數調優（min/max connections）
   - 大量資料的游標分頁效能驗證
3. **API 限流**（rate limiting）：
   - 全域限流：100 req/s per IP
   - 認證端點限流：10 req/min per IP（防暴力破解）
   - LLM 相關端點限流（避免濫用）
     - AI 建議端點（`/api/v1/tasks/{taskId}/suggestions/*`）：5 req/min per user（LLM 呼叫成本高昂）
   - 使用 `tower-http` 的 rate limiting middleware 或自建
4. **安全 Headers**（Axum middleware）：
   - `X-Content-Type-Options: nosniff`
   - `X-Frame-Options: DENY`
   - `X-XSS-Protection: 0`（現代瀏覽器由 CSP 取代）
   - `Strict-Transport-Security: max-age=31536000; includeSubDomains`
   - `Content-Security-Policy`（適當設定）
5. **CORS 配置**：
   - 從 `APP_CORS_ORIGINS` 環境變數讀取允許的來源
   - 僅允許必要的 HTTP methods 與 headers
6. **Input validation 加強**：
   - 所有 API 輸入長度限制
   - SQL injection 防護（sqlx 已處理）
   - XSS 防護（對話內容的 HTML sanitization）

**涉及檔案：**
- `src/api/middleware/rate_limit.rs`
- `src/api/middleware/security_headers.rs`
- `src/api/middleware/cors.rs`
- `src/config.rs`（擴展限流設定）

**測試要求：**
- 整合測試：限流正確觸發（超過閾值回傳 429）
- 整合測試：安全 headers 正確設定
- 整合測試：CORS 正確阻擋非法來源
- 效能測試：主要查詢在合理時間內完成

**驗收標準：**
- [ ] 限流機制正確
- [ ] 安全 headers 完整
- [ ] CORS 配置正確
- [ ] 主要查詢效能合理
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-12.1 前端建置最佳化與 PWA

**範圍：** 前端建置最佳化、PWA 支援、Service Worker。

**說明：**
1. **建置最佳化**：
   - Vite production build 配置（code splitting, tree shaking）
   - 懶載入路由（`() => import('...')`）
   - 圖片最佳化（壓縮、WebP 轉換）
   - Bundle 分析（`rollup-plugin-visualizer`）
2. **PWA 支援**（使用 `vite-plugin-pwa`）：
   - Service Worker 離線快取策略
   - App manifest（名稱、圖標、主題色）
   - 安裝提示
3. **Web Push 整合**：
   - Service Worker 接收 push notification
   - 通知點擊跳轉至對應頁面

**涉及檔案：**
- `frontend/vite.config.ts`（建置最佳化 + PWA）
- `frontend/public/manifest.json`
- `frontend/src/sw.ts`（Service Worker）

**測試要求：**
- 建置測試：production build 成功
- 建置測試：bundle size 在合理範圍

**驗收標準：**
- [ ] Production build 成功
- [ ] 路由懶載入
- [ ] PWA 可安裝
- [ ] Web Push 通知可接收
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 共享任務

### S-12.1 Docker 多階段建置與映像檔

**範圍：** 建立 Dockerfile（後端 + 前端合併）、建置腳本。

**說明：**
1. 建立 `Dockerfile`（多階段建置）：
   - Stage 1：Rust builder（`FROM rust:1.XX AS rust-builder`，編譯 release binary）
   - Stage 2：Node.js builder（`FROM node:20 AS frontend-builder`，前端 production build）
   - Stage 3：Runtime（`FROM debian:bookworm-slim`，複製 binary + 前端 dist 至 `/app/static`）
2. 建立 `Dockerfile.dev`（開發用，含 cargo-watch）
3. 建立 `.dockerignore`
4. 後端服務靜態檔案（前端 build output）由 Axum 直接提供

**涉及檔案：**
- `Dockerfile`
- `Dockerfile.dev`
- `.dockerignore`

**測試要求：**
- Docker build 成功
- 映像檔可正常啟動並回應 `/healthz`

**驗收標準：**
- [ ] Docker 映像檔可建置
- [ ] 映像檔大小合理（< 200MB）
- [ ] 應用可正常啟動

---

### S-12.2 生產 Docker Compose 與 Caddy 配置

**範圍：** 建立生產 Docker Compose、Caddyfile、部署腳本。

**說明：**
1. 建立 `docker-compose.prod.yml`（參考 `docs/system/02-deployment-architecture.md`）：
   - confops-app（主應用）
   - confops-worker（背景任務）
   - postgres（PostgreSQL 16）
   - caddy（反向代理 + 自動 HTTPS）
2. 建立 `Caddyfile`：
   - HTTPS 自動憑證
   - 反向代理至 confops-app
   - WebSocket 支援
3. 建立部署腳本 `scripts/deploy.sh`
4. 建立 `.env.prod.example`（生產環境變數範例）
5. 健康檢查端點不包含 Redis 檢查（系統不使用 Redis），若未來需要 Redis 再擴展

**涉及檔案：**
- `docker-compose.prod.yml`
- `Caddyfile`
- `scripts/deploy.sh`
- `.env.prod.example`

**測試要求：**
- `docker compose -f docker-compose.prod.yml config` 驗證配置正確
- 本地模擬生產環境啟動測試

**驗收標準：**
- [ ] 生產 Docker Compose 完整
- [ ] Caddy 反向代理配置正確
- [ ] 部署腳本可執行

---

### S-12.3 資料庫備份策略與還原測試

**範圍：** 建立資料庫與檔案備份腳本、定期排程、還原測試。

**說明：**
1. 建立 `scripts/backup-db.sh`（pg_dump + gzip + 保留 30 天）
2. 建立 `scripts/backup-files.sh`（rsync 同步檔案目錄）
3. 建立 `scripts/restore-db.sh`（從備份還原）
4. 建立 crontab 配置範例
5. 撰寫還原測試流程文件

**涉及檔案：**
- `scripts/backup-db.sh`
- `scripts/backup-files.sh`
- `scripts/restore-db.sh`
- `scripts/crontab.example`

**測試要求：**
- 備份腳本可成功執行
- 還原腳本可從備份完整還原

**驗收標準：**
- [ ] 資料庫備份可執行
- [ ] 檔案備份可執行
- [ ] 從備份還原後系統正常運作

---

## 階段交付物

完成 Phase 12 後，以下端到端流程可驗證：

1. **專案複製**：複製去年專案 → 結構完整繼承 → 成員重新邀請 → 開始新一輪籌備
2. **生產部署**：`docker compose -f docker-compose.prod.yml up -d` → 系統上線 → HTTPS 自動生效
3. **安全**：嘗試暴力登入 → 被限流阻擋 → 安全 headers 正確
4. **備份還原**：執行備份 → 模擬災難 → 從備份還原 → 系統恢復正常
5. **效能**：主要頁面載入 < 2 秒 → API 回應 < 200ms（P95）
6. **完整 E2E 流程**：
   - 建立組織 → 建立專案 → 邀請成員 → 建立標籤 → 定義模板
   - 建立任務 → 對話 → AI 建議 → 人類確認 → 工具執行
   - Email 收發 → 通知 → 資料表填寫 → Webhook 觸發
   - 審計日誌完整 → 指標正常 → 系統可靠運行
