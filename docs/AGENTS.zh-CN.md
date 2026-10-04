> 2026-10-05 更新：已撤回 Codex 同名命令替换、后台代理和用户配置写入。
> 普通 `codex` 命令使用原版安装；替代方案仅在显式协作会话中传入原生 MCP 参数，
> 本地启动入口与新版真实模型验收尚未完成。下文涉及旧 Codex 代理的验证记录属于历史证据。

# Agent 使用与兼容性

[English technical reference](../specs/agent-communication/COVERAGE.md) | **中文使用指南** · [返回产品介绍](../README.zh-CN.md)

Warpai 可以运行你独立安装的 CLI Agent。参与消息与任务协作还需要该 CLI 版本支持原生 MCP 客户端，并加载 Warpai 的本地桥接配置。

本页依据仓库的 [原生 MCP 兼容性记录（英文）](../specs/agent-communication/COVERAGE.md) 整理。
“具备接入条件”表示已有原生 MCP 文档或已安装 CLI 的帮助信息作为依据，不表示每个服务商、版本的真实模型协作都已通过验收。

## 开启协作

1. 先安装并认证所需 CLI Agent；使用包含协作功能的 [Warpai 试用构建](../README.zh-CN.md#get-warpai)。
2. 打开 **Settings > Features > Agent communication**。该功能默认关闭。
3. 勾选可用的 Agent，Warpai 会在后台配置捆绑的原生 stdio MCP 桥接器。
4. 在项目目录中新开终端和 Agent 会话；不支持配置热加载的 CLI 需要重启。
5. 打开 **Agent collaboration** 查看参与者、消息与任务。

无需手动输入注册提示或复制动态凭据。同一规范化项目中的参与 Agent 可以互相发现；仓库子目录共享范围，不同仓库和独立 worktree 默认隔离。

## 接入条件与识别命令

下表是仓库记录的接入条件，不是所有已认证模型的通过名单。组织策略、已安装版本或原生 MCP 加载失败仍可能阻止参与。

| Agent | 识别命令 | 接入条件 |
| --- | --- | --- |
| Claude Code | `claude` | 原生 MCP；已有已安装 CLI 帮助信息。 |
| Gemini | `gemini` | 原生 MCP；已有官方文档。 |
| Codex | `codex` | 原生 MCP；已有已安装 CLI 帮助信息。 |
| Amp | `amp` | 仅本地 CLI；云端 orbs 不在范围内。 |
| Droid | `droid` | 官方连接器支持。 |
| OpenCode | `opencode` | 官方原生 MCP 配置。 |
| Copilot | `copilot` | 官方 CLI MCP 配置。 |
| Pi | `pi` | 需使用具备原生 MCP 的版本；旧版本外部扩展不作为回退方案。 |
| oh-my-pi | `omp` | 维护者提供原生 MCP 配置。 |
| Auggie | `auggie` | 官方 CLI 集成。 |
| Cursor | `agent`、`cursor-agent` | CLI 原生 MCP 能力。 |
| Goose | `goose` | 原生扩展配置。 |
| Hermes | `hermes`、`hermes-agent` | 原生 MCP 集成。 |
| Mistral Vibe | `vibe`、`vibe-acp` | 原生 stdio 配置；启用后在目标目录中新开终端。 |
| Antigravity | `agy` | 已安装 CLI 帮助确认原生 stdio。 |
| Grok Build | `grok` | 官方 MCP 配置。 |
| DeepSeek Harness | `dsh-tui`；使用 TUI profile 的 `dsh` | profile 必须挂载官方 MCP 客户端。 |
| Qoder | `qoder`、`qodercli`、`qoder-cli` | 原生 MCP；与 QoderCN 独立发现和授权。 |
| QoderCN | `qodercn`、`qoderclicn` | 已安装 CLI 帮助确认原生 stdio；独立设置项。 |
| Trae | `trae`、`traecn`、`trae-cli`、`traecn-cli`、`traecli` | 需文档所述原生 MCP 版本；启动名称兼容性仍需逐版本确认。 |
| 自定义 CLI | 用户配置的识别规则 | 逐个确认原生 MCP 能力；识别到命令本身不足以证明可以协作。 |

各服务商的官方依据与适配器细节见 [完整技术记录（英文）](../specs/agent-communication/COVERAGE.md)。

## 配置和权限由谁管理

Warpai 仅管理自己拥有的 `warp-lite-communication` 配置，不接管其他 MCP 服务或用户设置。
格式错误、所有权冲突、用户修改过的受管项或符号链接，会使配置操作停止并显示原因。

取消勾选 Agent 或关闭功能，会立即撤销桥接服务中的权限。
配置清理失败仍保持无权限，并可通过 **Refresh agents / retry cleanup** 重试。
已有 CLI 的配置移除可能需要重载或重启才能体现在其界面中。

桥接器必须从受管理的 Warpai 终端继承当前会话绑定。在终端外启动的进程不能借此自动参与。
动态凭据不写入持久配置，也不需要用户复制。Vibe 使用内存中的原生环境配置，因此切换 profile 或项目后应新开终端。

## 查看状态与排队任务

- 忙碌的 Agent 可以积累待办；只有满足原生就绪条件后才提交下一条 inbox 指令。
- 不会覆盖输入草稿，也不会代答权限请求。
- 排队、投递、确认收到、提交结果和验收通过是不同状态。
- 无法确认生命周期或等待人工批准时，不应仅凭“桥接器已连接”判断任务能够自动执行。

## 已验证到哪一步

仓库在 2026-10-01 记录了 Codex、Claude Code、QoderCN、Cursor、Antigravity 和 DeepSeek Harness 的真实 CLI 握手与工具发现测试。
这些探测没有服务商模型调用，不等于完整的跨 Agent 任务执行或自动唤醒验收；Qoder 与 QoderCN 的结果不能混用。

后续工程验收覆盖本地任务状态机、原生进程、桌面操作和受控 OpenSSH 场景。
付费服务商会话、每种版本的模型行为和实体远程主机仍需按真实环境验证。
具体源码与 CI 证据见 [当前验收记录（英文）](../specs/agent-communication-v2/QUALITY.md)。

## SSH 项目

远程账号需要匹配版本的 companion 和在该账号中已安装、认证的 CLI。
桌面凭据不会复制到远端；远程参与权限固定在选定的账号和项目。
按照 [中文 SSH 入门步骤](../README.zh-CN.md#work-on-remote-projects) 配置和启动。
