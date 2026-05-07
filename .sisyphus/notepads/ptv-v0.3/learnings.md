## Task 5：ProjectMemoryPanel L1 集成

- `MemoryFileTree` 依赖 `source_group` 命中 `root/rules/notepads/plans/drafts/docs`，后端需把 collector 返回的 `design/specs` 统一映射为 `docs`，否则设计/需求文档不会出现在文件树分组中。
- `MarkdownRenderer` 自带目录栏，因此 `ProjectMemoryPanel` 的右侧内容会形成「文件树 + 文档目录 + 正文」的嵌套阅读布局，应保持外层 `lg:grid-cols-[16rem_minmax(0,1fr)]` 与现有组件一致。
- `template-update` 事件 payload 含 `project_path`，面板内监听时需要按当前项目路径过滤，避免其他项目更新误标记当前文件树。

## Task 6：session_transcript.rs 采集器

- JSONL 每行必须是一个完整 JSON 对象（单行），多行 raw string 会被 `write!` 拆成多行导致解析失败。复杂 JSON 需在一行内写完。
- `encode_cwd_path` 需要与 OpenCode/Claude Code 的项目目录命名规则一致：去首 `/`，替换 `/` → `-`。注意与 abtop-collector 版本区分（后者还替换 `_`、`.`）。
- 连续同角色轮次合并使用 pending buffer 模式：相同 role 追加文本 + 合并工具，不同 role 先 flush 再创建新的。flush 发生在角色切换和文件解析结束时。
- 时间戳解析需要处理两种格式：纯数字（Unix 毫秒）和 ISO 8601 字符串。为减少依赖，使用手动日期计算替代 chrono crate。
- 元数据模式（`build_turns=false`）复用同一解析管道，只跳过 turn 构建但不跳过 metadata 提取，避免两套解析逻辑。
- 测试中 `create_mock_jsonl` 辅助函数按行写入，每个 entries 数组元素对应一行 JSONL，而非自由格式的多行 JSON。
