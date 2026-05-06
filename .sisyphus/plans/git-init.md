# Git 初始化 + 首次提交

## TL;DR
初始化 git 仓库，将所有已规划的文件（需求文档、工作计划、项目记忆草稿）提交为初始 commit。

## TODOs

- [ ] 1. 初始化 git 仓库 + 首次提交

  **What to do**：
  ```bash
  cd /Users/ckstar/Repo/ai_project_template_visualization
  git init
  git add .sisyphus/ AGENTS.md .gitignore 2>/dev/null
  git add .sisyphus/drafts/requirements.md
  git add .sisyphus/drafts/project-memory.md
  git add .sisyphus/plans/ptv-v0.1.md
  git add .sisyphus/plans/git-init.md
  git commit -m "初始化: ptv 项目需求文档与工作计划

  - 需求规格文档（10 章，30+ 条需求，Oracle VERIFIED）
  - v0.1 执行计划（954 行，16 任务，Momus OKAY）
  - 项目记忆草稿（技术栈/测试机/关键决策）
  - 平台: macOS + Linux，Linux 优先验证"
  ```

  **Commit**: YES
  - Message: `初始化: ptv 项目需求文档与工作计划`
  - Files: `.sisyphus/`, 相关 md 文件
