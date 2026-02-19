# Conf-Ops Docs 一致性審查（排除所有 spec-review-report*）

## 審查範圍
- `docs/plans/phase-*.md`
- `docs/architecture.md`
- `docs/system/**`
- `docs/data-model/**`
- `docs/api/**`
- 明確排除：`spec-review-report.md`、`spec-review-report-v2.md`

## 總結
- **能力覆蓋**：完整（見 `coverage-matrix.md`）
- **主要風險**：不是功能缺席，而是跨文件契約不一致（路徑前綴、enum 值、資料結構、示例）

## 問題清單（依嚴重度）

### Critical

1) **External API 前綴定義重複，會導致路徑語義衝突**
- 證據：
  - `docs/api/openapi.yaml` 定義 server `url: /external/v1`
  - 同檔 paths 又定義 `/external/v1/projects/{projectId}/...`
- 影響：若依 OpenAPI server + path 組合，會形成 `/external/v1/external/v1/...` 的歧義。
- 建議：二擇一統一
  - A. 保留 server `/external/v1`，把 external paths 改成 `/projects/...`
  - B. 保留目前 paths，把 external server 改成 `/`

### High

2) **ApiKey `permissions` 結構互相衝突（array vs object）**
- 證據：
  - `docs/data-model/18-api-key.md`：`permissions JSONB DEFAULT '[]'`，JSON Schema 為字串陣列（`ApiKeyPermissions`）
  - `docs/plans/phase-11-webhook-audit-observability.md`：`permissions` 為 `{scopes, data_access}` 物件
  - `docs/api/paths/api-keys.yaml`：請求/回應也採物件結構（`scopes`, `dataAccess`）
- 影響：資料庫結構、service 驗證、API 契約三方無法同時成立。
- 建議：以 API + phase-11 的 object 結構為準，回寫修正 `data-model/18-api-key.md`（DDL、JSON Schema、Rust struct 範例）。

3) **Reminder type enum 與 example 值不一致**
- 證據：
  - `docs/api/paths/projects.yaml` create reminder enum: `due_date_approaching|due_date_overdue|todo_stale`
  - 同段 example 卻是 `type: "due_date"`
- 影響：用戶端按 example 實作會送出非法值。
- 建議：example 改成 enum 內合法值（如 `due_date_approaching`）。

4) **通知類型在 phase 文件使用不存在值 `system`**
- 證據：
  - `docs/plans/phase-07-email-integration.md` 提到 `notification_type: system`
  - `docs/data-model/15-notification.md` / `docs/api/schemas/common.yaml` NotificationType 無 `system`
- 影響：事件到通知的型別映射不一致，會造成實作歧義。
- 建議：將 phase-07 改為既有類型（若是系統提醒應使用 `reminder` 或補充新 enum 並全域同步）。

### Medium

5) **phase 責任重疊，易造成交付邊界不清**
- 證據（自動比對）：
  - `/api/v1/accounts/me/notification-preferences` 同時出現在 phase-01、phase-10
  - `/external/v1/projects/{projectId}/task-templates/{templateId}/data` 同時出現在 phase-04、phase-11
- 影響：里程碑驗收與責任歸屬可能重複。
- 建議：在 phase 文檔加上「先占位/後完整」說明與明確驗收邊界。

6) **phase-08 query 參數命名與 API 契約大小寫不一致**
- 證據：
  - phase-08 用 `scope_type` / `scope_id`
  - `docs/api/paths/memories.yaml` 為 `scopeType` / `scopeId`
- 影響：實作時容易誤用 query 參數。
- 建議：phase-08 改為與 OpenAPI 一致（camelCase）。

### Low

7) **多處 example ID 非合法 UUID v7 格式**
- 證據：
  - common UUID pattern 要求十六進位 v7
  - 多個 path example 使用 `...-n001...`, `...-ak01...`, `...-th01...` 等非 hex 片段
- 影響：可能使 lint / codegen / mock 測試資料混淆。
- 建議：統一改為合法 UUID v7 範例。

8) **cross-ref 腳本與文件政策有一處判讀噪音**
- 證據：`scripts/check-cross-refs.py` 警示 `conversation_states`、`last_seen_positions`、`scheduled_reminders`
- 對照：`docs/data-model/README.md` 已說明這些為基礎設施表（不需專屬 API response schema）
- 影響：工具警示噪音。
- 建議：調整腳本忽略清單或依 README 基礎設施表清單動態排除。

## 優先修補順序
1. 修正 External API 前綴衝突（Critical）
2. 統一 ApiKey permissions 契約（High）
3. 修正 reminder enum/example 與 `notification_type: system`（High）
4. 清理 phase 邊界重疊與參數命名（Medium）
5. 批次修正 UUID 範例與 cross-ref 腳本噪音（Low）
