# Phase 7：Email 整合系統

**階段目標：** 實作完整的 Email 收發系統，含 SMTP 寄件、HTTP Webhook 收件、信件串聯比對、寄件人身份解析，使成員與外部聯絡人可透過 Email 與任務對話互動。

**前置依賴：** Phase 6 完成

---

## 後端任務

### B-7.1 Email 寄件（SMTP Outbound）與多信件串管理

**範圍：** 建立 SMTP 寄件服務、每個任務的多信件串管理、outbound mail 記錄。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0015_email.sql`：
   - `email_threads` 表：id, task_id (FK), subject (VARCHAR), participants (JSONB — 參與者列表), message_ids (JSONB — SMTP Message-ID 陣列，用於串聯追蹤), last_message_at (TIMESTAMPTZ), created_at, updated_at
     - GIN 索引：`CREATE INDEX idx_email_threads_message_ids ON email_threads USING GIN (message_ids)`
   - `email_messages` 表（統一收發記錄）：id, thread_id (FK → email_threads), message_id (VARCHAR, UNIQUE — SMTP Message-ID), in_reply_to (VARCHAR, nullable), references_header (TEXT, nullable), from_address (VARCHAR), to_addresses (JSONB), cc_addresses (JSONB, DEFAULT '[]'), subject (VARCHAR), direction (VARCHAR: inbound/outbound), conversation_message_id (UUID, FK → messages, nullable — 對應的任務對話訊息), raw_headers (JSONB, nullable — 原始 Email 標頭供除錯用), created_at
   （資料表結構與 `docs/data-model/16-email-thread.md` 2.2 節一致，Email 記錄為 write-once 不可變資料）
3. 建立 `EmailService`（trait + impl）：
   - `send_email(task_id, thread_id?, to, subject, body, reply_to?) -> EmailMessage`：
     - 產生唯一的 Message-ID header（`<{uuid}@{domain}>`）
     - 如有 thread_id，設定 In-Reply-To 與 References headers，並更新 email_threads.message_ids 陣列
     - 如無 thread_id，建立新的 email_thread
     - 透過 lettre SMTP client 發送
     - 記錄至 email_messages（direction = outbound）
   - `retry_failed_emails()` → 重試失敗的郵件（最多 3 次，指數退避）
4. **多信件串機制**：
   - 每個任務可有多個 email_threads（如：與贊助商 A 的信件串、與贊助商 B 的信件串）
   - 信件串透過 SMTP Message-ID / In-Reply-To / References headers 追蹤
5. 建立 lettre SMTP client 初始化（從環境變數讀取設定）
6. 新增 DomainEvent：`EmailSent`

**涉及檔案：**
- `migrations/0015_email.sql`
- `src/modules/email/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/modules/email/smtp.rs`（完善）
- `src/api/routes/email.rs`

**測試要求：**
- 整合測試：寄件記錄正確（Message-ID, In-Reply-To, References）
- 整合測試：多信件串管理
- 整合測試：失敗重試邏輯
- 整合測試：MailHog 收到正確格式的郵件
- API 測試：寄件端點

**驗收標準：**
- [ ] SMTP 寄件可成功發送
- [ ] Message-ID / In-Reply-To / References headers 正確
- [ ] 多信件串追蹤機制正確
- [ ] 失敗重試機制運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-7.2 Email 收件（Inbound Webhook）與信件串比對

**範圍：** 建立 HTTP Webhook 收件端點、MIME 解析、信件串比對演算法、寄件人身份解析。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 Webhook 端點：
   - `POST /api/v1/email/inbound` — 接收轉發的 Email（API Key 認證）
   - 支援兩種格式：AWS SES Lambda 轉發 / Cloudflare Email Worker 轉發
   （兩條收信路線，參考 `docs/system/06-email-integration.md`）
   - **來源識別與驗證**：透過 request headers 區分來源——AWS SES 包含 `X-Amz-Sns-*` 或自訂 `X-SES-*` headers，Cloudflare Email Worker 包含 `CF-*` headers。依據來源套用對應的 payload 解析邏輯（SES 使用 SNS notification JSON 格式，Cloudflare 使用自訂 JSON 格式）
3. **MIME 解析**（使用 `mail-parser` crate）：
   - 解析 From, To, Subject, Message-ID, In-Reply-To, References headers
   - 提取 body（HTML + plain text）
   - 提取附件 → 儲存至 storage 模組
4. **信件串比對演算法**（三階段）：
   1. **精確比對**：同時檢查 In-Reply-To header 和 References header，使用 `email_threads.message_ids` 的 GIN 索引（`idx_email_threads_message_ids`，參考 `docs/data-model/16-email-thread.md` 第 3 節）進行高效查詢
   2. **啟發式比對**：正規化主旨（移除 Re:/Fwd:/Fw: 等前綴後比對）+ 相同寄件人 → 匹配到任務
   3. **未分類**：無法匹配時信件自動進入「專案未分類收件匣」（`GET /api/v1/projects/{projectId}/unassigned-inbox`），並觸發 `notification_type: reminder` 通知專案管理員（owner 角色），由人工透過 `POST .../unassigned-inbox/{emailId}/assign` 歸入任務
5. **寄件人身份解析**：
   - 從 From header 提取 email → 查詢 accounts 或 contacts
   - 若匹配到 account → 標記為 member 訊息
   - 若匹配到 contact → 標記為 contact 訊息
   - 若無匹配 → 自動建立 contact
   - **成員轉寄偵測**：若寄件者為成員但信件內容含轉寄標記，解析原始寄件者以 Contact 身份記錄（而非轉寄者本人）
6. 收件後產生對話訊息（source_type: email_inbound）
7. API 路由：
   - `POST /api/v1/email/inbound` — 收信 Webhook（API Key 認證）
   - `GET /api/v1/projects/{projectId}/unassigned-inbox` — 未分類收件匣列表
   - `POST /api/v1/projects/{projectId}/unassigned-inbox/{emailId}/assign` — 手動歸入任務
   （與 `docs/api/paths/email-inbound.yaml` 一致）
8. 新增 DomainEvent：`EmailReceived`（觸發 AI 建議）

**涉及檔案：**
- `src/modules/email/inbound.rs`
- `src/modules/email/thread_matcher.rs`
- `src/modules/email/sender_resolver.rs`
- `src/api/routes/email.rs`（擴展）

**測試要求：**
- 整合測試：MIME 解析（含附件）
- 整合測試：精確比對（Message-ID / In-Reply-To）
- 整合測試：啟發式比對（寄件人 + 主旨）
- 整合測試：未分類處理
- 整合測試：寄件人解析（member / contact / 自動建立）
- 整合測試：收件後自動產生對話訊息
- API 測試：Webhook 端點認證與處理

**驗收標準：**
- [ ] Webhook 收件端點正確處理 Email
- [ ] 三階段信件串比對正確
- [ ] 寄件人身份解析正確
- [ ] 附件自動儲存至 storage
- [ ] 收件自動轉為對話訊息
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-7.1 Email 管理 UI

**範圍：** 任務內的信件串檢視、寄件介面、未分類收件箱。

**說明：**
1. `EmailThreadsPanel.vue`（嵌入 TaskDetailView）：
   - 任務的信件串列表
   - 每個信件串的來往信件時間線
   - 回覆信件（引用原信 + 編輯回覆內容）
   - 寄送新信件（選擇收件人、主旨、內容）
2. `UnclassifiedInbox.vue`：
   - 未分類收件列表
   - 手動歸入任務（選擇專案 → 任務）
   - 建立新聯絡人
3. 整合至 `ConversationPanel.vue`：
   - Email 類型訊息以特殊樣式顯示（信件圖標、寄件人地址、主旨）

**涉及檔案：**
- `frontend/src/components/email/EmailThreadsPanel.vue`
- `frontend/src/components/email/EmailComposer.vue`
- `frontend/src/views/projects/UnclassifiedInboxView.vue`
- `frontend/src/components/conversation/MessageItem.vue`（擴展 email 樣式）

**測試要求：**
- 元件測試：EmailThreadsPanel 渲染
- 元件測試：EmailComposer 表單
- 元件測試：未分類歸入操作

**驗收標準：**
- [ ] 信件串檢視完整
- [ ] 回覆/新寄信件可操作
- [ ] 未分類收件箱可手動歸入任務
- [ ] Email 訊息在對話中顯示正確
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 7 後，以下端到端流程可驗證：

1. **寄件**：在任務中撰寫 Email → 寄出 → MailHog 收到正確格式郵件（含 Message-ID headers）
2. **收件**：外部寄送 Email → Webhook 接收 → 自動歸入對應任務對話
3. **信件串追蹤**：回覆信件 → 自動歸入同一信件串
4. **寄件人解析**：已知聯絡人寄信 → 自動辨識身份；未知寄件人 → 自動建立聯絡人
5. **未分類處理**：無法自動歸入的信件 → 出現在未分類收件箱 → 手動歸入任務
