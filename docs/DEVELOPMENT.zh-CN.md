# Warpai 开发指南

[English](DEVELOPMENT.md) | **简体中文** · [返回产品介绍](../README.zh-CN.md)

修改仓库前，请阅读 [AGENTS.md（英文）](../AGENTS.md) 和 [MEMORY.md（英文）](../MEMORY.md)。
面向用户的功能介绍与下载入口位于 [中文 README](../README.zh-CN.md)。

## 项目范围

- 桌面应用：macOS 与 Windows x64。
- SSH 项目环境及 companion：Linux、macOS 与 Windows。
- 保留终端渲染、输入、Shell 集成和现有编辑器。
- 相关修改须保持默认应用与 `warp_platform` 检查通过。
- 不恢复捆绑云端 AI、账号登录、计费或遥测产品入口。
- 直接维护源码，不在构建中加入上游同步、补丁重放或恢复步骤。

终端核心保护清单保留在 [README 贡献者部分](../README.zh-CN.md#develop-and-contribute)。

## 工具链与构建

使用 [rust-toolchain.toml](../rust-toolchain.toml) 固定的 Rust 工具链，
以及 [Cargo.lock](../Cargo.lock) 固定的依赖。构建前获取 Git LFS 资源。
macOS 构建需要完整 Xcode 和 Metal 工具链。

当前维护者工作空间仅在 GitHub 上运行 Rust 编译、测试和打包。
本地文档、脚本语法、格式与静态检查无需 Rust 构建。
以下命令说明已配置构建主机上的步骤：

```sh
cargo check -p warpai --bin warpai --locked
cargo check -p warpai --bin warpai --features warp_platform --locked
cargo build --release -p warpai --bin warpai --locked
cargo build --release -p warp-agent-bus --bin warpai-agent --locked
```

macOS 打包复用 [build-warpai-app.sh](../script/build-warpai-app.sh)。
Windows 打包复用 [build-warpai-windows.ps1](../script/build-warpai-windows.ps1)，
安装器资源说明见 [Windows 安装器文档（英文）](../script/windows/README.md)。
修改可见品牌名称时，保留原有 bundle 与数据存储标识，避免破坏兼容性。

macOS 分发构建使用 `--features release_bundle,extern_plist`，与仓库现有
macOS bundle 流程一致：保留发布配置的输入法组合文本支持，应用版本与标识由
实际安装包的 Info.plist 提供。

## 验证

- [桌面通信工作流](../.github/workflows/validate-agent-communication.yml)：macOS、Windows 应用检查、聚焦回归，以及可选原生 UI 截图与调试构建。`protocol_only` 跳过桌面检查，除非启用 `check_app` 或 `capture_ui`；`capture_ui` 构建隔离的调试应用。验证流程不生成桌面安装包。
- [远程 companion 工作流](../.github/workflows/validate-remote-companion.yml)：三平台协议和进程测试、受控 Linux OpenSSH 验收，以及带源码和摘要清单的 companion 产物。
- [当前验收记录（英文）](../specs/agent-communication-v2/QUALITY.md)：替换决策、清理范围和准确源码版本的验证证据。

根据修改的逻辑选择检查。Shell 脚本按 shebang 使用 `bash -n` 或 `zsh -n`。
Rust 格式检查应聚焦相关文件：全仓库格式化可能遇到指向已禁用上游模块的保留引用。
不要为修复小问题而重排无关源码。

## 发布与历史

[macOS 发布工作流](../.github/workflows/release-macos.yml) 构建已有仓库 tag，打包应用与三平台 companion，再创建发布草稿。
[Windows 工作流](../.github/workflows/release-windows-x64.yml) 使用同一 tag，验证并附加安装器与便携包后公开发布，也支持显式手动触发。
macOS 打包设置 `GIT_RELEASE_TAG=v1.0.1`，Windows 构建传入 `-ReleaseTag v1.0.1`；
应用包版本由同一个 tag 得出。流程与检查见 [发布规范（英文）](../specs/RELEASE.md)。
验证产物是调试构建和原生截图，与带 tag 的正式发布分开。
安装器、应用 ZIP 与 DMG 由上述正式发布流程生成。

默认分支是 `main`。保留版权、许可证与历史出处；不要删除无关的未合并工作或活动 PR 分支。

历史终端集成审计保留在 [WARP_LITE_SYNC_2026-08.md（英文）](../WARP_LITE_SYNC_2026-08.md)
和 Git 历史中，它们是工程记录，不是当前产品功能清单。
