# Phase 6：檔案儲存系統

**階段目標：** 實作本地檔案儲存系統，含上傳/下載 API、檔案驗證、附件管理、孤兒檔案清理，使對話訊息和資料表可附帶檔案。

**前置依賴：** Phase 5 完成

---

## 後端任務

### B-6.1 檔案儲存模組（Storage Module）

**範圍：** 建立完整的 storage 模組，實作 StorageTrait（本地檔案系統）、檔案 CRUD、驗證與清理。

**說明：**
1. 遵循 TDD 流程：先撰寫失敗的測試定義預期行為 → 實作最少量程式碼使測試通過 → 重構改善品質
2. 建立 migration `0014_files.sql`：
   - `files` 表：id, original_name (VARCHAR), stored_path (VARCHAR), mime_type (VARCHAR), size_bytes (BIGINT), scope_type (VARCHAR — task/project 等), scope_id (UUID — 所屬資源 ID), status (VARCHAR, DEFAULT 'confirmed' — pending/confirmed/rejected，支援 S3 兩階段上傳), uploaded_by (FK → accounts), organization_id (FK, nullable), project_id (FK, nullable), task_id (FK, nullable), created_at, deleted_at
   - `file_metadata` 表：id, file_id (FK), key (VARCHAR), value (TEXT), created_at
3. 建立 `StorageService` trait（可替換的儲存後端介面，與 `docs/system/11-file-storage.md` 一致）：
   ```rust
   #[async_trait]
   pub trait StorageService: Send + Sync {
       async fn upload(&self, filename: &str, mime_type: &str, data: impl AsyncRead + Send, file_size: u64, scope_type: &str, scope_id: Uuid, uploaded_by: Uuid) -> Result<FileMeta>;
       async fn download(&self, file_id: Uuid) -> Result<(FileMeta, impl AsyncRead + Send)>;
       async fn delete_file(&self, file_id: Uuid) -> Result<()>;
       async fn cleanup_orphaned_files(&self) -> Result<u64>;
   }
   ```
4. 建立 `LocalStorageService`：
   - 儲存至 `STORAGE_BASE_PATH`（環境變數）
   - 路徑規則：`{scope_type}/{scope_id}/{year}/{month}/{uuid}/{filename}`（與 `docs/system/11-file-storage.md` 一致）
5. 建立 `FileService`：
   - `upload_file(actor, file_data, metadata) -> FileHandle`：
     - 驗證檔案大小（依類型限制：Image 10MB `STORAGE_MAX_IMAGE_SIZE`、Document 50MB `STORAGE_MAX_DOCUMENT_SIZE`、Other 20MB `STORAGE_MAX_FILE_SIZE`，與 `docs/system/11-file-storage.md` 一致）
     - 驗證 MIME 類型（白名單：image/*, application/pdf, text/*, .xlsx, .docx 等）
     - 產生 UUID v7 作為檔案 ID
     - 儲存檔案 + 寫入 files 記錄
   - `download_file(actor, file_id) -> (FileStream, FileMetadata)`：
     - 權限檢查：根據檔案的 scope_type + scope_id 決定存取權限（如 scope_type='task' 時檢查使用者是否為任務參與者，scope_type='project' 時檢查專案成員身份）
     - 串流下載
   - `delete_file(actor, file_id)`：軟刪除（files.deleted_at）
   - `cleanup_orphaned_files()`：定期清理孤兒檔案（已軟刪除超過 7 天的檔案，`STORAGE_CLEANUP_GRACE_PERIOD` = 604800 秒，與 `docs/system/11-file-storage.md` 一致）
6. API 路由：
   - `POST /api/v1/files/upload` — multipart form upload
   - `GET /api/v1/files/{fileId}` — 下載檔案（Local 後端：200 OK 串流回應；S3 後端：302 重導向至 Presigned URL，與 `docs/system/11-file-storage.md` 一致）
   - `DELETE /api/v1/files/{fileId}` — 刪除檔案
   （注意：無 `/files/{fileId}/metadata` 端點，與 `docs/api/paths/files.yaml` 一致）
7. 整合至對話訊息與資料表：
   - 對話 attachments JSONB 結構：`[{ "fileName": "string", "mimeType": "string", "fileSize": number, "storagePath": "string", "fileId": "UUID" }]`（與 `docs/data-model/09-task-conversation.md` 的 Attachment 結構一致）
   - Email 附件自動儲存流程：收件時附件先存至 storage，再記錄到 messages.attachments
   - 資料表 image/file 欄位引用 file_id
8. 新增 DomainEvent：`FileUploaded`, `FileDeleted`

**涉及檔案：**
- `migrations/0014_files.sql`
- `src/modules/storage/mod.rs`, `models.rs`, `repository.rs`, `service.rs`
- `src/modules/storage/backend.rs`（StorageTrait）
- `src/modules/storage/local.rs`（LocalStorageBackend）
- `src/api/routes/files.rs`

**測試要求：**
- 單元測試：MIME 類型驗證白名單
- 單元測試：檔案大小限制
- 整合測試：上傳 → 下載 → 比對內容一致
- 整合測試：刪除檔案（軟刪除）
- 整合測試：孤兒檔案清理
- 整合測試：權限檢查（無權限者無法下載）
- API 測試：multipart upload 端點

**驗收標準：**
- [ ] 檔案上傳/下載完整
- [ ] MIME 類型與大小驗證正確
- [ ] StorageTrait 抽象可替換（為未來 S3 準備）
- [ ] 孤兒檔案清理機制
- [ ] 對話附件與資料表檔案欄位整合
- [ ] `cargo clippy -- -D warnings` 零警告（無使用 `#[allow(...)]` 忽略）
- [ ] `cargo fmt -- --check` 通過
- [ ] 錯誤回應符合 RFC 7807 Problem Details 格式
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 前端任務

### F-6.1 檔案上傳與附件管理 UI

**範圍：** 檔案上傳元件、訊息附件、資料表檔案欄位。

**說明：**
1. `FileUploader.vue` 通用元件：
   - 拖拽上傳 + 點擊選檔
   - 上傳進度條
   - 檔案大小/類型前端預驗證
   - 上傳完成後返回 file_id
2. 整合至 `MessageInput.vue`：
   - 附件按鈕 → 開啟 FileUploader
   - 附件預覽（圖片縮圖、檔案圖標）
3. 整合至 `DataEntryForm.vue`：
   - image 欄位類型：FileUploader + 圖片預覽
   - file 欄位類型：FileUploader + 檔案連結
4. `FilePreview.vue` 元件：
   - 圖片：縮圖 + 點擊放大
   - PDF：嵌入預覽
   - 其他：檔案名 + 下載連結
5. 建立 `useFileUpload` composable

**涉及檔案：**
- `frontend/src/components/file/FileUploader.vue`
- `frontend/src/components/file/FilePreview.vue`
- `frontend/src/composables/useFileUpload.ts`
- `frontend/src/components/conversation/MessageInput.vue`（擴展）
- `frontend/src/components/data-schema/DataEntryForm.vue`（擴展 image/file 欄位）

**測試要求：**
- 元件測試：FileUploader 互動（選檔、進度）
- 元件測試：FilePreview 各種類型渲染
- 單元測試：前端檔案驗證
- 單元測試：useFileUpload composable

**驗收標準：**
- [ ] 拖拽上傳可使用
- [ ] 上傳進度正確顯示
- [ ] 訊息附件正確顯示
- [ ] 資料表 image/file 欄位正確渲染
- [ ] `npm run lint -- --max-warnings 0` 零警告
- [ ] `npm run typecheck` 通過
- [ ] 所有 commit 遵循 Conventional Commits 格式

---

## 階段交付物

完成 Phase 6 後，以下端到端流程可驗證：

1. **檔案上傳**：在對話中拖拽上傳圖片 → 上傳進度 → 預覽圖片 → 其他人可下載
2. **資料表附件**：在資料表 image 欄位上傳圖片 → 縮圖顯示 → 點擊放大
3. **檔案管理**：上傳 → 查看元資料 → 刪除
4. **類型驗證**：嘗試上傳超大檔案或不允許的類型 → 前後端都正確拒絕
