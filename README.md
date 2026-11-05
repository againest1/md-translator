# Markdown Translator for Zed

将整篇英文 Markdown 翻译成简体中文，在 Zed 原生预览中阅读持久缓存译文。原文件保留，项目目录不会新增译文文件，重新打开文档或重启 Zed 后可以复用完整缓存。

程序用 Rust 编写，通过 Zed Task 接入，不是 WASM 扩展。运行编译好的程序不需要 Node.js、Python 或 Rust 开发环境。支持本地文件以及 SSH 远端文件地址；远端模式使用本机的系统 SSH 客户端和服务器已有的 SFTP 功能，不在远端安装翻译程序。

## 开始使用

从源码构建需要 Rust 稳定版：

```sh
cargo build --release
./target/release/md-translator install-zed --auto-preview
./target/release/md-translator configure
./target/release/md-translator doctor --check-key
```

安装器会备份已有 Zed 配置，保留其它设置和 JSONC 注释，安装三个 Task 与可选使用的翻译快捷键。安装记录程序的绝对路径；移动程序或更新版本后重新运行 `install-zed`。

`--auto-preview` 会让 **所有 Markdown 文件**默认进入 Zed 原生预览。省略此选项时，译文打开后可手动点击 Preview；省略不会关闭已经开启的设置。

已安装的 Task / 快捷键适用于本机项目。SSH 文件请从本机运行下面的远端地址命令；Remote Project 的原有 Task 仍在远端执行，不能借此启动 Mac 上的程序。当前没有添加后台助手、反向转发或远端部署。Dev Container 连接方式尚未适配。

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

工具不提供环境变量配置入口，不插值配置中的环境变量，也不读取 provider 密钥环境变量、`MD_TRANSLATOR_*`、`ZED_CLI`、`XDG_*` 或 HTTP 代理环境变量。HTTP 请求直连配置地址。仅使用配置文件和显式命令行参数；旧 `--api-key-env` 参数已移除。

修改非敏感配置可以使用参数；密钥必须在终端中隐藏输入：

```sh
./target/release/md-translator configure --base-url https://provider.example/v1 --model your-model --set-key
./target/release/md-translator configure --no-auth
./target/release/md-translator configure --timeout 180
./target/release/md-translator configure --test
```

## 日常使用

先在 Zed 中打开原文所在的本机项目。从 `task: spawn` 选择 **Markdown: 翻译全文** 或 **Markdown: 重新翻译全文**；快捷键只是另一种触发入口。

| 操作 | macOS | Linux / Windows |
| --- | --- | --- |
| 翻译全文或打开缓存 | `Ctrl+Shift+T` | `Ctrl+K` 后按 `Alt+T` |
| 强制重新翻译 | `Ctrl+Option+Shift+T` | `Ctrl+K` 后按 `Alt+Shift+T` |

任务先保存当前文档，读取最新磁盘内容。未命名文档需要先保存为 `.md` 或 `.markdown`。固定按英译中处理；中文或中英混排文档也会发送给模型，不额外检测语言。

首次翻译打开 `文件名 · 简体中文.md`，显示“正在翻译全文”，成功后更新同一份预览。已有完整缓存时直接展示，不请求模型，也不读取密钥。译文顶部提供“返回原文”链接；需要并排阅读时可手动拖动标签页。

在缓存预览中重译时，先返回项目内原文。Zed 的任务文件变量依赖可见项目目录。译文在原文所在的项目窗口中打开，缓存文件可能作为额外条目显示在该窗口的 Project Panel 中；不会自动安排分屏。旧版本已挂入其它窗口的缓存条目，可从该窗口的项目中移除后再从原文触发翻译。

没有独立配置时，翻译 Task 会提示使用配置向导，不会在后台等待输入，也不会回退到 Zed provider。

## 集中存储与路径

本地文件默认使用系统账户的用户目录，在其下创建统一的 `.md-translator`：

```text
~/.md-translator/
├── config.json
├── credentials.json
└── cache/
    ├── translations/<translation-key>/
    │   ├── manifest.json
    │   └── result-<digest>.md
    └── views/<document-and-provider-key>/
        ├── 文件名 · 简体中文.md
        ├── source.json
        └── request.lock
```

翻译结果标识包含原文内容、provider 标识与地址、模型、实际请求选项、提示词和保护规则版本。API Key 不参与标识。成功结果完整写入后再提交 manifest；强制重译中断不会损坏上次成功结果。重启不会清空缓存，当前不自动过期或清理。

每次触发都会读取当前原文内容计算缓存标识。原文改变后不会误用旧内容的缓存；当前内容没有对应结果时，重新翻译全文。只保存而内容未变，或将内容改回曾经翻译过的版本，仍可复用该版本的缓存。通过 Zed Task 触发时会先保存当前修改。

预览路径由原文件路径与 provider 配置决定，原文内容改变后仍更新同一预览文件。缓存正文保留原始相对资源地址；预览副本根据原文件目录解析图片和文件链接。锁文件避免同一文档重复请求，进程退出后锁自动释放。

默认目录不受 `HOME` / `XDG_*` 覆盖（Unix 上通过系统账户记录确定用户目录）。整体迁移位置使用显式参数：

```sh
./target/release/md-translator --data-dir /absolute/path/md-translator configure
./target/release/md-translator --data-dir /absolute/path/md-translator install-zed
```

安装器会把显式存储参数写进管理的 Task，确保向导与翻译使用相同位置。也支持单独的 `--config`、`--cache-dir`；凭据始终位于配置文件同目录。

`--zed-config` 只指定安装 Task / 快捷键和检查预览设置所用的 Zed 配置目录，不用于读取模型或密钥。Zed CLI 默认从常见安装路径查找；非标准安装可在独立配置中设置 `zed_cli` 的绝对路径，或传入 `--zed-cli /absolute/path/to/zed`。

从本机终端翻译时，可在 `translate` 命令中添加 `--project-root /absolute/path/to/project`，指定已在 Zed 打开的项目，普通文件夹项目也适用。Zed 翻译 Task 会自动传入所属项目；普通命令行未指定时，按原文所在目录定位窗口。

0.2 及之前版本的 Zed provider、环境变量鉴权和偏好格式不再使用，需要运行配置向导。旧缓存目录不自动删除；已有缓存仍可通过显式 `--cache-dir` 使用；只有原文和完整 provider 指纹一致时才命中。

## SSH 远端文件

在 **Mac / 本机终端** 运行 `md-translator`，传入像 `scp` 一样的远端地址。先在 Zed 保存远端文档，程序读取的是服务器上已保存的内容。下面示例中的连接名和文件路径需替换为实际值：

```sh
./target/release/md-translator translate --file 'dev-server:/home/user/project/README.md'
./target/release/md-translator translate --file 'dev-server:docs/README.md'
./target/release/md-translator translate --file 'ssh://user@server:2222/home/user/project/README.md'
```

支持 `[user@]host:path` 和 `ssh://[user@]host[:port]/path`，相对路径按远端登录目录解析。SCP 形式的中文、空格、百分号、`#` 和 `?` 可原样放在引号里；URL 形式的特殊字符需要 URL 编码。含冒号的本地文件名可以用 `./文件名` 或本地绝对路径消除歧义。路径通过 SFTP 协议传输，不拼进远端 shell 命令。

处理顺序是：SSH 读取原文 → 读取远端完整缓存 → 未命中才使用本机 provider 翻译 → 原子写回远端缓存 → 调用本机 `zed --add ssh://…` 打开远端预览。一次运行复用一个 SFTP 会话。API Key、provider 配置留在本机。

正式缓存默认放在远端 `~/.md-translator/cache/`，沿用 `translations/<缓存标识>/manifest.json`、`result-*.md` 和 `views/` 预览，不生成重复的 `translation.md`。SSH 预览正文中的图片、文件链接保持原样，不下载图片或转换路径；头部的“返回原文”链接使用 Zed 的 SSH 链接。

```sh
# 强制重译
./target/release/md-translator translate --file 'dev-server:/home/user/project/README.md' --force
# 只生成远端缓存并输出 SSH 地址，不打开 Zed
./target/release/md-translator translate --file 'dev-server:/home/user/project/README.md' --no-open
# 指定远端缓存位置；引号保留远端用户的 ~
./target/release/md-translator translate --file 'dev-server:/home/user/project/README.md' --remote-cache-dir '~/.md-translator/cache'
```

`--cache-dir` 专用于本地文件；远端输入必须用 `--remote-cache-dir`。`--data-dir` 和 `--config` 始终指定本机的配置/状态目录。

SSH 连接沿用系统 OpenSSH 的连接别名、登录密钥、跳板机和主机验证配置。工具使用 `BatchMode`；如果首次登录仍需要确认主机或输入密码，请先在本机用 `ssh 连接名` 完成登录配置。服务器需要启用 SFTP v3 和 `posix-rename` 原子替换扩展（常规 OpenSSH 服务提供这些能力）。当前已验证 macOS 客户端连接 Linux 服务器。

可在本机 `config.json` 添加可选的 `ssh` 配置；没有设置时使用系统 SSH 和上述默认值：

```json
{
  "ssh": {
    "binary": "/usr/bin/ssh",
    "config_file": "/absolute/path/to/ssh_config",
    "identity_file": "/absolute/path/to/private_key",
    "cache_dir": "~/.md-translator/cache",
    "connect_timeout_seconds": 15,
    "operation_timeout_seconds": 30
  }
}
```

以上是配置片段，合并到已有配置中；不需要的字段可以省略。向导再次保存 provider 时会保留这些设置。所有工具设置仍使用配置文件和显式参数，不新增环境变量开关。若连接参数只写在自定义 SSH 配置文件里，也需要让 Zed 能使用相同的连接设置，才能自动打开远端文件。

如果模型已经返回完整译文，但远端上传或预览文件更新失败，完成的结果会暂存到本机 `.md-translator/remote-state/pending/`。再次运行相同地址、内容和模型配置时优先继续上传（即使带了 `--force`），不重复请求模型；发布成功后删除暂存文件。打开 Zed 失败不会破坏已提交的缓存，普通重试会复用它。

`.md-translator/remote-state/locks/` 的锁用于避免同一台本机对同一远端地址重复请求；不同电脑同时翻译不做分布式任务去重。远端结果和预览使用临时文件加原子替换，manifest 最后指向完整结果。

Zed 按 SSH 连接和工作区匹配远端文件；同一服务器打开多个项目时，具体窗口选择仍由 Zed CLI 决定。缓存目录位于项目外，可能作为额外文件显示在工作区中。程序不会自动安排分屏。

## 翻译和失败恢复

每份文档使用一次整篇主请求。模型直接返回 Markdown；代码、公式、路径和链接目标通过保护标记保留，链接显示文字、图片替代文字和普通引文参与翻译。

保护标记校验失败时，会带具体错误进行 **一次整篇纠正**。正常路径一次请求，纠正路径最多两次。服务错误、额度不足、输出截断等直接报告，不自动循环重试、分片或切换模型。在任务终端中按 `Ctrl+C` 可取消。

成功译文才进入缓存。强制重译失败时保留上次完整结果；退出进行中的翻译不保存部分结果。超出模型上下文或输出上限时，可以调整模型或输出限制；不会截掉原文或把部分译文标为完成。

Frontmatter 和原始 HTML 按原文保留。保护校验不等于语义质量保证；标题翻译后，原文中依赖英文标题的 `#fragment` 目录链接目前可能失效。

```sh
./target/release/md-translator translate --file examples/preview-demo.md
./target/release/md-translator translate --file examples/preview-demo.md --force
./target/release/md-translator translate --file examples/preview-demo.md --no-open
```

## 开发

```sh
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

测试使用本地模拟服务，覆盖整篇翻译、保护校验、错误、取消、缓存、独立配置、环境变量不生效和 Zed 安装。SSH 集成测试通过系统原生 `sftp-server` 验证真实 SFTP 文件操作、中文/特殊字符路径、远端缓存复用、上传恢复、取消和打开失败；macOS 自带该测试程序，Linux 开发环境需要安装提供 `sftp-server` 的 OpenSSH 包。这些测试不调用真实付费模型。设计见[整篇翻译设计](docs/translation-design.md)，参考代码授权见[第三方声明](THIRD_PARTY_NOTICES.md)。
