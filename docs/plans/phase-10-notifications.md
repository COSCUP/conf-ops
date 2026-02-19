# Phase 10：通知與提醒系統

**階段目標：** 實作多頻道通知派送（in-app / Web Push / Email）、提醒排程（截止日期、停滯待辦）、通知偏好路由，使使用者能即時收到相關事件通知。

**前置依賴：** Phase 9 完成

---

## 後端任務

### B-10.1 通知派送引擎

**範圍：** 建立 notifications 資料表、多頻道派送、通知偏好路由。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0019_notifications.sql`：
   - `notifications` 表：id, account_id (FK), type (VARCHAR(50)), title (VARCHAR), body (TEXT, nullable), reference_type (VARCHAR(20), nullable — task/todo/message), reference_id (UUID, nullable), project_id (UUID, nullable, FK → projects), is_read (BOOLEAN, DEFAULT false), read_at (TIMESTAMPTZ, nullable), delivered_channels (JSONB, DEFAULT '[]'), created_at
   - （欄位與 `docs/data-model/15-notification.md` 一致：使用 `type` 而非 `notification_type`；`delivered_channels` 為 JSONB 而非 VARCHAR[]；不含 `project_name`、`task_name`、`task_id` 等非 Data Model 欄位；不含 `updated_at` 和 `deleted_at`）
   - `notification_preferences` 表：id, account_id (FK, UNIQUE), preferences (JSONB), updated_at
   - `web_push_subscriptions` 表：id, account_id (FK), endpoint (TEXT), p256dh_key (TEXT), auth_key (TEXT), device_name (VARCHAR, nullable — 使用者辨識不同裝置), created_at
3. 建立 `NotificationService`：
   - `dispatch_notification(recipient_account_id, notification_type, title, body, metadata) -> Vec<Notification>`：
     - 查詢收件人的 notification_preferences
     - 根據 notification_type 與偏好決定派送頻道
     - **in_app**：寫入 notifications 表
     - **web_push**：透過 VAPID 推送（使用 `web-push` crate）
     - 環境變數統一使用 `WEB_PUSH_VAPID_PRIVATE_KEY`、`WEB_PUSH_VAPID_PUBLIC_KEY` 前綴（不使用 `NOTIFICATION_` 前綴，與 `docs/system/10-notification-system.md` 統一）
     - **email**：透過 EmailService 發送通知信
   - `mark_as_read(account_id, notification_id)`
   - `mark_all_as_read(account_id)`
   - `list_notifications(account_id, cursor, limit, unread_only?) -> PaginatedNotifications`
   - `get_unread_count(account_id) -> usize`
   - `schedule_reminder(target_type, target_id, remind_at, reminder_type)` — 建立排程提醒（與 ReminderScheduler 互動）
4. **通知類型與觸發**（訂閱 DomainEvent，7 種類型，與 `docs/data-model/15-notification.md` 一致）：
   - `todo_assigned` → 待辦指派給你
   - `member_mentioned` → 對話中 @mention 你
   - `tag_mentioned` → 對話中 @mention 你的標籤
   - `task_completed` → 你參與的任務已完成
   - `linked_task_completed` → 關聯的子任務已完成
   - `source_data_changed` → 來源資料表資料異動
   - `reminder` → 排程提醒觸發的通知（涵蓋 due_date_approaching、due_date_overdue、todo_stale）
5. **通知偏好 JSONB 結構**（與 `docs/api/schemas/notifications.yaml` 的 `NotificationPreferences` 統一 schema 一致）：
   ```json
   {
     "channels": {
       "email": {
         "enabled": true,
         "categories": {
           "taskUpdates": true,
           "todoAssignments": true,
           "aiSuggestions": true,
           "mentions": true,
           "systemAnnouncements": true
         }
       },
       "webPush": {
         "enabled": true,
         "categories": { ... }
       },
       "inApp": {
         "enabled": true,
         "categories": { ... }
       }
     }
   }
   ```
   > **注意**：通知偏好以 channels 為頂層包裝（email/webPush/inApp），每個頻道包含 enabled 與 categories（taskUpdates, todoAssignments, aiSuggestions, mentions, systemAnnouncements）。
6. API 路由：
   - `GET /api/v1/notifications` — 通知列表（分頁 + 未讀過濾）
   - `GET /api/v1/notifications/unread-count` — 未讀數量
   - `PUT /api/v1/notifications/{notificationId}/read` — 標記已讀（與 API Spec 一致）
   - `PUT /api/v1/notifications/read-all` — 全部已讀（與 API Spec 一致）
   - `GET /api/v1/accounts/me/notification-preferences` — 取得通知偏好設定（與 API Spec 一致）
   - `PUT /api/v1/accounts/me/notification-preferences` — 更新通知偏好設定（與 API Spec 一致）
   > **注意：** Phase 10 完善 Phase 1 建立的 notification-preferences 端點
   - `POST /api/v1/notifications/web-push/subscribe` — 註冊 Web Push subscription（與 API Spec 一致）
   - `DELETE /api/v1/notifications/web-push/subscriptions/{endpoint}` — 取消 Web Push（與 API Spec 一致，按 endpoint 識別訂閱）

**涉及檔案：**
- `migrations/0019_notifications.sql`
- `src/modules/notifications/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/modules/notifications/dispatcher.rs`
- `src/modules/notifications/web_push.rs`
- `src/api/routes/notifications.rs`

**測試要求：**
- 整合測試：各通知類型觸發與派送
- 整合測試：偏好路由（啟用/停用各頻道）
- 整合測試：Web Push subscription 管理
- 整合測試：通知 CRUD（列表、已讀、未讀計數）
- API 測試：所有通知端點

**驗收標準：**
- [ ] 7 種通知類型全部觸發正確
- [ ] 3 種頻道派送正確
- [ ] 偏好路由正確
- [ ] 未讀計數與標記已讀
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

### B-10.2 提醒排程器

**範圍：** 建立背景排程任務，定期檢查截止日期與停滯待辦，觸發提醒通知。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0020_reminders.sql`：
   - `scheduled_reminders` 表：id, todo_id (FK → todos), type (VARCHAR(30)), trigger_at (TIMESTAMPTZ), fired (BOOLEAN, DEFAULT false), fired_at (TIMESTAMPTZ, nullable), notification_id (FK → notifications, nullable), config (JSONB, nullable), created_at, updated_at（與 `docs/data-model/15-notification.md` 一致）
3. 建立 `ReminderScheduler`（在 worker process 中運行）：
   - **截止日期提醒**：
     - 每小時掃描 todos WHERE due_date IS NOT NULL AND status != completed
     - due_date - 24h → dispatch due_date_approaching
     - due_date < now → dispatch due_date_overdue
   - **停滯檢測**：
     - 每日掃描 todos WHERE status = in_progress AND updated_at < now - 7d
     - dispatch todo_stale
   - **自訂提醒**（未來擴展）：
     - 使用者設定的自訂提醒時間
4. 使用 Tokio interval 排程，避免重複發送（scheduled_reminders.fired = true）
5. worker process 啟動時載入排程器

**涉及檔案：**
- `migrations/0020_reminders.sql`
- `src/modules/notifications/scheduler.rs`
- `src/main.rs`（worker mode 啟動排程器）

**測試要求：**
- 整合測試：截止日期 approaching 提醒
- 整合測試：逾期提醒
- 整合測試：停滯檢測
- 整合測試：避免重複發送

**驗收標準：**
- [ ] 截止日期提醒正確觸發
- [ ] 停滯檢測正確
- [ ] 不重複發送
- [ ] Worker mode 排程器正常運作
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-10.1 通知中心 UI

**範圍：** 通知鈴鐺、通知下拉面板、通知設定、Web Push 註冊。

**說明：**
1. `NotificationBell.vue`（頂部導航欄）：
   - 鈴鐺圖標 + 未讀數量徽章
   - 點擊展開通知下拉面板
2. `NotificationPanel.vue`（下拉面板）：
   - 通知列表（按時間排序）
   - 未讀/全部切換
   - 點擊通知 → 跳轉至對應任務
   - 「全部已讀」按鈕
3. `NotificationPreferencesView.vue`（帳號設定內）：
   - 各通知類型的頻道開關（in_app / web_push / email）
   - Web Push 啟用/停用
4. 建立 `notificationStore`
5. **即時通知**：透過 WebSocket 接收新通知，更新未讀計數

**涉及檔案：**
- `frontend/src/components/notification/NotificationBell.vue`
- `frontend/src/components/notification/NotificationPanel.vue`
- `frontend/src/views/settings/NotificationPreferencesView.vue`
- `frontend/src/stores/notification.ts`

**測試要求：**
- 元件測試：NotificationBell 未讀徽章
- 元件測試：NotificationPanel 列表渲染
- 元件測試：偏好設定互動
- 單元測試：notificationStore

**驗收標準：**
- [ ] 未讀徽章即時更新
- [ ] 通知列表正確顯示
- [ ] 點擊通知跳轉正確
- [ ] 偏好設定可操作
- [ ] Web Push 可啟用
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 10 後，以下端到端流程可驗證：

1. **@mention 通知**：在對話中 @mention 某人 → 該人收到 in-app 通知 + Web Push（若啟用）
2. **待辦指派通知**：指派待辦給某人 → 該人收到通知
3. **截止日期提醒**：設定待辦截止日期 → 到期前 24 小時收到提醒 → 逾期後收到逾期通知
4. **通知偏好**：關閉某類通知的 email 頻道 → 該類通知不再寄 Email
5. **即時更新**：收到新通知時鈴鐺徽章即時更新
