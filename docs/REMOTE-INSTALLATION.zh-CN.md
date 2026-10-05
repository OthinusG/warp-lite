# 远端组件安装

[English](REMOTE-INSTALLATION.md) | [简体中文](REMOTE-INSTALLATION.zh-CN.md)

Warpai 1.1.0 使用安装在远端 SSH 账号下的组件。每台远端主机安装一次，之后在 Warpai 终端中正常连接：

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
| Linux x64 | [独立安装包](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-linux-x64.run) | 运行 `sh WarpaiCompanion-linux-x64.run`。 |
| macOS Apple 芯片 | [DMG](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-macos-arm64.dmg) | 打开镜像，双击 **Install Warpai Companion.command**。 |
| Windows x64 | [EXE 安装器](https://github.com/OthinusG/warpai/releases/download/v1.1.0/WarpaiCompanion-windows-x64-setup.exe) | 使用远端 SSH 账号运行安装器。 |

安装包包含对应的运行组件、校验和、版本与源码清单及许可说明，无需手动解压或填写执行路径。
Unix 安装器会设置执行权限，Windows 使用当前账号安装方式。默认位置固定为：

- Linux/macOS：`~/.config/.warpai/bin/warpai-companion`
- Windows：`%USERPROFILE%\.config\.warpai\bin\warpai-companion.exe`

重复运行新版安装包可在原位置升级。桌面端和远端组件应来自同一个 release；绑定项目前会检查协议兼容性。

## 连接与排错

选中 SSH 终端，通过 `cd` 进入项目。面板根据已确认的远端用户目录、系统和工作目录寻找并检查组件。
组件缺失、无法执行或协议不兼容时，使用对应安装包安装或升级，再点击 **Reconnect**。
切换主机、账号、终端或项目时，旧响应不能污染当前项目；断开后的缓存状态不可写入。

companion 保留现有的按需私有服务启动方式。安装器不新增系统常驻服务，不修改 shell 启动文件或 PATH，
不复制凭据、不修改 SSH 配置，也不替换第三方 Agent 命令。远端 CLI Agent 仍通过自己的工具安装和认证。

## Windows 原生 SSH

登录到远端 PowerShell 提示符后，在 SSH 终端提示条选择
**Integrate PowerShell**，再用 `cd` 进入项目。这只启用当前会话的集成，
不会修改 PowerShell 配置文件。Windows 原生 OpenSSH 不支持连接复用；
companion 控制连接沿用原连接的目标与选项，由系统 SSH 密钥或密钥代理
认证。仅用密码登录时，第二条非交互连接可能无法认证；终端仍可使用，
面板会说明原因并提供重连操作。Warpai 不保存密码。

参考：[Microsoft Win32-OpenSSH 功能范围](https://github.com/PowerShell/Win32-OpenSSH/wiki/Project-Scope)。
