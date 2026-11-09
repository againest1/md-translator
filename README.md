# Markdown Translator for Zed

**在 Zed 里，把整篇英文 Markdown 翻译成简体中文。**

适合经常在 Zed 里阅读英文项目 README、安装指南和技术文档，希望直接看整篇中文的用户。打开项目的 `README.md`，运行 **Markdown: 翻译全文**，译文会在当前项目窗口中打开。

| 原文 `README.md` | 译文 `README · 简体中文.md`（示意） |
| --- | --- |
| **Check your changes** | **检查你的改动** |
| Run `git status` to inspect your changes. | 运行 `git status` 查看你的改动。 |

- **保留阅读格式**：按 Markdown 阅读标题、列表和表格，代码、公式、路径和链接目标按原文保留。实际译文措辞由所选模型决定。
- **原文和译文分开**：原文件不被译文覆盖，项目目录不会新增译文文件；阅读时可以返回原文或手动并排对照。
- **译文留存供下次阅读**：文档和模型配置未变时，重新打开直接使用已有译文，不再请求模型；重启 Zed 后也会保留。

目前按单个已保存的 `.md` / `.markdown` 文件整篇翻译，目标固定为简体中文。支持 macOS、Linux 和 Windows，也支持 Zed 的 SSH Remote Project；下载安装即可，无需编译。

[开始使用](#开始使用) · [日常使用](#日常使用) · [常见问题](#常见问题) · [完整指南](USAGE.md)

## 开始使用

先安装 [Zed](https://zed.dev/download)，再准备一个支持 **OpenAI 兼容 Chat Completions** 的云端服务或本地模型接口，以及它的接口地址、模型名称和 API Key（服务无需鉴权时可不填）。程序不附带模型或服务额度，也不会自动沿用 Zed 中配置的模型和密钥。

翻译会将整篇文档发送给你选择的服务，费用以该服务的计费规则为准；包含敏感内容时，请先确认服务适合处理这些数据。API Key 明文保存在程序运行机器的凭据文件中，详见[配置与数据存储](USAGE.md#独立配置向导)。

**1 · 安装程序和 Zed 入口**

以下命令在本机终端运行，会添加翻译 Task 和快捷键，并让 **所有 Markdown 文件**默认以预览打开。macOS / Linux 的入口也适用于这两种系统的 SSH Remote Project，远端准备见[远端项目指南](USAGE.md#ssh-remote-project)。

**macOS / Linux**：

```sh
curl -fsSL https://github.com/againest1/md-translator/releases/latest/download/install.sh | sh
"$HOME/.local/bin/md-translator" install-zed --auto-preview
```

<details>
<summary>Windows：在 PowerShell 中安装</summary>

```powershell
Invoke-WebRequest -UseBasicParsing https://github.com/againest1/md-translator/releases/latest/download/install.ps1 -OutFile "$env:TEMP\md-translator-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\md-translator-install.ps1"
& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe" install-zed --auto-preview
```

适用于 Intel / AMD 64 位 Windows；不需要管理员权限。

</details>

**2 · 配置翻译服务**

在 Zed 打开要阅读的项目，按 macOS `Cmd+Shift+P` 或 Linux / Windows `Ctrl+Shift+P` 打开[命令面板](https://zed.dev/docs/command-palette)，输入并运行 `task: spawn`，选择 **Markdown: 配置翻译服务**。

在任务终端中填写 **接口地址**和**模型名称**，两者可从所用服务的 API 文档获取。推理强度（effort）可保留默认的 `low`。选择“保存 API Key”后输入密钥；无需密钥的服务选择“无需鉴权”。请求超时可先保留默认值，最后选择“保存”。各项填写方法见[配置向导](USAGE.md#独立配置向导)。

**3 · 翻译第一份文档**

打开项目里一份已保存的英文 Markdown，再从 `task: spawn` 选择 **Markdown: 翻译全文**。Task 会先保存当前修改，打开 `文件名 · 简体中文.md`；等待“正在翻译全文”变为中文正文，即可阅读。

译文顶部的“返回原文”可以打开原文件。若看到的是 Markdown 源码，点击 **Preview** 切换到预览。

## 日常使用

| 操作 | 入口 |
| --- | --- |
| 翻译或打开已有译文 | **Markdown: 翻译全文** |
| 强制重新翻译 | **Markdown: 重新翻译全文** |
| 修改服务或测试连接 | **Markdown: 配置翻译服务** |
| 对照原文阅读 | 点击译文顶部的“返回原文”；需要并排时手动拖动标签页 |
| 取消进行中的翻译 | 在任务终端按 `Ctrl+C` |

翻译快捷键：macOS `Ctrl+Shift+T`；Linux / Windows `Ctrl+K` 后按 `Alt+T`。macOS / Linux 本机与 SSH Remote Project 共用入口。重译快捷键和默认预览的调整见[日常使用](USAGE.md#日常使用)。

关闭译文后，从原文再次运行 **Markdown: 翻译全文** 即可继续阅读。原文、服务或模型配置改变后会重新翻译；只想阅读已有译文时，无需强制重译。

译文默认保存在程序运行机器的 `~/.md-translator/cache/`，不在原文旁边。需要编辑或分享时，请将译文正文另存为自己的 Markdown 文件，避免后续翻译覆盖手动修改；详见[保存或分享译文](USAGE.md#保存或分享译文)。

**SSH Remote Project**：macOS / Linux 两端准备好程序和服务后，使用相同的 **Markdown: 翻译全文** 或快捷键，再在当前 Zed 任务终端按住 macOS **Cmd** 或 Linux **Ctrl** 点击译文链接。无需逐个远端项目安装 Task，配置与译文保存在远端。已有普通 Task 需在本机重新运行 `install-zed` 刷新；Windows 客户端的远端入口与完整步骤见[远端项目指南](USAGE.md#ssh-remote-project)。

### 常见问题

**很长的文档也能翻译吗？**

整篇原文和译文需要分别处于模型的输入、输出限制内。超限时请换用支持更长文档的模型，或按提示调整输出上限；程序不会自动拆分，也不会把截断的译文当作完成。具体处理见[翻译限制](USAGE.md#翻译和失败恢复)。

**图片、目录和特殊格式会怎样？**

链接显示文字、图片替代文字和普通引文会翻译；图片中的文字、文档开头的元数据（Frontmatter）和 HTML 块保留原样。标题翻译后，依赖英文标题的目录链接可能失效；远端预览中的部分图片也可能不显示，可返回原文查看。完整范围见[翻译范围与显示限制](USAGE.md#翻译范围与显示限制)。译文质量取决于所选模型，重要内容请对照原文确认。

**如何检查配置或处理翻译失败？**

先看任务终端的错误提示。地址、模型或密钥有问题时，运行 **Markdown: 配置翻译服务**，检查后选择“保存并测试”（会发送一次短文本请求，可能计费）。也可先做无需请求模型的[配置检查](USAGE.md#检查配置与连接)。

翻译失败或取消时保留上次成功译文，解决问题后通常重新运行 **Markdown: 翻译全文** 即可。各类错误的处理见[失败恢复](USAGE.md#失败后继续翻译)；仍无法解决时，按[问题反馈与诊断](USAGE.md#问题反馈与诊断)准备信息。

## 使用参考

- [下载、手动安装与安装问题](USAGE.md#下载与安装)
- [配置翻译服务与高级参数](USAGE.md#独立配置向导)
- [译文存储与迁移](USAGE.md#集中存储与路径)
- [更新与回退](USAGE.md#更新与回退) / [卸载](USAGE.md#卸载)
- [完整使用指南](USAGE.md) / [问题反馈与诊断](USAGE.md#问题反馈与诊断) / [MIT 许可证](LICENSE)
