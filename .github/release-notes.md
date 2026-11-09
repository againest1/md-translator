将整篇英文 Markdown 翻译成简体中文，在 Zed 中阅读；原文保留，译文自动缓存。

**v0.5.0 更新**：新增翻译推理强度（effort）设置，可在配置向导中选择 `none`、`low`、`medium`、`high`、`xhigh`、`max`，默认 `low`；也可运行 `md-translator configure --effort high` 修改，并用 `md-translator doctor` 查看。修改级别后，下次翻译会按新配置生成译文；各服务和模型支持的级别不同，请按所用模型的说明选择。

**不需要安装 Rust、Node.js、Python 或额外运行库。** 需要 Zed 和支持 OpenAI 兼容接口的翻译服务；服务可能收费，翻译时会收到整篇文档。

- **首次安装**：按 [开始使用](https://github.com/againest1/md-translator#开始使用) 选择自己的系统，复制安装命令，配置一次翻译服务即可。
- **更新**：再次运行安装命令，配置、密钥和译文保留；使用 SSH Remote Project 时，两端程序都需更新。旧配置未设置 effort 时改用 `low`；若服务不接受该参数，按[配置说明](https://github.com/againest1/md-translator/blob/main/USAGE.md#独立配置向导)将 `provider.reasoning_effort` 设为 `default`，由服务决定推理行为。
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

SSH Remote Project 需要在远端安装程序并配置服务，译文在当前 Zed 任务终端点击链接打开。Windows 客户端继续使用显式的 **Markdown (Remote)** 项目入口，步骤见 [远端使用](https://github.com/againest1/md-translator/blob/main/USAGE.md#ssh-remote-project)。
