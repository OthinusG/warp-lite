# README 截图来源

[English](README.md) | **简体中文** · [返回产品介绍](../../README.zh-CN.md)

这些图片是原生 macOS 应用输出的原始 PNG 截图，未经修改。
截图中的项目、Agent 和任务是原生验收驱动使用的样例，不是用户项目或经过认证的服务商模型会话。

- 标签页：启用 macOS 原生垂直标签页面板。
- 主题：默认的 **Claude Warm Light**，由原生应用直接渲染。
- 源码版本：`b41d07ba2896e8f3c755c9227ac3483d873c4e2b`。
- [GitHub 桌面验证记录 37259523352](https://github.com/OthinusG/warpai/actions/runs/37259523352)。
- 产物名称：`Warpai-collaboration-native-captures-macOS`。
- 截图目录：`capture-1194/collaboration-static/2026-10-05T03-55-06`。

| 文档图片 | 原始截图 | 内容 |
| --- | --- | --- |
| [warpai-workspace.png](warpai-workspace.png) | `live-evidence-file-open.png` | 源码编辑器与实时任务详情面板。 |
| [warpai-ssh-project.png](warpai-ssh-project.png) | `live-ssh-task.png` | 实时 SSH 面板中的远程项目、Agent 和任务。 |

## 视觉检查

已对照[原生 UI 规范（英文）](../../specs/agent-communication-v2/UI-CONSISTENCY.md)检查原始截图。
早先的 `5883221` 截图因工具栏图标过淡而未采用；修正后的源码使用完整红色通道作为透明度遮罩，显示颜色仍由原生主题控制。
下载的 macOS 截图包通过 GitHub SHA-256 和 ZIP CRC 校验，161 张 PNG 均可解码；
诊断中的源码版本匹配、退出码为 0，原生操作断言没有失败。

| 检查项 | 预期与检查结果 |
| --- | --- |
| 配色 | 工作区和 SSH 图片由原生 Claude Warm Light 主题渲染，没有后期改色。 |
| 布局 | macOS 垂直标签页可见，旁边为源码或终端及任务面板；长内容使用原生滚动区域。 |
| 工具栏素材 | Settings 终端符号与四行 Tools 图标清晰，素材外围圆圈、方框已移除；已检查浅色、深色和两种标签布局。 |
| 字体层级 | 主面板半粗体标题与分区标题、次要提示保持区分。 |
| 按钮与间距 | SSH 使用 Secondary 按钮，旁边为刷新；320px 面板在 125% 缩放下导航正常换行，主要控件没有重叠。 |
| 设置 | Agents 分区、原生设置行对齐、说明文字可读；已检查 Claude Warm 的 100%/125% 以及 Light/Dark 的 125%。 |

两张文档 PNG 与原始截图逐字节相同，尺寸均为 1440 × 684。
用于截图的调试构建不是已发布安装包。

准备文档时，本工作副本中没有找到独立设计的 Warpai App 图标。
README 使用文字标题，未另造替代品牌标志。
