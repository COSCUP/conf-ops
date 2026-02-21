# Conf-Ops — Claude Code 開發指引

## 專案概述

AI 輔助的研討會/活動專案管理系統（以 COSCUP 為設計對象）。
Modular Monolith 架構，單一 Rust binary，9 個內部模組。

## 技術棧

- 後端：Rust（Tokio + Axum + sqlx + PostgreSQL）
- 前端：Vue.js + TypeScript（strict mode）
- 即時協作：CRDT（yrs）+ WebSocket
- AI：Google Gemini API
- 部署：Docker Compose + Caddy

## 開發準則

完整開發準則詳見 `docs/development-guidelines.md`，以下為必須遵循的核心規則。

### TDD 開發流程

所有功能開發與 bug 修復採用 TDD：Red → Green → Refactor。

### 提交前品質要求（全部必須通過）

**Lint 修復原則：** 禁止以 `#[allow(...)]`、`// eslint-disable`、`@ts-ignore` 等忽略方式修復 lint 問題。必須使用時先詢問確認。

**後端：**
- `cargo clippy -- -D warnings` — 零錯誤、零警告（含 info 層級，使用 `.sqlx/` 離線快取，不需 DB）
- `cargo fmt -- --check` — 格式檢查通過
- `cargo test` — 所有測試通過（使用 `postgresql_embedded` 自動管理 PostgreSQL，無需手動啟動）
- 若修改了 SQL query：`DATABASE_URL=... cargo xtask sqlx-prepare` 更新 `.sqlx/` 快取並提交

**前端：**
- `pnpm run lint` — 零錯誤、零警告
- `pnpm run typecheck` — TypeScript strict 模式類型檢查通過
- `pnpm run test` — 所有測試通過

### 測試要求

- 測試分三層：單元測試、API 測試、整合測試
- 每個 bug 修復必須附帶迴歸測試
- DB 測試禁止 mock，使用 `postgresql_embedded`（自動下載並管理嵌入式 PostgreSQL，無需預先安裝）
- 前端測試使用 Vitest + Vue Test Utils

### Rust 規範

- 錯誤處理使用 `thiserror`，業務邏輯禁止 `unwrap()` / `expect()`
- 非同步使用 Tokio，阻塞操作用 `spawn_blocking`
- 模組間依賴透過 trait 注入（`Arc<dyn Trait>`）
- 資料庫使用 `sqlx` compile-time query checking，禁止跨模組 JOIN
- HTTP 錯誤回應使用 RFC 7807 Problem Details
- sqlx 離線模式：預設 `SQLX_OFFLINE=true`，編譯不需 DB。修改 SQL 後須執行 `DATABASE_URL=... cargo xtask sqlx-prepare` 更新 `.sqlx/` 快取並提交

### TypeScript / Vue 規範

- `tsconfig.json` 啟用 `strict: true` + `noUncheckedIndexedAccess`
- 使用 `<script setup lang="ts">` Composition API
- 區塊順序固定：`<script>` → `<template>` → `<style>`
- Props / Emits 使用 TypeScript 類型定義
- 狀態管理使用 Pinia

### 前後端契約

- Rust 手寫類型（含 utoipa 註解）為 runtime source of truth
- `cargo xtask generate-openapi` 從 utoipa 匯出 OpenAPI spec 至 `docs/api/openapi-generated.yaml`
- 前端類型：`npx openapi-typescript` 從 `openapi-generated.yaml` 生成至 `src/api/schema.d.ts`
- API request/response 類型直接在 route handler 檔案定義並加上 utoipa 註解
- `docs/api/openapi.yaml` 保留為設計參考

### Git 規範

- Commit 格式：Conventional Commits — `<type>(<scope>): <description>`
- 常用 type：`feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- 常用 scope：`auth`, `core`, `conversation`, `ai`, `tools`, `email`, `notifications`, `storage`, `audit`, `frontend`

## 文件結構

```
docs/
  architecture.md              # 功能架構規格
  development-guidelines.md    # 完整開發準則
  data-model/                  # 18 個資料模型定義
  api/                         # OpenAPI 3.1 規格（24 個檔案）
  system/                      # 系統架構文件（13 個檔案）
scripts/
  check-cross-refs.py          # 跨文件一致性檢查
```
