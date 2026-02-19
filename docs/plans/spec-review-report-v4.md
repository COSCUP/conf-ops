# 規格與計劃審查報告 (v4)

## 1. 審查總結

本次審查涵蓋了專案的計劃文件 (`plans/`)、資料模型 (`data-model/`)、系統架構 (`system/`) 以及 API 定義 (`api/`)。整體而言，文檔品質極高，核心概念（如 AI-Human Loop、CRDT、RBAC）在各處的描述高度一致。

然而，在 **帳號個人資料架構** 與 **專案複製範圍** 兩個領域發現了邏輯落差與文檔矛盾，建議在實作前修正。

---

## 2. 核心落差與矛盾 (Critical Findings)

### 2.1 個人資料表 (Profile Data) 的 Schema 維護機制落差

*   **文件描述**：
    *   `docs/data-model/01-account.md` 指出 `profile_schema` 由系統自動維護，但同時提到「可手動新增或透過 `saveToProfile` 工具...存入」。
    *   `docs/plans/phase-01-auth-account.md` (F-1.2) 提到前端 `ProfileDataView` 根據 `profile_schema` 動態生成表單。
*   **API 定義**：
    *   `docs/api/paths/accounts.yaml` 中的 `PUT /accounts/me/profile` 僅接受 `profileData` (Key-Value pair)。
*   **矛盾點**：
    *   若使用者想「手動新增」一個全新的欄位（例如 "Line ID"），他需要定義該欄位的 `label` ("Line 帳號") 與 `type` ("string")。
    *   目前的 `PUT` API 僅能更新 **值**，無法傳遞 **Schema 定義**（Label/Type/Description）。
    *   **結果**：使用者無法如文件所述「手動新增」新欄位，除非後端有隱藏邏輯能從 Key 推斷 Label（不現實），或者必須依賴 AI 工具 `saveToProfile` 才能建立新欄位 Schema。
*   **建議**：
    *   修改 `UpdateProfileRequest` API，允許傳入 Schema 定義；或
    *   明確定義僅能透過 AI 工具新增 Schema，API 僅用於填寫既有欄位。

### 2.2 專案複製 (Project Copy) 範圍的矛盾

*   **計劃文件 (`docs/plans/phase-02-org-project.md`)**：
    *   明確指出 Phase 2 的 `copy_project` **「僅複製專案基本資料... (成員標籤、任務模板... 於 Phase 12 完善)」**。
*   **API 定義 (`docs/api/paths/projects.yaml`)**：
    *   `/projects/{projectId}/copy` 的描述宣稱執行 **「完整複製：成員標籤、任務模板、專案記憶、工具設定、權限設定」**。
*   **系統文件 (`docs/data-model/02-project.md`)**：
    *   描述 Phase 2 提供「基本複製（含成員標籤、任務模板...）」，Phase 12 擴充「深複製（資料快照）」。
*   **矛盾點**：
    *   API 規格書與系統文件承諾的功能範圍（複製 Template/Tags/Settings），遠大於 Phase 2 實作計劃的範圍（僅複製 Name/Description）。
    *   若開發者依據 Phase 2 計劃實作，將無法滿足 API 規格的宣告，導致 API 使用者（前端）預期落空。
*   **建議**：
    *   修正 Phase 2 計劃，將「成員標籤、任務模板、權限設定」的複製邏輯納入 Phase 2（因為這是建立新專案能立即運作的基礎）；或
    *   修改 API 文件，明確標註目前僅支援基本資料複製。

---

## 3. API 結構與一致性問題

### 3.1 Schema 檔案組織不一致
*   **現狀**：
    *   **Auth/Account**：擁有獨立的 `docs/api/schemas/auth.yaml` 與 `docs/api/schemas/accounts.yaml`。
    *   **Tasks/Projects/Organizations**：沒有獨立 Schema 檔案（如 `tasks.yaml` 不存在），而是集中依賴 `docs/api/schemas/requests.yaml`、`responses.yaml` 與 `entities.yaml`。
*   **影響**：
    *   維護風格不統一，增加查找定義的認知負擔。建議統一採用「按領域分檔」或「按用途分檔（Request/Response）」其中一種策略。

### 3.2 命名與路徑微小差異
*   **Magic Link Verify**：
    *   系統文件圖表顯示 `GET ...?token=xxx`。
    *   API 定義 `GET /auth/magic-link/verify` (Query param)。
    *   **一致**（皆為 GET），但在部分舊的討論或 System Doc 文字描述中曾出現 POST 的痕跡，需確保實作時以 API YAML 為準。

---

## 4. 結論

專案規格完整度很高。建議在進入 Phase 2 開發前，先解決 **專案複製的範圍定義** 問題，以免後端實作過於簡陋無法支撐前端需求。同時需補強 **Profile API** 以支援手動定義新欄位 Schema 的需求。
