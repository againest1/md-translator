# 使用指南

[返回 README](README.md)

## 独立配置向导

在命令面板的 `task: spawn` 中选择 **Markdown: 配置翻译服务**，终端会展开并获得焦点。也可在本机终端运行 `md-translator configure`。

向导依次填写接口地址、模型名称、鉴权方式、隐藏输入的 API Key、请求超时。鉴权只提供“保存 API Key”和“无需鉴权”。再次配置时可以编辑现有值；同一服务的密钥可留空保留。改变接口地址后，需要为新地址填写密钥。最后选择“保存”“保存并测试”或“取消”；退出向导不会提交未确认的配置。

默认只保存。“保存并测试”使用固定短文本发出一次模型请求；成功结果不写入文档翻译缓存。测试失败时已保存的配置保留，可修改后重试。

独立配置只使用一个服务、一个模型，不读取 Zed LLM provider、Edit Prediction 或系统钥匙串。模型必须支持 OpenAI 兼容的 Chat Completions。

```json
{
  "provider": {
    "name": "default",
    "base_url": "https://provider.example/v1",
    "model": "your-model",
    "auth": "api_key"
  },
  "request_timeout_seconds": 120
}
```

`auth` 可以是 `api_key` 或 `none`。密钥由向导保存到独立的 `credentials.json`，按服务标识与请求地址绑定，不写入命令行、日志或译文缓存。凭据文件是明文；macOS/Linux 上新建目录权限为 0700，配置和凭据文件权限为 0600。

可选高级配置包括 provider 下的 `reasoning_effort`、`max_completion_tokens` 或 `max_tokens`（输出上限只能选择一项）、`custom_headers`。未声明时不猜测厂商参数。`custom_headers` 不允许覆盖鉴权或 Content-Type。

工具只使用配置文件和显式命令行参数，不读取环境变量中的密钥或代理设置，也不支持配置中的环境变量插值。HTTP 请求直连配置地址；旧 `--api-key-env` 参数已移除。

修改非敏感配置可以使用参数；密钥必须在终端中隐藏输入：

```sh
./target/release/md-translator configure --base-url https://provider.example/v1 --model your-model --set-key
./target/release/md-translator configure --no-auth
./target/release/md-translator configure --timeout 180
./target/release/md-translator configure --test
```

## 日常使用

本机使用：先在 Zed 中打开原文所在的本机项目。远端操作见[SSH Remote Project](#ssh-remote-project)。从 `task: spawn` 选择 **Markdown: 翻译全文** 或 **Markdown: 重新翻译全文**；快捷键只是另一种触发入口。

| 操作 | macOS | Linux / Windows |
| --- | --- | --- |
| 翻译全文或打开缓存 | `Ctrl+Shift+T` | `Ctrl+K` 后按 `Alt+T` |
| 强制重新翻译 | `Ctrl+Option+Shift+T` | `Ctrl+K` 后按 `Alt+Shift+T` |

任务先保存当前文档，读取最新磁盘内容。未命名文档需要先保存为 `.md` 或 `.markdown`。固定按英译中处理；中文或中英混排文档也会发送给模型，不额外检测语言。

首次翻译打开 `文件名 · 简体中文.md`，显示“正在翻译全文”，成功后更新同一份预览。已有完整缓存时直接展示，不请求模型，也不读取密钥。译文顶部提供“返回原文”链接；需要并排阅读时可手动拖动标签页。

本地译文在原文所在的项目窗口中以标签页打开，不新增 Project Panel 条目。旧版本已经加入 Project Panel 的缓存条目，请右键选择 **Remove from Project** 后再从原文打开译文；此操作保留磁盘上的缓存文件。

在缓存预览中重译时，先返回项目内原文。Zed 的任务文件变量依赖可见项目目录。

提示尚未配置服务时，先运行配置向导，再重新触发翻译。

## 集中存储与路径

配置、密钥和缓存保存在程序运行的机器上。Remote Project 使用远端账户的目录；本机项目使用本机账户的目录。默认使用系统账户的用户目录，在其下创建统一的 `.md-translator`：

```text
~/.md-translator/
├── config.json
├── credentials.json
└── cache/
    ├── translations/（完整成功结果）
    └── views/<document-and-provider-key>/
        └── 文件名 · 简体中文.md
```

更换服务、模型或请求选项后会重新翻译；只轮换同一服务的 API Key 仍可使用已有缓存。强制重译中断时保留上次成功结果。重启不会清空缓存，当前不自动过期或清理。迁移存储位置时，请复制完整数据目录，保留配置、凭据和缓存的关联文件。

每次触发都会读取当前原文内容计算缓存标识。原文改变后不会误用旧内容的缓存；当前内容没有对应结果时，重新翻译全文。只保存而内容未变，或将内容改回曾经翻译过的版本，仍可复用该版本的缓存。通过 Zed Task 触发时会先保存当前修改。

原文内容改变后仍更新同一预览文件，无需手动更换标签页。不同机器的缓存各自独立。本机预览中的相对图片和文件链接按原文所在目录查找；远端资源限制见[SSH Remote Project](#ssh-remote-project)。同一文档正在翻译时，再次触发不会重复请求。

默认目录不受 `HOME` / `XDG_*` 覆盖（Unix 上通过系统账户记录确定用户目录）。整体迁移位置使用显式参数：

```sh
./target/release/md-translator --data-dir /absolute/path/md-translator configure
./target/release/md-translator --data-dir /absolute/path/md-translator install-zed
```

安装器会把显式存储参数写进管理的 Task，确保向导与翻译使用相同位置。也支持单独的 `--config`、`--cache-dir`；凭据始终位于配置文件同目录。

`--zed-config` 只指定安装 Task / 快捷键和检查预览设置所用的 Zed 配置目录，不用于读取模型或密钥。Zed CLI 默认从常见安装路径查找；非标准安装可在独立配置中设置 `zed_cli` 的绝对路径，或传入 `--zed-cli /absolute/path/to/zed`。

从本机终端翻译时，可在 `translate` 命令中添加 `--project-root /absolute/path/to/project`，指定已在 Zed 打开的本机项目，普通文件夹项目也适用。Zed 翻译 Task 会自动传入所属项目；本地文件未指定时，按原文所在目录定位窗口。

0.2 及之前版本的 Zed provider、环境变量鉴权和偏好格式不再使用，需要运行配置向导。旧缓存目录不自动删除；已有缓存仍可通过显式 `--cache-dir` 使用；只有原文和完整 provider 指纹一致时才命中。

## SSH Remote Project

两端安装 `md-translator`，并分别配置 provider。远端 Task 直接使用远端文件、远端 provider 和远端缓存；远端必须能访问配置的服务地址。完整文档会发送给该服务。本机 provider 不参与远端翻译，远端是否安装 Zed CLI 不影响此流程。

### 安装远端 Task

在 Zed 的 **Remote Project 终端**中运行，替换为远端程序与项目的实际路径：

```sh
/absolute/path/to/md-translator configure
/absolute/path/to/md-translator install-zed --remote --project-root /home/user/project
```

已有 provider 配置时可省略第一条。安装器将三个 **Markdown (Remote)** Task 写入远端项目的 `.zed/tasks.json`，保留其它 Task 和注释，修改已有文件前创建备份。可先添加 `--dry-run` 查看目标文件；再次安装不会重复添加任务。程序移动或更新后重新安装。

远端安装只管理项目 Task。本机的快捷键和 Markdown 预览设置仍在本机 Zed 中设置；`--remote` 不与 `--auto-preview`、`--zed-config` 一起使用。本机安装的翻译快捷键不能用于这些远端 Task，以免触发本机程序路径；需要快捷键时，在本机 Zed 为下表中的远端 Task 名称另设绑定。

| 操作 | 在 `task: spawn` 中选择 |
| --- | --- |
| 翻译全文或使用缓存 | **Markdown (Remote): 翻译全文** |
| 强制重新请求模型 | **Markdown (Remote): 重新翻译全文** |
| 修改远端 provider | **Markdown (Remote): 配置翻译服务** |

### 点击打开译文

1. 在当前 Remote Project 中打开原文，再运行远端翻译 Task。Task 会先保存当前文档。
2. 任务终端出现 `file://...` 译文链接。macOS 按住 **Cmd** 点击；Linux / Windows 按住 **Ctrl** 点击。请在当前 Zed 任务终端点击，让它按所属远端项目打开文件。
3. 译文以标签页打开；尚未完成时显示进度，完成后更新同一文件。未启用默认预览时，手动点击 **Preview**。

任务结束后终端保留，缓存命中也会输出链接。链接中的中文、空格等会编码；直接点击完整链接即可。如果点击未生效，复制终端最后输出的原始绝对路径，在当前 Remote Project 中用打开文件操作选择该文件。

缓存无需加入 Project Panel；从缓存预览重译时，先返回原文。返回原文和普通文件链接按远端路径打开。项目外缓存中的图片仍受 Zed 远端预览的路径解析限制，可能不显示，请返回原文查看；程序不下载或复制图片。

也可在远端终端直接运行：

```sh
md-translator translate --remote --file /home/user/project/README.md
md-translator translate --remote --file /home/user/project/README.md --force
```

`--remote` 不启动 Zed，即使远端已安装 Zed CLI 也一样。它输出适合 Zed 终端点击的链接；普通 SSH 终端不保证按同一个 Remote Project 打开。

### 存储位置与旧版本

默认使用远端账户的 `~/.md-translator/`。需要其它位置时，在远端配置和安装时传入相同参数：

```sh
md-translator --data-dir /absolute/remote/data configure
md-translator --data-dir /absolute/remote/data install-zed --remote --project-root /home/user/project
```

`--data-dir`、`--config`、`--cache-dir` 都指向程序运行所在机器；安装器会把显式参数写进 Task。本机和远端不会同步配置、密钥或缓存。

旧版 `host:path`、`ssh://...` 文件输入和 `--remote-cache-dir` 已停用。请在远端从原文重新翻译；旧的本机 SSH 缓存和远端缓存不会自动迁移或删除。旧配置中的 `ssh` 字段会被忽略，重新保存配置时移除，其 provider 保留。含冒号的普通文件名可用 `./文件名.md` 或绝对路径传入。

## 翻译和失败恢复

每份文档使用一次整篇主请求。模型直接返回 Markdown；代码、公式、路径和链接目标通过保护标记保留，链接显示文字、图片替代文字和普通引文参与翻译。

译文完整性检查失败时，最多发出 **一次整篇纠正请求**，因此一次翻译最多产生两次模型请求。服务错误、额度不足、输出截断等直接报告，不自动循环重试、分片或切换模型。在任务终端中按 `Ctrl+C` 可取消。

成功译文才进入缓存。强制重译失败时保留上次完整结果；退出进行中的翻译不保存部分结果。超出模型上下文或输出上限时，可以调整模型或输出限制；不会截掉原文或把部分译文标为完成。

Frontmatter 和原始 HTML 按原文保留。完整性检查不等于语义质量保证；标题翻译后，原文中依赖英文标题的 `#fragment` 目录链接目前可能失效。

```sh
./target/release/md-translator translate --file examples/preview-demo.md
./target/release/md-translator translate --file examples/preview-demo.md --force
./target/release/md-translator translate --file examples/preview-demo.md --no-open
```

## 构建与更新

从源码构建需要支持 Rust 2024 edition 的稳定工具链：

```sh
cargo build --release
```

将对应操作系统与架构的程序安装在需要运行 Task 的机器上，更新后重新执行该机器对应的安装命令。本机用 `install-zed`，远端项目用 `install-zed --remote --project-root /远端项目目录`。安装记录绝对路径，复制本机任务配置到远端不能替代远端安装。
