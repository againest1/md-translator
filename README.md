# Markdown Translator for Zed

将整篇英文 Markdown 翻译成简体中文，在 Zed 原生预览中阅读持久缓存译文。例如，打开项目的 `README.md`，运行翻译 Task，即可阅读 `README · 简体中文.md`；原文件保留，项目目录不会新增译文文件。

支持 macOS、Linux 和 Windows，通过 Zed Task 使用。下载安装即可，不需要 Rust、Node.js、Python 或额外运行库。支持本机项目和 SSH Remote Project。

## 开始使用

先安装 [Zed](https://zed.dev/download)，准备支持 OpenAI 兼容 Chat Completions 的服务地址、模型名称和 API Key。服务可能收费；翻译会发送整篇文档。

**macOS / Linux**：在本机终端粘贴运行，安装命令会自动选择适合的版本：

```sh
curl -fsSL https://github.com/againest1/md-translator/releases/latest/download/install.sh | sh
"$HOME/.local/bin/md-translator" install-zed --auto-preview
"$HOME/.local/bin/md-translator" configure
```

<details>
<summary>Windows：在 PowerShell 中安装</summary>

```powershell
Invoke-WebRequest -UseBasicParsing https://github.com/againest1/md-translator/releases/latest/download/install.ps1 -OutFile "$env:TEMP\md-translator-install.ps1"
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\md-translator-install.ps1"
& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe" install-zed --auto-preview
& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe" configure
```

适用于 Intel / AMD 64 位 Windows；不需要管理员权限。

</details>

在 Zed 中打开一份已保存的 Markdown，从 `task: spawn` 选择 **Markdown: 翻译全文**。首次翻译打开预览并显示进度；已有完整缓存时直接展示，不请求模型。

`--auto-preview` 会让 **所有 Markdown 文件**默认进入原生预览。省略时可在译文打开后手动点击 **Preview**。API Key 明文保存在独立凭据文件中，详见[配置与数据存储](USAGE.md#独立配置向导)。

也可从 [下载页面](https://github.com/againest1/md-translator/releases/latest) 获取程序包。手动安装、系统拦截及找不到命令的处理见[下载与安装](USAGE.md#下载与安装)；远端项目按 [SSH Remote Project](USAGE.md#ssh-remote-project) 在远端安装。

## 日常使用

| 操作 | 入口 |
| --- | --- |
| 翻译或打开缓存 | **Markdown: 翻译全文**；本机快捷键：macOS `Ctrl+Shift+T`，Linux / Windows `Ctrl+K` 后按 `Alt+T` |
| 强制重新翻译 | **Markdown: 重新翻译全文**；本机快捷键：macOS `Ctrl+Option+Shift+T`，Linux / Windows `Ctrl+K` 后按 `Alt+Shift+T` |
| 修改服务或测试连接 | **Markdown: 配置翻译服务** |
| 更新 / 卸载 | [更新与回退](USAGE.md#更新与回退) / [卸载](USAGE.md#卸载) |

远端使用名称含 **(Remote)** 的 Task，配置和缓存都在远端；本机快捷键不用于远端任务。

Task 会先保存当前文档。原文内容或模型配置改变后，按新内容翻译；未改变时复用缓存。失败时保留上次成功译文，重试通常无需强制重译。结果始终为整篇译文，不自动分片；超出模型限制时请调整模型或配置。

## 使用参考

- [日常使用与预览](USAGE.md#日常使用)
- [SSH Remote Project](USAGE.md#ssh-remote-project)
- [集中存储与路径](USAGE.md#集中存储与路径)
- [翻译和失败恢复](USAGE.md#翻译和失败恢复)
- [下载与安装](USAGE.md#下载与安装)
- [更新与回退](USAGE.md#更新与回退) / [卸载](USAGE.md#卸载)
