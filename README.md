# Zoe Meet（Mac 版）

給 Zoe 使用的 Mac 會議字幕、中文轉英文回覆，以及中英對照講稿工具。

Based on OwnVoice Meet by Zaious, ChronicleCore Studio。
本專案是 [OwnVoice Meet](https://github.com/Zaious/ownvoice-meet) 的修改版；保留 Apache 2.0 授權與原作者的 NOTICE，應用程式另名為 **Zoe Meet**。

目前為第一個 Mac 測試版。已完成本機建置、介面示範、自動測試及授權後的系統音訊／預設麥克風短測。系統音訊已收到播放的測試音；麥克風能回傳音量，本次環境聲偏小。雲端辨識／翻譯仍需自行填入 API key 後測試。

## Mac 使用方式

需要 **Apple Silicon Mac（M1／M2／M3／M4 等）與 macOS 14.6 以上**。目前打包目標是 arm64；Intel Mac 尚未驗證。

1. 開啟 `Zoe Meet.app`。也可以從 DMG 將 App 拖到「應用程式」。
2. 按 **Demo** 先看字幕、回覆與講稿，不需要 API key、不收音、不連線辨識服務。
3. 開 **⚙ 設定**，線上會議選「電腦播放的聲音」，現場活動選「麥克風」。
4. 先按「電腦聲音檢查」或「麥克風檢查」，完成 macOS 權限授權。檢查只在本機量音量，不存音檔、不呼叫雲端服務。
5. 選擇語音辨識與翻譯服務，將自己的 API key 填入設定並儲存。key 存在 macOS 鑰匙圈；設定畫面不會讀回 key。
6. 按「聆聽」，開啟回覆／講稿視窗；輸入中文後讀出產生的英文。

若 macOS 不讓首次開啟，先確認檔案來自你自己的 fork 或這台電腦的建置，再依系統設定「隱私權與安全性」提供的個別 App 開啟流程操作。此個人版本使用本機 ad-hoc 簽章，尚未申請 Apple Developer ID 簽章與公證。

## 權限與快捷鍵

- 系統收音：系統設定 → 隱私權與安全性 → 螢幕與系統錄音 → 僅限系統錄音。允許 **Zoe Meet**。不需要螢幕錄影權限，也不用安裝虛擬音效驅動。
- 麥克風：系統設定 → 隱私權與安全性 → 麥克風。允許 **Zoe Meet**。
- 授權後若仍無聲，結束並重開程式，再做收音檢查。
- **⌘+Shift+Space**：叫出回覆視窗。
- **⌘+Shift+F8**：切換字幕視窗滑鼠穿透，也可從選單列圖示恢復。
- **⌘+Shift+F9**：切換錄音段落。依鍵盤設定，F8／F9 可能還要加按 **fn**。

**分享畫面時請只分享要展示的視窗。** macOS 上無法保證將回覆與講稿排除在整個螢幕的分享／截圖之外，畫面已移除「一定隱藏」的承諾。

## 與上游的差異

- Mac `.app`／`.dmg` 打包、音訊用途說明與麥克風 entitlement。
- 使用 CoreAudio 原生全系統播放擷取；麥克風仍走 CPAL。系統收音不會因為耳機同時提供輸入裝置而誤選麥克風。
- 啟用 Keyring 的 `apple-native`，key 使用此修改版獨立的鑰匙圈服務名稱。
- Mac 顯示 Command 快捷鍵，收音期間防止電腦／螢幕閒置休眠。
- 本機收音檢查不需要金鑰、也不儲存錄音。
- App、設定與錄音資料夾另名，更新只檢查 **azoezoe/ownvoice-meet**；不沿用上游的更新簽章公鑰。
- GitHub Actions 增加 Apple Silicon Mac 測試、打包與下載成品。

字幕、翻譯、回覆、講稿與錄音的既有用法，見 [使用手冊](docs/MANUAL.md)。其中上游的 Windows 安裝、權限與更新步驟不適用於 Mac。原始專案說明保存在 [UPSTREAM_README](docs/UPSTREAM_README.md)；其中版本、支援平台和價格是上游的原始描述，不代表此分支已重新驗證。

## 從原始碼執行

需要 Node.js 22.12+（建議 24）、Rust stable、Xcode Command Line Tools。

```sh
npm ci
npm run tauri dev
```

打包：

```sh
node scripts/third-party-licenses.mjs aarch64-apple-darwin
npm run tauri build -- --bundles app,dmg --config src-tauri/tauri.release.conf.json
```

產物位於 `src-tauri/target/release/bundle/`。開發版可複製 `env.example` 為 `.env.local`；請勿提交金鑰、講稿或會議資料。

檢查：

```sh
npm run build
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
```

需要硬體或真實 API key 的測試預設略過。Mac CI 會建立可下載的 App ZIP 和 DMG，沒有自動發布公開 release。

## 授權與致謝

Apache License 2.0，見 [LICENSE](LICENSE)、[NOTICE](NOTICE)、[第三方致謝](THIRD_PARTY_NOTICES.md)。修改版附上原作者 NOTICE。`OwnVoice Meet` 是原作品名稱，不是此修改版的產品名稱。
