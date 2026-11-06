# 使用指南

[返回 README](README.md)

## 下载与安装

首次安装按 [README 的开始使用](README.md#开始使用) 操作。不需要编译程序或安装开发工具；本机阅读需要 Zed，翻译需要能访问所配置的服务。远端项目按 [SSH Remote Project](#ssh-remote-project) 操作。

### 选择下载包

安装命令会自动下载并校验程序。手动下载时，打开 [最新版本](https://github.com/againest1/md-translator/releases/latest)，在 **Assets** 中选择对应文件，不要选择 **Source code**：

| 你的设备 | 下载文件 |
| --- | --- |
| Mac，Apple 芯片（M 系列） | `md-translator-aarch64-apple-darwin.tar.gz` |
| Mac，Intel 芯片 | `md-translator-x86_64-apple-darwin.tar.gz` |
| Linux，Intel / AMD 64 位 | `md-translator-x86_64-unknown-linux-musl.tar.gz` |
| Linux，ARM64 | `md-translator-aarch64-unknown-linux-musl.tar.gz` |
| Windows，Intel / AMD 64 位 | `md-translator-x86_64-pc-windows-msvc.zip` |

Mac 可在 **关于本机** 查看芯片；Windows 可在 **设置 → 系统 → 系统信息** 查看系统类型；Linux 在终端运行 `uname -m`，`x86_64` 对应 Intel / AMD，`aarch64` 对应 ARM64。不提供 32 位或 Windows ARM 原生包。macOS 程序要求 macOS 11 或更新版本，Windows 程序要求 Windows 10 或更新版本；Zed 自身的系统要求以其下载页面为准。

程序包不包含 Zed 或翻译服务。Linux 包可在常见 glibc / musl 发行版运行，不需要另装 OpenSSL；Windows 包不需要另装 Visual C++ 运行库。

### 安装位置与手动安装

安装命令不需要管理员权限。macOS / Linux 安装到 `~/.local/bin/md-translator`，Windows 安装到 `%LOCALAPPDATA%\Programs\md-translator\md-translator.exe`。配置、密钥和译文另存于[数据目录](#集中存储与路径)，更新程序不会删除它们。

手动安装时，先解压下载包，将 `md-translator`（Windows 为 `md-translator.exe`）放到固定目录，再使用该程序运行 `install-zed --auto-preview` 和 `configure`。macOS / Linux 可在解压目录运行：

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 ./md-translator "$HOME/.local/bin/md-translator"
"$HOME/.local/bin/md-translator" install-zed --auto-preview
"$HOME/.local/bin/md-translator" configure
```

Windows 将程序复制到上述安装目录后，使用 README 的最后两条 PowerShell 命令。安装 Zed 入口时会备份原配置，保留其它 Task、快捷键和注释。不要将程序留在临时解压目录；Zed 会记住它的绝对路径，移动后需要重新运行 `install-zed`。

下文中的 `md-translator` 表示安装后的程序。若终端提示找不到命令，macOS / Linux 使用 `"$HOME/.local/bin/md-translator"`，Windows PowerShell 使用 `& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe"` 替代命令名。Zed Task 不依赖终端的 PATH 设置。

### 安装问题

| 现象 | 处理方式 |
| --- | --- |
| 下载失败或校验失败 | 检查能否打开 GitHub 下载页面，恢复网络后重跑安装命令；原程序保留。也可下载对应程序包后手动安装 |
| 缺少 `curl`、`tar` 或 SHA-256 工具 | 通过系统软件管理器安装提示的工具，或手动下载；这些工具只用于下载和解压 |
| macOS 提示无法验证开发者 | 当前程序未做 Apple 公证。确认下载来自本仓库后，按系统提示到 **系统设置 → 隐私与安全性** 允许该程序，再重试；不要关闭系统整体保护 |
| Windows 拦截运行 | 当前程序未做 Windows 代码签名。核对下载来源后，按系统提示查看文件并选择是否允许运行；受组织策略限制时联系管理员 |
| Windows 更新提示文件被占用 | 结束正在运行的翻译或配置 Task，关闭使用该程序的终端任务后重试 |
| 安装成功，但本机翻译无法打开 Zed | 确认 Zed 已安装；非标准安装路径可按[集中存储与路径](#集中存储与路径)指定 `--zed-cli` |
| 安装 Zed 入口提示快捷键冲突 | 在 Zed 的 `keymap.json` 中调整提示的冲突绑定后重跑 `install-zed`；冲突时不会覆盖已有配置 |

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
md-translator configure --base-url https://provider.example/v1 --model your-model --set-key
md-translator configure --no-auth
md-translator configure --timeout 180
md-translator configure --test
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
md-translator --data-dir /absolute/path/md-translator configure
md-translator --data-dir /absolute/path/md-translator install-zed
```

安装器会把显式存储参数写进管理的 Task，确保向导与翻译使用相同位置。也支持单独的 `--config`、`--cache-dir`；凭据始终位于配置文件同目录。

`--zed-config` 只指定安装 Task / 快捷键和检查预览设置所用的 Zed 配置目录，不用于读取模型或密钥。Zed CLI 默认从常见安装路径查找；非标准安装可在独立配置中设置 `zed_cli` 的绝对路径，或传入 `--zed-cli /absolute/path/to/zed`。

从本机终端翻译时，可在 `translate` 命令中添加 `--project-root /absolute/path/to/project`，指定已在 Zed 打开的本机项目，普通文件夹项目也适用。Zed 翻译 Task 会自动传入所属项目；本地文件未指定时，按原文所在目录定位窗口。

0.2 及之前版本的 Zed provider、环境变量鉴权和偏好格式不再使用，需要运行配置向导。旧缓存目录不自动删除；已有缓存仍可通过显式 `--cache-dir` 使用；只有原文和完整 provider 指纹一致时才命中。

## SSH Remote Project

在 Zed 的远端终端按[下载与安装](#下载与安装)安装适合远端系统的程序，并在远端配置 provider；本机只需 Zed，要翻译本机项目时再在本机安装程序。远端 Task 使用远端文件、provider 和缓存，远端必须能访问配置的服务地址。完整文档会发送给该服务；远端不需要安装 Zed CLI。

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
md-translator translate --file /path/to/README.md
md-translator translate --file /path/to/README.md --force
md-translator translate --file /path/to/README.md --no-open
```

## 更新与回退

先结束正在进行的翻译，然后再次运行 [README 中的下载命令](README.md#开始使用)。安装器替换固定位置的程序，保留配置、密钥和缓存，无需重新填写 API Key。运行 `md-translator --version` 查看当前版本。

更新后重新执行 `md-translator install-zed` 以刷新 Zed 入口；远端则在对应项目运行 `md-translator install-zed --remote --project-root /远端项目目录`。使用过 `--data-dir`、`--config`、`--cache-dir` 或 `--zed-cli` 时，重新安装入口要带上原参数。原来从源码目录运行的用户，先按新方式安装，再刷新入口，就可以移走旧程序。

需要退回某个已发布版本时，macOS / Linux 在安装命令末尾指定版本：

```sh
curl -fsSL https://github.com/againest1/md-translator/releases/latest/download/install.sh | sh -s -- --version v0.4.1
```

Windows 下载 README 中的安装脚本后，指定版本运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "$env:TEMP\md-translator-install.ps1" -Version v0.4.1
```

将示例版本换成[发布列表](https://github.com/againest1/md-translator/releases)中存在的版本。回退前复制备份整个数据目录，并先阅读该版本说明，旧版本可能不支持较新的配置或缓存。程序不自动检查或后台安装更新。

## 卸载

先结束翻译任务。卸载分为移除 Zed 入口和删除程序，默认保留服务配置、密钥及译文，方便重新安装。

**macOS / Linux**：

```sh
"$HOME/.local/bin/md-translator" uninstall-zed && rm "$HOME/.local/bin/md-translator"
```

**Windows PowerShell**：

```powershell
& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe" uninstall-zed
if ($LASTEXITCODE -eq 0) { Remove-Item "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe" }
```

自定义安装目录时请替换程序路径；安装时指定过 `--zed-config`，卸载入口时也要带上该参数。`uninstall-zed --dry-run` 可以先查看会改动的文件。命令只移除本工具的三个 Task 及指向这些 Task 的快捷键，修改前备份，其它配置保留。**所有 Markdown 默认预览**的偏好保留；需要关闭时，在 Zed 的 `settings.json` 中将 `markdown_preview` 下的 `open_markdown_files_in_preview` 设为 `false`。

**SSH Remote Project**：在删除远端程序前，对安装过 Task 的每个远端项目运行：

```sh
md-translator uninstall-zed --remote --project-root /远端项目目录
```

然后删除远端程序文件。此命令只移除该项目中的 **Markdown (Remote)** Task；本机安装需在本机另行卸载。

若不再需要配置和译文，确认备份后手动删除 `~/.md-translator`（Windows 为 `%USERPROFILE%\.md-translator`）。**删除后 API Key 和全部缓存译文无法恢复**；使用过自定义数据路径时，按原参数找到相应目录。Zed 配置目录中的 `*.md-translator-*.bak` 是安装或卸载时生成的备份，确认不再需要后可单独删除。
