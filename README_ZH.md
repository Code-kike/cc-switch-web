<div align="center">

# cc-switch-web

### 面向 Claude Code、Codex、Gemini CLI、OpenCode 和 OpenClaw 的 Web 优先远程管理工具

[![Platform](https://img.shields.io/badge/platform-Linux%20Server%20%7C%20Browser-lightgrey.svg)](#当前部署模式)
[![Built with Tauri](https://img.shields.io/badge/backend-Tauri%202%20Web%20Server-orange.svg)](https://tauri.app/)
[![Frontend](https://img.shields.io/badge/frontend-React%20%2B%20Vite-646cff.svg)](#开发)

[English](README.md) | 中文 | [日本語](README_JA.md)

</div>

## 概述

`cc-switch-web` 是 `cc-switch` 生态的 Web 优先（web-first）部署形态，聚焦于对本地 AI CLI 工具配置的**远程访问**、**常驻服务部署**与**浏览器管理**。

如果你在一台经常需要远程访问的机器上管理 Claude Code、Codex、Gemini CLI、OpenCode 或 OpenClaw，本项目让你通过 Web 界面进行管理，而无需依赖本地桌面应用。

## 致谢

本项目直接受益于两个上游项目：

- [`farion1231/cc-switch`](https://github.com/farion1231/cc-switch)：提供了成熟的产品基础、数据模型、供应商管理逻辑、多工具集成以及核心后端能力。
- [`Laliet/CC-Switch-Web`](https://github.com/Laliet/CC-Switch-Web)：展示了浏览器化的方向，验证了为 `cc-switch` 工作流提供远程 Web 管理的价值。

`cc-switch-web` 是上述两条工作线的一次务实融合：在持续跟进 `cc-switch` 新能力的同时，让它们可以通过远程可访问的 Web 部署模式来使用。

## 为什么做这个项目

原版 `cc-switch` 桌面应用在本地使用上很强大，但在以下场景并不理想：

- 你的主力机器通过 SSH 或远程桌面访问
- 你希望服务在重启后仍保持在线
- 你需要从局域网中的另一台设备通过浏览器访问
- 你希望在不启动桌面 GUI 的情况下管理供应商、提示词、MCP、技能与会话

本仓库专注于解决这一空缺。

## 提供的功能

- **Web 优先的管理界面**，支持 Claude Code、Codex、Gemini CLI、OpenCode 和 OpenClaw
- **远程浏览器访问** 自托管机器
- **Systemd 服务部署**，实现常驻运行与开机自启
- **复用现有的 `~/.cc-switch` 数据**，而非另建独立的数据孤岛
- **现代 `cc-switch` 功能基线**，不再局限于早期的 Web 原型
- **独立 Web 服务器运行时**，可部署于 Linux 服务器或工作站

Web 服务器模式主要用于远程配置与管理。你可以在浏览器中编辑供应商、提示词、MCP
服务器、技能、会话及相关设置；而需要控制本地代理运行时的操作仍仅限桌面端。在 Web
模式下，代理（Proxy）与故障转移（Failover）设置可作为配置项进行编辑，但启动本地代理
进程、运行时接管以及实时代理控制不会由独立服务器暴露。

## 当前部署模式

本项目目前针对自托管 Linux 场景进行优化。

典型部署流程：

1. 使用 `pnpm build:web` 构建 Web 前端
2. 使用 Cargo 构建独立服务器
3. 安装二进制文件与静态资源
4. 以 `systemd --user` 服务方式运行
5. 通过 `http://<host>:3010` 访问

在本仓库中，服务部署已经支持：

- 绑定地址 `0.0.0.0`
- 默认端口 `3010`
- `systemd --user` 开机自启
- 静态资源安装到 `~/.local/share/cc-switch-web/dist-web`
- 复用 `~/.cc-switch` 数据

## 仓库结构

- `src/`：React + Vite 前端
- `src-tauri/`：共享后端逻辑与独立 Web 服务器
- `deploy/systemd/`：用户级服务单元
- `scripts/install-cc-switch-web-service.sh`：用于常驻服务部署的构建与安装脚本
- `dist-web/`：生成的 Web 前端构建产物

## 开发

### 前端开发

```bash
pnpm install
pnpm dev:web
```

### Web 构建

```bash
pnpm build:web
```

### 独立 Web 服务器

```bash
cargo run --manifest-path src-tauri/Cargo.toml \
  --no-default-features \
  --features web-server \
  --example server
```

### 服务安装

```bash
./scripts/install-cc-switch-web-service.sh
```

### 服务管理

```bash
systemctl --user status cc-switch-web.service --no-pager
systemctl --user restart cc-switch-web.service
journalctl --user -u cc-switch-web.service -f
```

## 数据目录

当前服务部署默认复用：

```bash
~/.cc-switch
```

这意味着现有的供应商、提示词、技能、备份及相关数据可以继续被 Web 服务使用，而不会在重启时被重置，也不会默认拆分到第二个数据库中。

## 项目定位

本仓库并不试图在概念上替代上游项目。

它的角色更窄、也更务实：

- 跟进 `cc-switch` 的新功能
- 通过可远程使用的 Web 界面将其暴露出来
- 支持长期运行的自托管部署
- 缩小面向桌面的 `cc-switch` 与早期面向 Web 的原型之间的差距

## 状态

项目已具备可用的独立 Web 运行时与常驻服务部署路径，但在部分领域的功能对齐工作仍在进行中。主要方向是持续将 `cc-switch` 的新能力同步到 Web 体验中，并补齐剩余的管理功能空缺。

## 上游项目

- `cc-switch`: https://github.com/farion1231/cc-switch
- `CC-Switch-Web`: https://github.com/Laliet/CC-Switch-Web

## 许可证

本仓库目前遵循项目目录中所包含的许可证条款。在再分发或衍生使用前，请先查阅 `LICENSE` 文件。
