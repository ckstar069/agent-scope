# FileWatcher 集成 — 学习记录

## 任务: 0.4 文件监听集成验证

### 关键发现

1. **tauri-plugin-fs `watch()` API**:
   - 前端 (JS/TS): `watch()` 和 `watchImmediate()` 函数，需要 `features = ["watch"]`
   - 后端 (Rust): 仅提供 `FsExt` trait 用于权限管理，无原生 Rust watch API
   - Rust 端文件监听需要自实现轮询或依赖 `notify` crate
   - 参考: https://v2.tauri.app/plugin/file-system/#watching-changes

2. **FileWatcher 设计决策**:
   - 采用 mtime 轮询方案（std::fs::metadata），避免直接依赖 notify crate
   - 默认轮询间隔 500ms，可在 3 秒内检测并触发事件
   - 事件类型: Modified / Created / Deleted，覆盖文件生命周期的全部阶段
   - 线程模型: 单后台线程 + AtomicBool 停止信号
   - 支持文件级别和目录级别监听（递归/非递归）

3. **项目状态**:
   - Task 0.2 (Tauri 骨架) 已完成 — `src-tauri/Cargo.toml` 和 `lib.rs` 已存在
   - `tauri-plugin-fs` 已安装并注册（watch feature 已启用）
   - 权限配置已更新（fs:allow-read, fs:allow-watch, fs:allow-stat 等）

### 测试结果

- 16 个单元测试全部通过
- 覆盖场景: 单文件修改、目录递归/非递归、文件创建/删除、空目录、不存在的路径、并发修改、stop 信号
- 文件变化在 <3 秒内触发事件（50ms 轮询间隔测试中实测约 50-100ms）
- 监听不存在的文件不 panic
- 空目录不 panic
- 权限问题通过 stderr 报告，不崩
