---
name: favorites-design
description: 收藏列表功能设计 — 查看收藏视频列表 + 单个下载 + 批量下载收藏夹
type: design
---

# 收藏列表功能设计

## 1. 概述

为 Douyin Downloader 添加收藏列表（收藏夹）功能，支持：
- 查看收藏视频列表
- 单个收藏视频下载
- 批量下载全部收藏视频

## 2. 架构决策

收藏列表完全复用点赞列表（Likes）的架构模式。两者在前端展示形态上高度一致（视频卡片列表），区别仅在于：
- 后端调用不同的抖音 API 端点
- 前端 UI 文案和图标不同（`bi-star-fill` 收藏 vs `bi-heart-fill` 点赞）
- 独立的 localStorage 缓存键

## 3. 后端变更

### 3.1 `src-tauri/src/api/client.rs`

新增 2 个方法：

```rust
/// 请求收藏列表原始响应
pub async fn get_collected_videos_response(
    &self,
    max_cursor: i64,
    count: u32,
) -> Result<serde_json::Value>

/// 获取收藏视频列表（Python 风格，复用 LikedVideoItem 类型）
pub async fn get_collected_videos_python_style(
    &self,
    max_cursor: i64,
    count: u32,
) -> Result<Vec<LikedVideoItem>>
```

API 端点：`GET https://www.douyin.com/aweme/v1/web/aweme/listcollection/`
参数：`cursor`、`count`，`skip_sign: true`

实现模式完全参照 `get_liked_videos_response()` / `get_liked_videos_python_style()`。

### 3.2 `src-tauri/src/lib.rs`

新增 2 个 Tauri 命令：

```rust
#[tauri::command]
async fn get_collected_videos(
    state: State<'_, AppState>,
    cursor: i64,
    count: u32,
) -> Result<serde_json::Value, String>

#[tauri::command]
async fn download_collected_videos(
    state: State<'_, AppState>,
    count: u32,
) -> Result<serde_json::Value, String>
```

实现模式分别参照 `get_liked_videos()` 和 `download_liked_videos()`。

### 3.3 命令注册

在 `run()` 的 `invoke_handler![]` 中添加：
- `get_collected_videos`
- `download_collected_videos`

## 4. 前端变更

### 4.1 `dist/index.html`

**导航栏"更多"下拉菜单**：在点赞作者获取下方添加收藏获取入口：
```html
<div class="dropdown-item-compact">
    <span class="dropdown-item-label"><i class="bi bi-star-fill"></i> 收藏获取</span>
    <input type="number" class="form-control form-control-sm count-input"
           id="collected-videos-count" value="20" min="1" max="100">
    <button class="btn btn-sm btn-warning" id="download-collected-btn">获取</button>
</div>
```

**收藏视频列表 Section**：在 `likedAuthorsSection` 之后添加 `collectedVideosSection`，结构复用 `likedVideosSection`：
- 图标：`bi-star-fill`，标题：`收藏视频列表`
- ID：`collectedVideosSection`、`collectedVideosList`、`collectedVideoCount`
- 批量下载按钮：`downloadAllCollectedVideos()`

**空状态快捷入口**：添加收藏入口卡片：
```html
<div class="shortcut-card" onclick="document.getElementById('download-collected-btn').click()">
    <i class="bi bi-star"></i>
    <span>收藏视频</span>
</div>
```

**Cookie 引导文案**：更新 `cookieOnboardingCopy` 文案加入"收藏列表"。

### 4.2 `dist/js/tauri-adapter.js`

在 `/api/download_liked` 映射之后添加：

```javascript
if (path === '/api/get_collected_videos') {
    const result = await invoke('get_collected_videos', {
        cursor: 0,
        count: Number(body.count || params.count || 20)
    });
    // 同 /api/get_liked_videos 的 normalizeVideos 处理
}

if (path === '/api/download_collected') {
    return invoke('download_collected_videos', {
        count: Number(body.count || params.count || 20)
    });
}
```

### 4.3 `dist/js/app.js`

新增函数（全部参照点赞列表的对应函数）：

| 函数 | 参照 | 说明 |
|------|------|------|
| `downloadCollectedVideos()` | `downloadLikedVideos()` | 获取收藏列表 |
| `displayCollectedVideos(videos)` | `displayLikedVideos(videos)` | 渲染收藏视频卡片 |
| `handleCollectedVideosClick()` | `handleLikedVideosClick()` | 按钮点击处理（含缓存逻辑） |
| `downloadAllCollectedVideos()` | `downloadAllLikedVideos()` | 批量下载全部收藏视频 |

事件监听绑定（在 `likedAuthorsBtn` 绑定之后）：
```javascript
var collectedBtn = document.getElementById('download-collected-btn');
if (collectedBtn) collectedBtn.addEventListener('click', function(e) {
    if (!checkLoginRequired(collectedBtn)) { e.preventDefault(); return; }
    handleCollectedVideosClick();
});
```

### 4.4 `dist/js/storage.js`

新增 `CollectedDataCache`，结构完全参照 `LikedDataCache`：

```javascript
const CollectedDataCache = {
    COLLECTED_VIDEOS_KEY: 'collected_videos_cache',
    CACHE_VERSION: 1,
    // saveCollectedVideos / getCollectedVideos / clearAll / isCacheExpired
    // 实现同 LikedDataCache，使用独立的 localStorage key
};
```

## 5. 数据流

```
用户点击"获取"按钮
  → handleCollectedVideosClick()
  → fetch('/api/get_collected_videos', {count})
    → tauri-adapter 拦截
    → invoke('get_collected_videos', {cursor: 0, count})
      → lib.rs get_collected_videos 命令
      → DouyinClient::get_collected_videos_python_style()
      → GET douyin.com/aweme/v1/web/aweme/listcollection/
      → 解析 aweme_list → Vec<LikedVideoItem>
    → 返回 {success, data, count}
  → displayCollectedVideos(result.data)
  → 渲染视频卡片，存入 CollectedDataCache

用户点击"下载全部"
  → downloadAllCollectedVideos()
  → 遍历 currentVideos，逐个 fetch('/api/download_single_video')
  → invoke('download_video') → Downloader 执行下载
```

## 6. 复用清单

| 组件 | 复用 | 说明 |
|------|------|------|
| `LikedVideoItem` 类型 | ✅ 直接复用 | 收藏视频字段完全匹配 |
| `build_liked_video_item()` | ✅ 直接复用 | 收藏列表解析用同一方法 |
| Section HTML 结构 | ✅ 复制+修改 ID/文案/icon | 结构一致 |
| `normalizeVideos()` | ✅ 直接复用 | adapter 中同一函数 |
| `download_video` 命令 | ✅ 直接复用 | 单个下载走同一命令 |
| `start_batch_download` | ✅ 直接复用 | 批量下载走同一方法 |
| `CollectedDataCache` | ⚠️ 新建（结构复用） | 独立 localStorage key |
