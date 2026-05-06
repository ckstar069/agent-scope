# FileWatcher 集成 — 问题记录

## 已解决问题

### 1. thread::Builder::spawn() 错误类型不匹配

- **症状**: `?` 操作符无法将 `std::io::Error` 转换为 `Box<dyn Any + Send>`
- **原因**: `thread::Builder::spawn()` 返回 `Result<JoinHandle, io::Error>`，但函数的返回类型是 `thread::Result<JoinHandles>`（即 `Result<T, Box<dyn Any + Send>>`）
- **修复**: 使用 `.map_err(|e| Box::new(e) as Box<dyn std::any::Any + Send>)?` 手动转换

### 2. 测试辅助模块中未使用的 OnceLock 导入

- **症状**: 编译警告 `unused import: OnceLock`
- **原因**: 最初计划用于线程安全计数器但被 AtomicUsize 替代
- **修复**: 删除 `use std::sync::OnceLock;`

## 未解决问题

暂无。
