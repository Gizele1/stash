# Stash

> git stash for your brain

[![CI](https://github.com/Gizele1/stash/actions/workflows/ci.yml/badge.svg)](https://github.com/Gizele1/stash/actions/workflows/ci.yml)

A desktop app that automatically captures and visualizes your [Claude Code](https://claude.ai/code) work sessions — surfacing them as managed tasks with evolving intent, agent branches, and drift markers.

---

## Features

- **Auto session capture** — polls `~/.claude/projects/` every 5 s; no manual interaction needed
- **Intent versioning** — append-only intent history; the full evolution of your thinking is always preserved
- **Task management** — park, resume, and switch between work sessions without losing context
- **Agent branch tracking** — track what each agent did, its progress, and output
- **Drift markers** — mark where work veered from the original intent
- **Resume notes** — auto-generated or manual context for picking up parked tasks
- **Intent graph** — interactive DAG view of intent evolution and agent branches

## Quick Start

**Prerequisites:** Node.js 20+, Rust 1.77+ ([rustup](https://rustup.rs))

Linux users also need:

```sh
sudo apt install libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

**Run:**

```sh
git clone https://github.com/Gizele1/stash.git
cd stash
npm install
npm run tauri   # starts full desktop app (first Rust build takes a few minutes)
```

## Documentation

| Doc | Description |
|-----|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Domain map, IPC layer, key design decisions |
| [AGENTS.md](AGENTS.md) | Agent orientation map, layer rules, quick-reference commands |
| [docs/guides/setup.md](docs/guides/setup.md) | Full dev environment setup |
| [docs/guides/testing.md](docs/guides/testing.md) | Testing guide |
| [docs/SECURITY.md](docs/SECURITY.md) | Threat model, secrets handling |

## Contributing

```sh
npm test                    # frontend tests + architecture boundary check
cd src-tauri && cargo test  # Rust tests
npm run lint && npm run typecheck
cd src-tauri && cargo clippy
npm run gc                  # doc drift + architecture violation scan
```

See [docs/guides/setup.md](docs/guides/setup.md) for the full contributor workflow and [docs/golden-principles/](docs/golden-principles/) for code conventions.

---

# Stash

> 大脑的 git stash

[![CI](https://github.com/Gizele1/stash/actions/workflows/ci.yml/badge.svg)](https://github.com/Gizele1/stash/actions/workflows/ci.yml)

一个桌面应用，自动捕获并可视化你的 [Claude Code](https://claude.ai/code) 工作会话，将它们呈现为带有意图演变、Agent 分支和漂移标记的可管理任务。

---

## 功能特性

- **自动会话捕获** — 每 5 秒轮询 `~/.claude/projects/`，无需手动操作
- **意图版本管理** — 追加式意图历史，完整保留你的思路演变过程
- **任务管理** — 暂存、恢复、切换工作会话，不丢失上下文
- **Agent 分支追踪** — 追踪每个 Agent 的行为、进度和输出
- **漂移标记** — 标记工作偏离原始意图的位置
- **恢复备注** — 自动生成或手动添加的暂存任务恢复上下文
- **意图图谱** — 交互式 DAG 视图，展示意图演变与 Agent 分支

## 快速开始

**前提条件：** Node.js 20+、Rust 1.77+（[rustup](https://rustup.rs)）

Linux 用户还需安装：

```sh
sudo apt install libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev
```

**运行：**

```sh
git clone https://github.com/Gizele1/stash.git
cd stash
npm install
npm run tauri   # 启动完整桌面应用（首次 Rust 编译需要几分钟）
```

## 文档

| 文档 | 说明 |
|------|------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | 领域图、IPC 层、关键设计决策 |
| [AGENTS.md](AGENTS.md) | Agent 导引图、层级规则、常用命令速查 |
| [docs/guides/setup.md](docs/guides/setup.md) | 完整开发环境配置 |
| [docs/guides/testing.md](docs/guides/testing.md) | 测试指南 |
| [docs/SECURITY.md](docs/SECURITY.md) | 威胁模型、机密处理 |

## 参与贡献

```sh
npm test                    # 前端测试 + 架构边界检查
cd src-tauri && cargo test  # Rust 测试
npm run lint && npm run typecheck
cd src-tauri && cargo clippy
npm run gc                  # 文档漂移 + 架构违规扫描
```

详见 [docs/guides/setup.md](docs/guides/setup.md) 完整贡献流程，以及 [docs/golden-principles/](docs/golden-principles/) 代码规范。
