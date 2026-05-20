# 软件发布页 Artifact 堆积修复规则

> 本文档定义发布页后端过滤旧版本 artifact 的三层规则，供发布页项目参考实施。
> 适用场景：GitLab CI 构建产物过多，发布页展示大量历史版本文件。

---

## 一、问题根因

```
refresh_project()
  → GitLab /jobs?scope=success&per_page=100
  → 返回最近 100 个成功 job（跨多个版本）
  → 每个 job 带 2-4 个 artifact 文件
  → 数据库全量缓存，不做版本过滤
  → 发布页展示 119 个文件（Linux 88 + Windows 31）
```

GitLab Releases 页面只显示通过 Git tag 手动创建的 release，而 CI job artifact 是构建产物，两者**不自动关联**。因此 Releases 页面只有 5 个，但 artifact 文件有上百个。

**额外问题：同一版本内的重复构建**

同一版本（如 `0.2.2`）可能因 retry/rebuild 产生多个成功 job，每个 job 产出相同文件名的 artifact。如果不做文件名级去重，发布页会展示重复文件（如 `AgentScope_0.2.2_x64-setup.exe` 出现 2 次，分别来自 job 598 和 599）。规则 1 已包含按文件名去重逻辑。

---

## 二、三层过滤规则

### 规则 1 — 版本号解析与平台分组（后端核心过滤）

**目标**：每个平台只保留**最高版本号**的 artifact；同一版本内同一文件名只保留**最新构建**（最大 job_id），历史版本和重复构建自动丢弃。

**版本号提取**：

```python
# 文件名示例：
#   AgentScope_0.2.14_amd64.AppImage    → version=0.2.14, platform=linux
#   AgentScope_0.2.14_x64-setup.exe     → version=0.2.14, platform=windows

import re

def extract_version(filename: str) -> str | None:
    match = re.search(r'AgentScope_(\d+\.\d+\.\d+(?:-rc\.\d+)?)_', filename)
    return match.group(1) if match else None
```

**平台识别**：

| 平台 | 文件扩展名 |
|------|-----------|
| Linux | `.AppImage`, `.deb`, `.tar.gz` |
| Windows | `.exe`, `.msi` |

```python
def detect_platform(filename: str) -> str:
    linux_exts = ['.AppImage', '.deb', '.tar.gz']
    windows_exts = ['.exe', '.msi']
    if any(ext in filename for ext in linux_exts):
        return 'linux'
    if any(ext in filename for ext in windows_exts):
        return 'windows'
    return 'unknown'
```

**语义化版本排序**：

```python
from functools import cmp_to_key

def semver_key(version: str):
    """
    返回可比较的元组。
    正常版本: [major, minor, patch, 1]
    RC 版本:  [major, minor, patch, 0, rc_num]  (RC 优先级低于正式版)
    """
    parts = version.split('-')
    nums = list(map(int, parts[0].split('.')))
    if len(parts) == 1:
        return nums + [1]  # 正式版标记
    # RC 版本，如 "0.2.14-rc.1"
    rc_num = int(parts[1].replace('rc.', ''))
    return nums + [0, rc_num]

# 测试版本黑名单
BLACKLIST_VERSIONS = {'99.99.99'}
```

**过滤逻辑**：

```python
from collections import defaultdict

def filter_latest_per_platform(artifacts):
    """
    输入: 所有 artifact 对象列表（含 filename, job_id 字段）
    输出: 每个平台最高版本的 artifact 列表，同一文件名去重
    """
    by_platform = defaultdict(list)

    for artifact in artifacts:
        version = extract_version(artifact.filename)
        if not version or version in BLACKLIST_VERSIONS:
            continue
        platform = detect_platform(artifact.filename)
        by_platform[platform].append((version, artifact))

    filtered = []
    for platform, items in by_platform.items():
        # 按版本号降序排序
        items.sort(key=lambda x: semver_key(x[0]), reverse=True)
        latest_version = items[0][0]

        # 保留该平台的最新版本
        latest_artifacts = [a for v, a in items if v == latest_version]

        # 同一文件名去重：保留 job_id 最大的记录（处理同一版本的重复构建）
        by_filename = {}
        for artifact in latest_artifacts:
            filename = artifact.filename
            if filename not in by_filename or artifact.job_id > by_filename[filename].job_id:
                by_filename[filename] = artifact

        filtered.extend(by_filename.values())

    return filtered
```

---

### 规则 2 — Release 标签对齐（可选增强）

**目标**：只展示有对应 GitLab Release 的版本，避免展示测试构建或内部版本。

```python
def filter_by_release_tags(artifacts, project_id: int, gitlab_client):
    """
    只保留有对应 GitLab Release tag 的 artifact。
    """
    releases = gitlab_client.get(f'/projects/{project_id}/releases?per_page=100')
    release_tags = {r['tag_name'] for r in releases}

    filtered = []
    for artifact in artifacts:
        version = extract_version(artifact.filename)
        if not version:
            continue
        tag = f'v{version}'
        if tag in release_tags:
            filtered.append(artifact)

    return filtered
```

**使用方式**（双条件组合）：

```python
def refresh_project():
    jobs = gitlab.get('/jobs?scope=success&per_page=100')
    all_artifacts = extract_artifacts_from_jobs(jobs)

    # 第一层：版本去重
    filtered = filter_latest_per_platform(all_artifacts)

    # 第二层（可选）：Release 对齐
    # filtered = filter_by_release_tags(filtered, project_id, gitlab)

    # 写入数据库：先清空，再写入
    db.artifacts.clear()
    db.artifacts.save_all(filtered)
```

---

### 规则 3 — CI 侧 artifact 生命周期

已在 AgentScope 项目 `.gitlab-ci.yml` 中将 `expire_in` 从 `1 month` 改为 `1 day`：

```yaml
build:linux:
  artifacts:
    expire_in: 1 day    # 原为 1 month

build:windows:
  artifacts:
    expire_in: 1 day    # 原为 1 month
```

新构建的 artifact 会在 1 天后自动过期。但这**不影响已缓存到发布页数据库的历史数据**，因此必须配合规则 1/2 的后端过滤。

---

## 三、数据层清理建议

### 一次性清理（数据库层面）

```sql
-- 示例：标记每个平台的最新版本
-- 具体 SQL 取决于表结构

-- 方案 A：添加 is_latest 标记
ALTER TABLE artifacts ADD COLUMN is_latest BOOLEAN DEFAULT FALSE;

-- 方案 B：直接删除旧版本（风险较高，建议先备份）
-- DELETE FROM artifacts WHERE version != (SELECT MAX(version) ...);
```

### 推荐方案：按平台+版本分组表结构

若重新设计表结构，建议增加 `platform` 和 `version` 字段：

```sql
CREATE TABLE artifacts (
    id INTEGER PRIMARY KEY,
    filename TEXT NOT NULL,
    version TEXT NOT NULL,
    platform TEXT NOT NULL,  -- linux / windows
    download_url TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_latest BOOLEAN DEFAULT FALSE
);

-- 查询时直接过滤
SELECT * FROM artifacts WHERE is_latest = TRUE;
```

---

## 四、前端展示建议

后端过滤后，同一版本的 Linux 平台可能仍有多个格式（AppImage + deb + tar.gz），前端可按平台分组展示：

```
┌─ Linux (v0.2.14) ─┐
│ AppImage   [下载] │
│ deb        [下载] │
│ tar.gz     [下载] │
└───────────────────┘

┌─ Windows (v0.2.14) ┐
│ setup.exe  [下载]  │
└────────────────────┘
```

---

## 五、验证 Checklist

- [ ] `refresh_project()` 执行后，数据库 artifact 数量合理（每个平台 2-3 个格式）
- [ ] 文件名中包含旧版本号（如 `0.2.1`、`99.99.99`）的 artifact 不再出现
- [ ] RC 版本（如 `0.2.14-rc.1`）在正式版 `0.2.14` 发布后不再展示
- [ ] **同一版本、同一文件名只出现一次**（如 `AgentScope_0.2.2_x64-setup.exe` 不重复）
- [ ] 重复构建时保留的是最新 job_id 的记录
- [ ] 新 tag 发布并构建成功后，自动替换为最新版本 artifact
- [ ] GitLab Releases 页面数量与发布页展示版本一致

---

## 六、相关文档

| 文档 | 路径 | 说明 |
|------|------|------|
| CI/CD 设置 | `docs/ci-cd-setup.md` | AgentScope CI/CD 配置 |
| 桌面端 CI/CD 经验 | `docs/desktop-ci-cd-lessons.md` | artifact 生命周期管理等踩坑记录 |
