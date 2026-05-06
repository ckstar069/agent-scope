# F2 代码质量审查 — 发现的问题

## 阻塞性问题: 无

## 非阻塞性问题

### 1. clippy `type_complexity` 警告
- **文件**: `src-tauri/src/collectors/template/mod.rs:214`
- **类型**: `Arc<Mutex<Option<(Instant, Vec<PathBuf>)>>>`
- **严重度**: 低 (代码风格建议)
- **修复建议**: 提取 type alias

### 2. console.warn 残留
- **文件**: `src/pages/ProjectDetail.tsx:277, 282`
- **内容**: 实时监听失败时的警告日志
- **严重度**: 低 (可接受，后续可迁移到 tracing)

### 3. TODO 标记
- **文件**: `src-tauri/src/watcher.rs:407`
- **内容**: `// TODO: 在集成到 Tauri 后替换为 log/tracing`
- **严重度**: 低 (技术债务标记，非功能缺陷)
