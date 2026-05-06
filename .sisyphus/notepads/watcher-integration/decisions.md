# FileWatcher 集成 — 决策记录

## 决策 1: Rust 端文件监听方案

- **问题**: 需要 Rust 端实现文件监听，但 tauri-plugin-fs 的 watch() 是前端 API
- **选项**:
  A. 使用 `notify` crate（被任务明确禁止）
  B. 使用 `std::fs::metadata` mtime 轮询
  C. 通过 IPC 调用前端 watch() API
- **选择**: B — mtime 轮询
- **理由**:
  - 无需额外依赖
  - 跨平台兼容（由 std 处理）
  - 轮询间隔可配置，满足 3 秒内触发的要求
  - 代码简单，易于测试和维护
- **代价**: 相比 native notify 的延迟稍高，但 500ms 轮询对文件变化场景足够

## 决策 2: FileWatcher API 设计

- **模式**: Builder + start/stop
- **调用方式**: 通过 `start(on_event)` 传入回调，返回 `JoinHandles`
- **线程安全**: `stop()` 通过 `Arc<AtomicBool>` 信号通知
- **移除**: `remove()` 仅影响下次 start 调用，运行时路径集合不可变
- **理由**: 简化线程安全模型，start 时快照路径集合，避免运行时竞争

## 决策 3: 路径比较方式

- **方案**: 规范化路径后比较（canonicalize）
- **理由**: 避免 "/tmp/foo/../bar" 和 "/tmp/bar" 被视为不同的路径

## 决策 4: Cargo.toml 分离

- 使用项目已有的 Cargo.toml（不创建独立的 watcher crate）
- watcher 作为 `ptv_lib` crate 内的子模块 `crate::watcher`
- 后续被 collectors/template 通过 `use crate::watcher::FileWatcher` 调用
