## Task 5：ProjectMemoryPanel L1 集成

- 项目切换时在文件列表加载流程开始处清理旧 `files/selectedPath/content/changedPaths`，避免新增单独 reset effect 触发 React Hooks 依赖诊断，同时减少旧项目内容闪现。
- 选中文件内容加载时先捕获 `selectedPath` 为局部常量，再传给 Tauri command，保证 TypeScript 在异步函数内可稳定窄化为 `string`。
