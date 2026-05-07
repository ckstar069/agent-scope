## 2026-05-06 Stage 时间线蛇形布局

- `StageTimeline` 可用 `ResizeObserver` 根据面板宽度动态计算每行阶段数，保留最多 5 个阶段一行，避免横向滚动。
- 蛇形布局中奇数行反向显示时，横向连接箭头需要旋转 180 度；换行连接应挂在逻辑行尾，而不是视觉行尾。

## 2026-05-06 Tauri dialog 权限配置

- 当前 Tauri v2 配置不接受 `tauri.conf.json` 的 `app.permissions` 字段；插件权限应写入 `src-tauri/capabilities/default.json`。
- `tauri-plugin-dialog` 需要同时添加 Rust 依赖、前端 `@tauri-apps/plugin-dialog` 包、Builder 插件注册和 `dialog:default` capability 权限。
