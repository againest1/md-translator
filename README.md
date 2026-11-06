# Markdown Translator for Zed

**在 Zed 里，把整篇英文 Markdown 翻译成简体中文。**

打开项目的 `README.md`，运行 **Markdown: 翻译全文**，就能在 Zed 原生预览中阅读 `README · 简体中文.md`。

- **保留原文**：译文另行保存，项目目录不会新增译文文件。
- **保留代码和链接目标**：阅读中文说明时，代码、公式、路径和链接目标仍按原文保留。
- **译文会保留**：重启 Zed 后也能继续阅读。

支持 macOS、Linux 和 Windows，也支持 Zed 的 SSH Remote Project。下载即可使用，无需编译。

## 开始使用

先安装 [Zed](https://zed.dev/download)，准备支持 OpenAI 兼容 Chat Completions 的服务地址、模型名称和 API Key。整篇文档需在模型的输入和输出限制内。

**1 · 安装程序和 Zed 入口**

以下命令在本机终端运行，会添加翻译 Task，并让 **所有 Markdown 文件**默认以预览打开。远端项目请按 [SSH Remote Project](USAGE.md#ssh-remote-project) 在远端安装。

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

在 Zed 打开要阅读的项目，从命令面板运行 `task: spawn`，选择 **Markdown: 配置翻译服务**。按提示填写服务地址、模型名称和 API Key，最后选择“保存”。

翻译会将整篇文档发送给你配置的服务，可能产生费用。API Key 明文保存在程序运行机器的凭据文件中，详见[配置与数据存储](USAGE.md#独立配置向导)。

**3 · 翻译第一份文档**

在 Zed 项目中打开一份已保存的英文 Markdown，再从 `task: spawn` 选择 **Markdown: 翻译全文**。等待翻译完成，即可在 `文件名 · 简体中文.md` 预览中阅读中文。

## 日常使用

| 操作 | 本机入口 |
| --- | --- |
| 翻译或打开已有译文 | **Markdown: 翻译全文** |
| 强制重新翻译 | **Markdown: 重新翻译全文** |
| 修改服务或测试连接 | **Markdown: 配置翻译服务** |
| 使用快捷键或调整预览 | [日常使用](USAGE.md#日常使用) |

从原文触发翻译即可，Task 会先保存当前修改。原文或模型配置改变后会重新翻译；未改变时直接打开已有译文，不再请求模型。

## 使用参考

- [下载、手动安装与安装问题](USAGE.md#下载与安装)
- [配置翻译服务](USAGE.md#独立配置向导)
- [SSH Remote Project](USAGE.md#ssh-remote-project)
- [译文存储与迁移](USAGE.md#集中存储与路径)
- [翻译限制与失败恢复](USAGE.md#翻译和失败恢复)
- [更新与回退](USAGE.md#更新与回退) / [卸载](USAGE.md#卸载)
