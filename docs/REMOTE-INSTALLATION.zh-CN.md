# 远端组件安装

[English](REMOTE-INSTALLATION.md) | [简体中文](REMOTE-INSTALLATION.zh-CN.md)

Warpai 1.5.5 使用安装在远端 SSH 账号下的组件。每台远端主机安装一次，之后在 Warpai 终端中正常连接：

```sh
ssh user@host
cd /absolute/path/to/project
```

也可以使用已有的 SSH 主机昵称。无需填写 SSH alias、项目地址或 companion 地址。
认证、跳板机和主机密钥确认由系统 OpenSSH 处理。
Warpai 根据已确认的远端 shell 会话识别目录；仅安装组件不会让不透明的 SSH 会话自动报告 cwd。
面板提示时，请启用 Warpai 的 SSH shell 集成。

## 在远端主机安装

选择与**远端**系统、架构相符的安装包，并使用登录 SSH 的同一个账号安装。

| 远端系统 | 下载 | 安装方式 |
| --- | --- | --- |
| Linux x64 | [独立安装包](https://github.com/OthinusG/warpai/releases/download/v1.5.5/WarpaiCompanion-4.0.0-linux-x64.run) | 运行 `sh WarpaiCompanion-4.0.0-linux-x64.run`。 |
| macOS Apple 芯片 | [DMG](https://github.com/OthinusG/warpai/releases/download/v1.5.5/WarpaiCompanion-4.0.0-macos-arm64.dmg) | 打开镜像，双击 **Install Warpai Companion.command**。 |
| Windows x64 | [EXE 安装器](https://github.com/OthinusG/warpai/releases/download/v1.5.5/WarpaiCompanion-4.0.0-windows-x64-setup.exe) | 使用远端 SSH 账号运行安装器。 |

安装包包含对应的运行组件、校验和、版本与源码清单及许可说明，无需手动解压或填写执行路径。
Unix 安装器会设置执行权限，Windows 使用当前账号安装方式。默认位置固定为：

- Linux/macOS：`~/.config/.warpai/bin/warpai-companion`
- Windows：`%USERPROFILE%\.config\.warpai\bin\warpai-companion.exe`

重复运行新版安装包可在原位置升级。桌面端和远端组件应来自同一个 release；绑定项目前会检查协议兼容性。

## 连接与排错

Companion 4.0.0 自带私有 Git。文件操作、Agent 协作和 MCP 桥接由编译后的组件处理，
代码、文本和 Markdown 预览由桌面端渲染。Warpai 基础功能无需在远端另装 Git、Python、
Node、tmux 或 socat；第三方 Agent CLI 仍使用自己的运行环境和认证。
SSH/SFTP 和会话级 shell 集成仍是连接前提。私有 Git 不修改账号 PATH，也不替换系统 Git。
可使用已安装组件的绝对路径执行 `--check-runtime` 检查内置运行环境。

选中 SSH 终端，通过 `cd` 进入项目。面板根据已确认的远端用户目录、系统和工作目录寻找并检查组件。
组件缺失、无法执行或协议不兼容时，使用对应安装包安装或升级，再点击 **Reconnect**。
切换主机、账号、终端或项目时，旧响应不能污染当前项目；断开后的缓存状态不可写入。

Companion 按需启动；所有连接和活动 Agent 结束后，空闲 60 秒自动退出。
安装器不新增系统常驻服务，不修改 shell 启动文件或 PATH，
不复制凭据、不修改 SSH 配置，也不替换第三方 Agent 命令。远端 CLI Agent 仍通过自己的工具安装和认证。

## 启动远端 Agent

选择远端项目后，协作面板会连接到该项目。要让 Agent 加入协作，在远端终端使用
已安装组件的 `agent` 入口；普通远端 Agent 命令不会被自动改写。

例如，在 Linux/macOS 中，原生 Codex 可执行文件位于 PATH 时：

```sh
"$HOME/.config/.warpai/bin/warpai-companion" agent "$PWD" codex "$(command -v codex)"
```

参数顺序为 `agent <项目绝对路径> <程序名> <原厂可执行文件绝对路径> [原有参数...]`。
项目路径须与 `cd` 选中的目录一致，使用原厂安装的可执行文件。
Windows PowerShell 使用 `& "$env:USERPROFILE\.config\.warpai\bin\warpai-companion.exe" agent`，
随后传入相同顺序的参数和 Windows 原生绝对路径。Agent 保持交互式终端输入输出，
通过现有远端 MCP 适配器参与协作；不新增别名、不修改 PATH，也不替换原厂安装。

协作面板的 Project 模式用于当前项目内通信。需要同一仓库的并行开发时，切换到 Worktree，
选择活动 Coordinator，创建 worktree，并在对应目录通过上述入口启动工作 Agent，再在面板绑定。
创建 worktree 不会启动 Agent。团队成员必须位于同一 SSH 主机、账号和实际 Git 仓库；
Coordinator 分派任务、安排交叉审阅，并在 SSH 终端中完成集成与最终 Git 提交。

## Windows 原生 SSH

登录到远端 PowerShell 提示符后，在 SSH 终端提示条选择
**Integrate PowerShell**，再用 `cd` 进入项目。这只启用当前会话的集成，
不会修改 PowerShell 配置文件。Windows 原生 OpenSSH 不支持连接复用；
companion 控制连接沿用原连接的目标与选项，由系统 SSH 密钥或密钥代理
认证。仅用密码登录时，第二条非交互连接可能无法认证；终端仍可使用，
面板会说明原因并提供重连操作。Warpai 不保存密码。

参考：[Microsoft Win32-OpenSSH 功能范围](https://github.com/PowerShell/Win32-OpenSSH/wiki/Project-Scope)。

## Windows 直接接入 WSL（1.5.8）

直接 WSL 接入需要 Warpai 1.5.8 和 Companion 5.0.0。在目标发行版中，使用实际工作的
Linux 用户安装对应的 Linux x64 Companion。4.0.0 不提供 WSL 文件通道。
从[对应版本的 Warpai release](https://github.com/OthinusG/warpai/releases)获取 Linux 安装包；
Windows Companion 安装器不会将组件安装进 WSL。

在 Windows Warpai 终端中明确选择发行版和用户：

```powershell
wsl.exe --distribution Ubuntu --user your-linux-user
```

随后在 Linux shell 中安装、检查组件并进入项目：

```sh
sh WarpaiCompanion-5.0.0-linux-x64.run
"$HOME/.config/.warpai/bin/warpai-companion" --check-runtime
cd /absolute/path/to/project
```

Warpai 根据已确认的 WSL shell 信息选择 Linux 项目，文件浏览、编辑、预览、Review
和协作共用现有工作流。文件通过 Companion 的有界 stdio 分块通道传输，无需 SSH 服务、
密钥或 SFTP；在 WSL 内继续连接 SSH 时，仍使用原有 SSH 通道。

启用 Agent communication 后，Bash、Zsh、Fish 中识别出的普通 Agent 启动命令通过
Linux Companion 加入协作。Agent 的安装与认证留在 Linux；已有别名、函数和管理命令
保留原行为。也可以使用上面的显式 `companion agent` 入口。
Codex 的 `--cd`/`-C` 会绑定实际项目，同时保留原启动目录，避免改变其他相对路径参数。

项目模式和 Coordinator 按 Linux 用户与实际仓库持久化，同仓库 worktree 共享项目身份。
切换 pane、tab 或面板连接不会清除仍在运行的 Coordinator。其他发行版、用户和 Windows
本地项目保持隔离。WSL 停止后须显式重连，旧进程权限不能复用；缺少组件或版本过旧时，
安装对应的 Linux 组件后重连。
