将整篇英文 Markdown 翻译成简体中文，在 Zed 中阅读；原文保留，译文自动缓存。

**v0.6.0 更新**：移除存在路径解释风险的旧远端项目 Task 安装入口，统一使用普通 Task 或远端终端命令；修复安装 Zed 快捷键时改变同组用户快捷键作用域的问题。配置向导和 `configure --effort` 现在支持 `default`，重新配置时会保留该选项；修复英文缩写后带引号的路径未被正确保护、可能参与翻译的问题。

**不兼容变更**：`install-zed` 和 `uninstall-zed` 不再接受 `--remote`、`--project-root`，也不再管理 **Markdown (Remote)** 项目 Task；`translate --remote` 和 `translate --auto-remote` 继续可用。曾安装旧入口时，请从对应项目的 `.zed/tasks.json` 中移除名称以 **Markdown (Remote)** 开头的三个 Task，并移除本机指向这些 Task 的自定义快捷键，再按[远端使用](https://github.com/againest1/md-translator/blob/main/USAGE.md#ssh-remote-project)改用普通 Task 或终端命令。

**不需要安装 Rust、Node.js、Python 或额外运行库。** 需要 Zed 和支持 OpenAI 兼容接口的翻译服务；服务可能收费，翻译时会收到整篇文档。

- **首次安装**：按 [开始使用](https://github.com/againest1/md-translator#开始使用) 选择自己的系统，复制安装命令，配置一次翻译服务即可。
- **更新**：再次运行安装命令，配置、密钥和译文保留；然后在本机运行 `md-translator install-zed` 刷新入口，曾指定存储路径或 Zed CLI 时带上原参数，再从 `task: spawn` 重新选择任务。使用 SSH Remote Project 时，两端程序都需更新。若服务不接受推理强度参数，可在配置向导中选择 `default` 或运行 `md-translator configure --effort default`；它会省略该参数，与关闭推理的 `none` 不同，详见[配置说明](https://github.com/againest1/md-translator/blob/main/USAGE.md#独立配置向导)。
- **卸载**：见 [卸载](https://github.com/againest1/md-translator/blob/main/USAGE.md#卸载)。

手动下载时选择下面对应的程序包，**不要选择 GitHub 自动生成的 Source code**。

| 系统 | 下载文件 |
| --- | --- |
| Mac，Apple 芯片（M 系列） | `md-translator-aarch64-apple-darwin.tar.gz` |
| Mac，Intel 芯片 | `md-translator-x86_64-apple-darwin.tar.gz` |
| Linux，Intel / AMD 64 位 | `md-translator-x86_64-unknown-linux-musl.tar.gz` |
| Linux，ARM64 | `md-translator-aarch64-unknown-linux-musl.tar.gz` |
| Windows，Intel / AMD 64 位 | `md-translator-x86_64-pc-windows-msvc.zip` |

安装命令自动选择文件并校验下载。手动安装、系统拦截和常见问题见 [下载与安装](https://github.com/againest1/md-translator/blob/main/USAGE.md#下载与安装)。

SSH Remote Project 需要在远端安装程序并配置服务。macOS / Linux 客户端使用普通 Task 和快捷键；Windows 客户端连接 macOS / Linux 远端时，在 Zed 的 Remote Project 终端运行 `md-translator translate --remote --file "/远端/原文.md"`，将示例换成实际路径。译文在当前 Zed 终端点击链接打开，步骤见 [远端使用](https://github.com/againest1/md-translator/blob/main/USAGE.md#ssh-remote-project)。
