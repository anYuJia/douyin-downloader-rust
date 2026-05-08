---
name: clipboard-monitor-design
description: 剪切板监听功能设计 — 自动检测抖音分享链接并弹出下载确认
type: design
---

# 剪切板监听功能设计

## 1. 概述

为 Douyin Downloader 添加剪切板监听功能：
- 后台轮询剪切板，检测抖音分享链接（`v.douyin.com` 短链接和 `www.douyin.com/video/` 完整链接）
- 检测到链接时弹出原生确认对话框
- 用户确认后自动填充链接到输入框并开始解析下载

## 2. 技术方案

### 2.1 依赖

- 添加 `tauri-plugin-clipboard-manager` 到 `Cargo.toml`（读取剪切板内容）
- 使用 `tauri-plugin-dialog`（已有依赖）弹出原生确认框
- 前端 `setInterval` 每 1.5 秒轮询一次（Tauri 插件不提供 OS 级剪切板 hook）

### 2.2 链接匹配规则

匹配以下格式的抖音链接：
- `https://v.douyin.com/xxx/` — 短链接
- `https://www.douyin.com/video/xxx` — 完整视频链接
- `https://www.douyin.com/note/xxx` — 笔记链接
- 也匹配 `http://` 和带 `www.` / 不带 `www.` 的变体

正则：`https?://(?:www\.)?(?:v\.douyin\.com/[a-zA-Z0-9]+/?|douyin\.com/(?:video|note)/[0-9]+)`

### 2.3 去重机制

- 记录上次检测到的链接（`lastClipboardUrl`）
- 如果剪切板中的链接与上次相同，不重复弹窗
- 仅当链接变化时才触发检测

## 3. 后端变更

### 3.1 `src-tauri/Cargo.toml`

添加依赖：
```toml
tauri-plugin-clipboard-manager = "2"
```

### 3.2 `src-tauri/src/lib.rs`

新增 1 个 Tauri 命令：

```rust
#[tauri::command]
async fn read_clipboard(
    state: State<'_, AppState>,
) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let app_handle = state.app_handle.lock().await.clone()
        .ok_or_else(|| "AppHandle 未初始化".to_string())?;
    app_handle.clipboard()
        .read_text()
        .map_err(|e| format!("读取剪切板失败: {}", e))?
        .ok_or_else(|| "剪切板为空".to_string())
}
```

注册到 `invoke_handler![]`。

### 3.3 `src-tauri/tauri.conf.json`

添加插件注册：
```json
"plugins": {
    "clipboard-manager": {},
    ...
}
```

## 4. 前端变更

### 4.1 `dist/js/tauri-adapter.js`

新增映射：
```javascript
if (path === '/api/read_clipboard') {
    return invoke('read_clipboard');
}
```

### 4.2 `dist/js/app.js`

新增剪切板监听逻辑：

```javascript
var lastClipboardUrl = null;

function extractDouyinUrl(text) {
    if (!text) return null;
    var pattern = /https?:\/\/(?:www\.)?(?:v\.douyin\.com\/[a-zA-Z0-9]+\/?|douyin\.com\/(?:video|note)\/[0-9]+)/gi;
    var matches = text.match(pattern);
    return matches ? matches[0] : null;
}

function isDouyinUrl(text) {
    if (!text) return false;
    return /(?:v\.douyin\.com\/|douyin\.com\/(?:video|note)\/)/i.test(text);
}

async function checkClipboard() {
    try {
        var text = await fetch('/api/read_clipboard').then(function(r) { return r.text(); });
        if (!text || !isDouyinUrl(text)) return;
        var url = extractDouyinUrl(text);
        if (!url || url === lastClipboardUrl) return;
        lastClipboardUrl = url;
        // 使用浏览器原生确认框
        var confirmed = confirm('检测到抖音链接，是否下载？\n\n' + url);
        if (confirmed) {
            document.getElementById('link-input').value = url;
            downloadFromLink();
        }
    } catch (e) {
        // 剪切板读取失败时静默忽略
    }
}
```

在 app 初始化时启动轮询（在 `DOMContentLoaded` 监听器末尾）：
```javascript
// 每 1.5 秒检查一次剪切板
setInterval(checkClipboard, 1500);
```

## 5. 交互流程

```
setInterval 每 1.5s
  → fetch('/api/read_clipboard')
    → invoke('read_clipboard') → 读取系统剪切板
  → 正则匹配抖音链接
  → 与 lastClipboardUrl 比对（去重）
  → 链接变化时弹出浏览器原生确认框（confirm）
  → 用户点击"确认"
    → 填充 link-input
    → 调用 downloadFromLink() 开始解析
  → 用户点击"取消"
    → 仅更新 lastClipboardUrl，不再重复弹窗
```

## 6. 注意事项

- 剪切板轮询仅在应用窗口聚焦时运行（可选优化：失焦时暂停）
- 读取剪切板在某些平台可能需要权限，失败时静默忽略
- 正则同时匹配短链接和完整链接
- 使用浏览器原生 `confirm()` 弹窗，简单可靠，无需额外 Tauri dialog API
