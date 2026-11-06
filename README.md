# Markdown Translator for Zed

将整篇英文 Markdown 翻译成简体中文，在 Zed 原生预览中阅读持久缓存译文。例如，打开项目的 `README.md`，运行翻译 Task，即可阅读 `README · 简体中文.md`；原文件保留，项目目录不会新增译文文件。

程序通过 Zed Task 接入。运行编译好的程序不需要 Node.js、Python 或 Rust 开发环境。支持本机项目和 SSH Remote Project；远端 Task 使用远端安装的程序、provider 和缓存。

## 开始使用

从源码构建需要 Rust 稳定版。在本机终端运行：

```sh
cargo build --release
./target/release/md-translator install-zed --auto-preview
./target/release/md-translator configure
```

在 Zed 中打开一份已保存的 Markdown，从 `task: spawn` 选择 **Markdown: 翻译全文**。首次翻译打开预览并显示进度；已有完整缓存时直接展示，不请求模型。

`--auto-preview` 会让 **所有 Markdown 文件**默认进入原生预览。省略时可在译文打开后手动点击 **Preview**。安装器会备份已有配置；移动或更新程序后重新运行安装命令。

服务须支持 OpenAI 兼容的 Chat Completions。翻译会发送整篇文档；API Key 明文保存在程序独立的凭据文件中，详见[配置与数据存储](USAGE.md#独立配置向导)。

SSH Remote Project 请在**远端终端**安装项目 Task：

```sh
md-translator install-zed --remote --project-root /home/user/project
```

远端程序和 provider 配好后，从 `task: spawn` 选择 **Markdown (Remote): 翻译全文**，再在任务终端 **Cmd+Click**（macOS）或 **Ctrl+Click**（Linux / Windows）译文链接。缓存留在远端。完整步骤见[SSH Remote Project](USAGE.md#ssh-remote-project)。

## 日常使用

| 操作 | 入口 |
| --- | --- |
| 翻译或打开缓存 | **Markdown: 翻译全文**；本机快捷键：macOS `Ctrl+Shift+T`，Linux / Windows `Ctrl+K` 后按 `Alt+T` |
| 强制重新翻译 | **Markdown: 重新翻译全文**；本机快捷键：macOS `Ctrl+Option+Shift+T`，Linux / Windows `Ctrl+K` 后按 `Alt+Shift+T` |
| 修改服务或测试连接 | **Markdown: 配置翻译服务**，或运行 `md-translator configure` |
| 检查配置和密钥 | `md-translator doctor --check-key` |

远端使用名称含 **(Remote)** 的 Task，配置和缓存都在远端；本机快捷键不用于远端任务。

Task 会先保存当前文档。原文内容或模型配置改变后，按新内容翻译；未改变时复用缓存。失败时保留上次成功译文，重试通常无需强制重译。结果始终为整篇译文，不自动分片；超出模型限制时请调整模型或配置。

## 使用参考

- [日常使用与预览](USAGE.md#日常使用)
- [SSH Remote Project](USAGE.md#ssh-remote-project)
- [集中存储与路径](USAGE.md#集中存储与路径)
- [翻译和失败恢复](USAGE.md#翻译和失败恢复)
- [构建与更新](USAGE.md#构建与更新)
