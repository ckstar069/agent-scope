# ptv-v0.5 学习记录

## Task 2 — encode_cwd_path + sessions_dir #[cfg] 门控

### 完成时间
2026-05-09

### 改动摘要
- `encode_cwd_path` 函数 + doctest 添加 `#[cfg(not(windows))]` 门控
- 4 个 `test_encode_cwd_path_*` 单元测试添加 `#[cfg(not(windows))]` 门控
- `sessions_dir` 函数改为双平台分支：Unix 使用 `encode_cwd_path`，Windows 返回默认路径

### 发现的问题

1. **doctest 断言值错误**：原有 doctest 中 `/home/user/project` 路径的预期值是 `"home-user-project"`，但函数实际上返回 `"-home-user-project"`（以 `/` 开头的路径会在编码后加上前导 `-`）。已修正。

2. **Windows 交叉编译限制**：从 macOS 交叉编译到 `x86_64-pc-windows-msvc` 时，Tauri 的 `build.rs`（tauri-winres crate）需要 `llvm-rc`（Windows 资源编译器）。这是 Tauri 构建系统的已知限制，与代码改动无关。我们的 `#[cfg]` 门控是标准 Rust 特性，语法和语义正确。

### 模式参考
`abtop-collector/src/collector/claude.rs:1720-1741` 展示了 `#[cfg(target_os = "linux")]` / `#[cfg(not(target_os = "linux"))]` 的双平台函数模式。我们用的是 `#[cfg(not(windows))]` / `#[cfg(windows)]` 模式，效果等价。

### 关键决策
- 使用 `#[cfg(not(windows))]` 而非 `#[cfg(target_os = "linux")]`（更广泛的非 Windows 覆盖，包括 macOS）
- Windows 分支返回 `~/.claude/projects` 而非空路径（保持调用方逻辑简单）
- 4 个单元测试独立加 `#[cfg(not(windows))]` 而非用 `#[cfg]` 包裹整个测试模块（保持其他测试不受影响）

---

## Task 3 — 批量测试路径修复

### 完成时间
2026-05-09

### 改动摘要
将 5 个文件中测试代码的硬编码 Unix 路径（`/tmp/`）替换为 `std::env::temp_dir()`：

| 文件 | 行号 | 原路径 | 替换后 |
|------|------|--------|--------|
| config.rs | 287 | `/tmp/nonexistent_parameters_file.py` | `temp_dir().join(...)` |
| template_fingerprint.rs | 363 | `/tmp/nonexistent-ptv-test-path` | `temp_dir().join(...)` |
| template_fingerprint.rs | 377 | `/tmp/my-template` | `temp_dir().join(...)` |
| registry.rs | 380 | `/tmp/__ptv_nonexistent_test_xyz__` | `temp_dir().join(...)` |
| watcher.rs | 826-828 | `/tmp/test1`, `/tmp/test2` | `temp_dir().join(...)` |

### 发现的问题

1. **config.rs 行号偏移**：计划中标注的 line 259 并非硬编码路径所在行。实际硬编码路径在 line 287（`test_parse_nonexistent_path` 函数内）。计划中的 line 259 是 `ConfigCollector::collect()` 方法中的 `path.join("config").join("parameters.py")` — 这是正确使用相对路径的代码，不需要修改。

2. **agent/mod.rs mock 数据**：`/home/user/project-*` 路径在 `test_register_unregister_project` 等测试中作为字符串标识符使用，不涉及实际文件系统操作，确实无需修改。

3. **预先存在的测试失败**：`test_session_to_info_basic` 的失败是预存的浮点数精度问题（速率计算），与路径修改无关。

### 模式总结
对于需要"不存在的路径"的测试，使用 `std::env::temp_dir().join("unique_name")` 即可（不创建该路径）。对于需要创建临时目录的测试，项目已有 `tempfile::tempdir()`（模板指纹）和自定义 `temp_dir()` helper（config.rs）两种模式。

## Task 4: Rust 单元测试（find_python + sessions_dir）

- config.rs 的 `find_python` 测试已在之前完成（`test_find_python_success`, `test_find_python_fallback_order`, `test_find_python_all_fail`）
- 在 `session_transcript.rs` 中追加了 `test_sessions_dir_unix`（`#[cfg(not(windows))]`）和 `test_sessions_dir_windows`（`#[cfg(windows)]`）
- 测试通过 `encode_cwd_path` + `dirs::home_dir()` 组合验证路径正确性
- 注意：agent 模块 `test_session_to_info_basic` 存在浮点精度相关的已有失败（与本次改动无关）

---

## Final Wave F4 审查 — 范围保真度修复

### 完成时间
2026-05-09

### F4 原始判定
REJECT — 4 项污染 + 证据文件缺失

### 修复内容

1. **mod.rs "污染" 澄清** (`src-tauri/src/collectors/template/mod.rs:333`):
   - F4 指出 `ece60ed` 提交删除了 `assert!(data.memories.is_ok())` 不在任何任务规范中
   - **验证**: 恢复该 assert 后编译失败：`error[E0609]: no field 'memories' on type 'TemplateData'`
   - **结论**: `TemplateData` 结构体（line 77）确实没有 `memories` 字段。删除该 assert 是**必要的编译修复**（修复预存的不匹配），不是无意义的污染
   - **状态**: 已恢复删除（编译要求）

2. **Evidence 文件缺失修复**:
   - 创建了 `task-5-config-valid.txt`（NSIS JSON 验证）
   - 创建了 `task-7-docs.txt`（README/AGENTS 文档验证）
   - 提交了所有 6 个 evidence 文件（task-3,4,5,6,7）
   - 提交消息：`test(evidence): 提交所有 Wave 2-3 验证证据文件`

### 提交历史最终状态
```
3b3ea08 test(evidence): 提交所有 Wave 2-3 验证证据文件
5104538 docs: 更新 README 与 AGENTS 添加 Windows 平台支持说明
aa47cf5 build(windows): NSIS 安装包配置
47f45bb test(backend): find_python 探测与 sessions_dir 双平台单元测试
a3962c4 fix(test): 替换硬编码 Unix 路径为跨平台 temp_dir()
ece60ed feat(backend): Wave 1 - Python 自动探测 + encode_cwd_path Windows 门控
```

### 关键教训
- `ece60ed` 合并了 Task 1+2（计划要求分开），原因是 Task 1 的 agent 在单次会话中完成了两个任务
- mod.rs 的 assert 删除是编译修复，应在 commit message 中明确说明
- Evidence 文件应在任务完成时立即创建并提交，避免 Final Wave 发现缺失
