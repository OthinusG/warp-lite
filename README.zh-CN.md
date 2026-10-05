<div align="center">

[English](README.md) | **简体中文**

# Warpai

<img src="app/assets/branding/warpai.png" alt="Warpai" width="160" height="160">

### 围绕你的代码与 Agent 展开的终端工作空间。

面向 macOS 和 Windows 的原生、本地优先终端。
在一个工作空间中运行命令、浏览项目，并协调你独立安装的 CLI Agent。

**命令分块 · GPU 渲染 · 垂直标签页 · Agent 协作 · SSH 项目**

[下载 1.0.1](#get-warpai) · [快速开始](#start-working) ·
[Agent 使用与兼容性](docs/AGENTS.zh-CN.md) ·
[反馈问题](https://github.com/OthinusG/warpai/issues)

[![版本](https://img.shields.io/github/v/release/OthinusG/warpai)](https://github.com/OthinusG/warpai/releases/tag/v1.0.1)
[![许可证：AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE-AGPL)
[![桌面平台](https://img.shields.io/badge/desktop-macOS%20%7C%20Windows-24292f.svg)](#get-warpai)

</div>

Warpai 将开发工作集中在项目附近：Shell 负责执行，编辑器提供代码上下文，协作面板跟进 Agent 任务。
继续使用自己安装和维护的 Git、包管理器、构建工具与 CLI Agent。
打开应用即可开始，无需 Warpai 账号或订阅。

![Warpai macOS 工作空间：垂直标签页、源码编辑器与 Agent 任务面板](docs/images/warpai-workspace.png)

*macOS 原生界面，采用垂直标签页和默认 Claude Warm Light 主题。
项目与 Agent 为测试样例；[查看截图来源](docs/images/README.zh-CN.md)。*

## 为什么选择 Warpai？

| 优势 | 日常使用中的价值 |
| --- | --- |
| **命令历史清晰可读** | 每条命令与输出保留在同一个命令块中，方便回看构建结果、错误和历史输出。 |
| **围绕项目的一体化空间** | 终端、文件、Markdown 与 Agent 任务触手可及；垂直标签页和分屏让并行工作各有位置。 |
| **自己选择 Agent 与服务商** | 使用独立安装的 Codex、Claude Code、QoderCN 等工具，自行选择服务商、认证方式和权限设置。 |
| **有记录的协作流程** | 在项目内互发消息、委派任务、提交验证材料并审阅结果。 |
| **掌握数据与运行位置** | 本地协作在本机运行；SSH 协作在选定的远程账号和项目中运行，无需 Warpai 托管服务。 |

## 终端与项目工具

- **GPU 渲染终端：**命令块、标签页与分屏，适用于交互 Shell、开发服务器及持续运行的构建。
- **项目浏览器与编辑器：**浏览文件、查看源码、使用 Vim 模式并阅读 Markdown，保留终端上下文。
- **命令面板与补全：**查找操作，减少日常 Shell 工作中的重复输入。
- **外观设置：**选择浅色或深色主题、字体和标签页布局，默认采用 Claude Warm Light。
- **Keep awake（保持唤醒）：**当前会话的开关在被跟踪的 Agent 工作时阻止空闲系统睡眠。屏幕仍可休眠；重启应用后开关重置。

## Agent 协作

让一个 Agent 实现功能，另一个 Agent 审查结果。
协作面板集中展示消息、任务状态、验证材料与审阅决定；提交结果后，还需审阅才能验收通过。

```mermaid
flowchart LR
    A[明确任务] --> B[委派给 Agent]
    B --> C[执行并提交验证材料]
    C --> D[审阅]
    D --> E[验收通过]
    D -->|要求修改| C
```

参与者共享同一个规范化项目范围，不相关的项目保持隔离。
排队任务保留输入草稿，并遵循 Agent 原生就绪状态；Warpai 不代替你回答 Agent 的权限请求。

协作需要已安装 Agent 版本支持原生 MCP 客户端。
支持的命令、配置管理方式与验证范围见 [中文 Agent 指南](docs/AGENTS.zh-CN.md)。

<a id="start-working"></a>
### 快速开始

1. [安装 Warpai](#get-warpai)。
2. 使用各 Agent 自己的工具完成安装和认证。
3. 打开项目，进入 **Settings > Features > Agent communication**，开启协作并选择允许参与的 Agent。
4. 在该项目中新开 Agent 会话；已有会话可能需要重启才能加载 MCP 配置。
5. 打开 **Agent collaboration**，发送消息、委派任务并审阅结果。

### Codex：继续输入原来的命令

在已开启协作的 Warpai 本地终端中正常启动：

```sh
codex
codex --yolo
codex resume --last
```

Warpai 自动为符合条件的交互启动添加仅本次会话生效的 MCP 参数与 `--no-daemon`，
使不同终端使用独立的原生后台。原有参数、项目目录、服务商与权限选择继续生效。

已安装 Codex 必须支持 `-c` 和 `--no-daemon`。
Warpai 每次启动都重新解析并探测当前安装，因此仍可通过原包管理器更新。
不替换命令、不复制临时可执行文件、不修改 PATH 或 CODEX_HOME，也不读写 Codex 配置。
不支持的版本按原样启动，并显示 MCP 提示。

帮助、登录、MCP 管理、批处理、显式远程连接、用户别名和 WSL 保持原生行为。
受管理的 SSH 项目使用下方 companion 入口；在 Warpai 外执行的命令不受影响。

<a id="work-on-remote-projects"></a>
## 在远程项目中工作

通过协作面板管理 **Linux、macOS 或 Windows** 上的 SSH 项目。
同一远程账号、同一项目中的 Agent 共享协作状态，桌面显示远程位置、消息、任务与连接状态。

![Warpai SSH 项目：已连接的远程 Agent 与任务](docs/images/warpai-ssh-project.png)

*macOS 原生界面、垂直标签页与 Claude Warm Light 主题，展示受控 SSH 样例项目。截图来自实际运行的应用。*

1. 在系统 OpenSSH 客户端中配置可信的主机别名与可用的非交互认证。
2. 下载匹配的 [1.0.1 companion](#remote-companion)，在远程账号下解压；Unix 主机需赋予二进制执行权限。在该账号内安装并认证 CLI Agent。
3. 打开 **Agent collaboration > Connect SSH project**，填写 SSH 别名、项目绝对路径和 companion 绝对路径。
4. 在普通 SSH 终端中，用相同项目根目录启动受管理的 Agent：

```sh
/opt/warpai/warpai-companion agent /srv/project codex /usr/local/bin/codex
```

按远程安装位置替换示例路径。Windows 主机使用 `warpai-companion.exe` 与原生绝对路径。
关闭 SSH 终端会停止其拥有的 Agent 运行。断线后保留草稿、标记远程状态过期，重连前禁止写操作。
桌面凭据不会复制到远程账号；文件传输采用 SFTP。

每个 SSH 项目有独立的协作权限范围。具体行为见 [SSH 技术说明（英文）](specs/agent-communication-v2/TECH.md)。

<a id="get-warpai"></a>
## 下载与安装

**[Warpai 1.0.1](https://github.com/OthinusG/warpai/releases/tag/v1.0.1)** 是改名后的首个版本，
包含本地 Agent 协作、Codex 会话级 MCP、SSH 项目和新的应用图标。

| 平台 | 下载 | 安装 |
| --- | --- | --- |
| **macOS · Apple 芯片** | [DMG](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai.dmg) · [应用 ZIP](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai.app.zip) | 将 **Warpai.app** 拖入 Applications。 |
| **Windows · x64** | [安装器](https://github.com/OthinusG/warpai/releases/download/v1.0.1/WarpaiSetup-x64.exe) · [便携 ZIP](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai-windows-x64.zip) | 运行安装器，或解压 ZIP 后启动 **Warpai.exe**。 |

macOS 应用采用临时签名，尚未进行公证。如果首次启动被系统拦截，确认下载来源后，
在 **System Settings > Privacy & Security > Open Anyway** 中批准打开。
Windows 可能要求确认运行未签名安装器。发布页面提供校验和。

更新时下载新版本，替换应用或运行新版安装器，设置保留在应用数据目录中。
独立安装的 CLI Agent 继续使用各自的包管理器更新。

<a id="remote-companion"></a>
### 远程 companion

桌面应用与 companion 应来自同一个 release。压缩包清单包含准确源码版本、Rust 目标平台与 SHA-256 校验和。

| 远程主机 | 下载 |
| --- | --- |
| Linux x64（基于 Ubuntu 22.04 构建） | [Companion ZIP](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai-companion-x86_64-unknown-linux-gnu.zip) |
| macOS Apple 芯片 | [Companion ZIP](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai-companion-aarch64-apple-darwin.zip) |
| Windows x64 | [Companion ZIP](https://github.com/OthinusG/warpai/releases/download/v1.0.1/Warpai-companion-x86_64-pc-windows-msvc.zip) |

## 设置与本地控制

设置、主题、受管理的 MCP 配置及本地应用数据统一使用以下位置：

| 系统 | 目录 |
| --- | --- |
| macOS | `~/.config/.warpai` |
| Windows | `%USERPROFILE%\.config\.warpai` |

首次启动时仅导入缺失的历史设置，不覆盖新文件；旧目录保留，便于恢复。
第三方 Agent 继续使用自己的认证与设置目录。

Warpai 无需云端账号、登录门槛或订阅，默认产品排除捆绑云端 AI、计费与遥测入口。
Shell 命令、SSH 和所选 Agent 服务商仍会按照各自行为使用网络。

<a id="develop-and-contribute"></a>
## 开发与贡献

Warpai 使用 Rust 开发。贡献前阅读 [开发指南](docs/DEVELOPMENT.zh-CN.md)、
[Agent 规则（英文）](AGENTS.md) 与 [SSH 计划（英文）](specs/agent-communication-v2/PLAN.md)。
构建和发布直接使用仓库中已提交的源码。

验证覆盖 macOS、Windows 应用检查、原生 UI 操作与截图、三平台 companion 测试及受控 OpenSSH。
原版 macOS Codex 0.160.0 另已完成两个并发会话与合计四轮模型、MCP 调用。
详见 [验收记录（英文）](specs/agent-communication-v2/QUALITY.md)、
[发布规范（英文）](specs/RELEASE.md) 与 [Agent 兼容性指南](docs/AGENTS.zh-CN.md)。

反馈问题时请提供版本、操作系统、复现步骤与预期行为，省略凭据和私人项目内容。

<details>
<summary>贡献者必须保留的终端核心路径</summary>

保留以下渲染、输入与模型路径。将其整体替换为占位实现，不是有效的编译修复；需要同时验证终端行为。

- `app/src/terminal/view.rs`
- `app/src/terminal/input.rs`
- `app/src/terminal/block_list_element.rs`
- `app/src/terminal/view/`
- `app/src/terminal/input/`
- `app/src/terminal/local_tty/terminal_manager.rs`
- `app/src/terminal/alt_screen/alt_screen_element.rs`
- `app/src/terminal/model/`

</details>

## 致谢与许可证

Warpai 独立维护，源自 [terzigolu/warp-lite](https://github.com/terzigolu/warp-lite)
与 [Warp](https://github.com/warpdotdev/warp)，与上游 Warp 团队没有隶属或背书关系，保留原有版权与署名。

源码采用 [AGPL-3.0-only](LICENSE-AGPL)；原始 `warpui`、`warpui_core` 子库保留
[MIT 许可证](LICENSE-MIT)。出处与商标说明见 [fork notice（英文）](FORK_NOTICE.md)。
