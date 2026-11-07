# 使用指南

[返回 README](README.md)

本指南说明如何配置翻译服务、阅读和保存译文，以及遇到问题后怎样继续。已经完成 [README 的开始使用](README.md#开始使用)，可直接跳到[日常使用](#日常使用)。

[安装](#下载与安装) · [配置服务](#独立配置向导) · [日常使用](#日常使用) · [范围与限制](#翻译范围与显示限制) · [失败恢复](#失败后继续翻译) · [问题反馈](#问题反馈与诊断) · [存储与迁移](#集中存储与路径) · [更新](#更新与回退) · [卸载](#卸载)

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

Windows 将程序复制到上述安装目录后，运行 README 中的 PowerShell `install-zed` 命令，再按第二步配置服务。安装 Zed 入口时会备份原配置，保留其它 Task、快捷键和注释。普通 Task 在执行机器的 PATH 中查找 `md-translator`，找不到时使用上述默认安装目录；不要将程序留在临时解压目录。使用自定义安装目录时，将该目录加入执行机器的 PATH。

`--auto-preview` 会让所有 Markdown 文件默认以预览打开。想保留当前打开方式，安装时省略此参数；安装后也可按[预览设置](#预览设置)调整。

下文中的 `md-translator` 表示安装后的程序。若终端提示找不到命令，macOS / Linux 使用 `"$HOME/.local/bin/md-translator"`，Windows PowerShell 使用 `& "$env:LOCALAPPDATA\Programs\md-translator\md-translator.exe"` 替代命令名。默认安装位置的程序无需额外设置 PATH 即可从普通 Zed Task 启动。

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
| SSH 翻译报错仍显示本机程序路径 | 更新两端程序，并在本机重新运行 `install-zed` 刷新普通 Task，再从 `task: spawn` 启动；不要直接重跑旧任务终端中的命令 |
| Task 提示找不到 `md-translator` | 在项目所在机器确认程序已安装到默认目录，或将自定义目录加入该机器的 PATH；远端检查需使用 Zed 的远端终端 |

## 独立配置向导

在 Zed 按 macOS `Cmd+Shift+P` 或 Linux / Windows `Ctrl+Shift+P` 打开[命令面板](https://zed.dev/docs/command-palette)，运行 `task: spawn`，选择 **Markdown: 配置翻译服务**，在打开的任务终端中填写配置。也可在本机终端运行 `md-translator configure`。

模型必须支持 OpenAI 兼容的 Chat Completions，可使用云端服务或本地模型接口；程序不附带模型或服务额度。翻译服务需单独配置，不沿用 Zed 中的模型或 API Key，一次只使用一个服务、一个模型。

按向导依次填写：

| 项目 | 填写方法 |
| --- | --- |
| 接口地址 | 填服务商或本地模型服务提供的 API 地址，例如 `https://provider.example/v1`（示例，请替换为实际地址） |
| 模型名称 | 从所用服务的 API 文档中复制模型标识，确认该服务已开通这个模型 |
| 鉴权方式 | 服务要求 API Key 时选择“保存 API Key”；明确无需密钥的服务选择“无需鉴权” |
| API Key | 在终端中隐藏输入；再次配置同一服务时可留空保留，改变接口地址后需为新地址填写密钥 |
| 请求超时 | 单次请求的等待上限，单位为秒；默认 120，可设为 1–3600，模型响应较慢时可适当增加 |

使用本地模型前，先启动其兼容 API 服务。接口地址要能从程序运行的机器直接访问；在 Remote Project 中，`localhost` / `127.0.0.1` 指向远端机器。

最后选择“保存”“保存并测试”或“取消”；退出向导不会提交未确认的配置。默认只保存，不请求模型。想确认服务能否使用，可选择“保存并测试”，发送一段固定短文本；这次请求也可能计费。测试失败时已保存的配置保留，可修改后重试。

向导将设置保存到[数据目录](#集中存储与路径)的 `config.json`，例如：

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

`auth` 可以是 `api_key` 或 `none`。API Key 由向导保存到独立的 `credentials.json`，不写入命令行、日志或译文缓存。**凭据文件是明文，请勿公开或提交到代码仓库**；macOS / Linux 上新建的配置和凭据文件仅当前用户可读写。分享配置排查问题时，不要附上凭据文件。

服务要求额外参数时，可在 `config.json` 的 `provider` 下添加 `reasoning_effort`、`max_completion_tokens` 或 `max_tokens`（输出上限只能选择一项）、`custom_headers`，取值以服务商说明为准。`custom_headers` 不允许覆盖鉴权或 Content-Type。

工具不读取环境变量中的密钥或代理设置，也不支持配置中的环境变量插值。请填写程序所在机器能够直接访问的服务地址；旧 `--api-key-env` 参数已移除，API Key 请通过向导保存。

修改非敏感配置可以使用参数；密钥必须在终端中隐藏输入：

```sh
md-translator configure --base-url https://provider.example/v1 --model your-model --set-key
md-translator configure --no-auth
md-translator configure --timeout 180
```

### 检查配置与连接

根据要确认的问题选择检查方式：

| 想确认什么 | 操作 | 结果说明 |
| --- | --- | --- |
| 使用的是哪份配置、哪个模型，译文存在哪里 | `md-translator doctor` | 显示配置和缓存路径、模型、请求超时及预览设置；不请求模型 |
| API Key 是否已保存且能读取 | `md-translator doctor --check-key` | 在上述检查中增加凭据读取检查，不显示密钥、不请求模型；服务是否接受密钥需另测连接 |
| 服务能否实际完成一次翻译 | 在向导中选择“保存并测试”，或运行 `md-translator configure --test` | 发送固定短文本请求，看到“测试成功。”后可尝试自己的文档；可能计费 |

短文本测试成功，只说明这次示例请求成功。长文仍可能超时、超出模型限制或无法完整翻译；遇到这些情况按[失败恢复](#失败后继续翻译)处理。

使用过 `--data-dir`、`--config`、`--cache-dir` 等自定义路径时，检查命令也要带上相同参数。Remote Project 的服务与凭据在远端终端检查；预览偏好在本机 Zed 中按[预览设置](#预览设置)核对。

## 日常使用

先在 Zed 中打开原文所在的项目。从 `task: spawn` 选择 **Markdown: 翻译全文** 或 **Markdown: 重新翻译全文**；快捷键只是另一种触发入口。macOS / Linux 本机项目与 SSH Remote Project 共用这些入口，远端准备与其它平台的操作见[SSH Remote Project](#ssh-remote-project)。

| 操作 | macOS | Linux / Windows |
| --- | --- | --- |
| 翻译全文或打开缓存 | `Ctrl+Shift+T` | `Ctrl+K` 后按 `Alt+T` |
| 强制重新翻译 | `Ctrl+Option+Shift+T` | `Ctrl+K` 后按 `Alt+Shift+T` |

Task 会先保存当前修改；未命名文档需要先保存为 `.md` 或 `.markdown`。翻译固定为简体中文，中文或中英混排文档也会发送给模型；若只需阅读已有中文内容，无需运行翻译。

本机首次翻译自动打开 `文件名 · 简体中文.md`；SSH 远端翻译在任务终端点击链接打开。等待完成后即可阅读，无需关闭再打开。已有译文时使用缓存，不请求模型；原文或模型配置改变后会重新翻译。译文顶部提供“返回原文”链接；需要并排阅读时可手动拖动标签页。

本地译文在原文所在的项目窗口中以标签页打开，不新增 Project Panel 条目。旧版本已经加入 Project Panel 的缓存条目，请右键选择 **Remove from Project** 后再从原文打开译文；此操作保留磁盘上的缓存文件。

需要重新翻译时，先点击“返回原文”，再触发翻译 Task。

### 保存或分享译文

译文是保存在[缓存目录](#集中存储与路径)中的 Markdown 文件，关闭标签页或重启 Zed 后仍会保留。重新阅读时，从原文再次运行 **Markdown: 翻译全文** 即可。

需要编辑、提交到项目或分享给别人时，请复制译文正文，另存为自己的 `.md` 文件。不要把手动修改只留在缓存预览中：再次打开或重新翻译时，该预览会重新生成。复制时可省略顶部的“返回原文”说明；图片和文件链接可能包含原机器上的绝对路径，移到其它位置后需按新文件位置调整。

### 预览设置

| 想要的阅读方式 | 操作 |
| --- | --- |
| 只预览当前译文 | 在译文标签页点击 **Preview** |
| 所有 Markdown 文件默认进入预览 | 在本机运行 `md-translator install-zed --auto-preview` |
| 恢复默认编辑 Markdown | 在本机 Zed 的 `settings.json` 中，将 `markdown_preview` 下的 `open_markdown_files_in_preview` 设为 `false` |

安装入口时指定过自定义路径，请在重新运行 `install-zed` 时带上原参数。Remote Project 的预览偏好也在本机 Zed 中设置；不要在远端运行 `install-zed --auto-preview`。

## 集中存储与路径

配置、密钥和缓存保存在程序运行的机器上。Remote Project 使用远端账户的目录；本机项目使用本机账户的目录。默认保存在该账户的 `.md-translator` 目录：

```text
~/.md-translator/
├── config.json       （服务配置）
├── credentials.json  （API Key，明文）
└── cache/            （译文）
```

更换服务、模型或请求选项后会重新翻译；只轮换同一服务的 API Key 仍可使用已有缓存。缓存长期保留，不自动过期或清理。迁移存储位置时，请复制完整数据目录。

原文改变后会重新翻译全文。只保存而内容未变，或将内容改回曾经翻译过的版本，仍可复用该版本的译文。

原文内容改变后仍更新同一预览，无需手动更换标签页。本机预览中的相对图片和文件链接按原文所在目录查找；远端资源限制见[SSH Remote Project](#ssh-remote-project)。同一文档正在翻译时，再次触发不会重复请求。

需要更换数据目录时，使用显式参数；修改 `HOME` / `XDG_*` 不会改变存储位置：

```sh
md-translator --data-dir /absolute/path/md-translator configure
md-translator --data-dir /absolute/path/md-translator install-zed
```

配置服务和安装 Task 时使用相同的路径参数，之后从 Zed 触发即可使用该位置。也支持用 `--config`、`--cache-dir` 分别指定配置文件和缓存目录；凭据始终位于配置文件同目录。

`--zed-config` 只指定安装 Task / 快捷键和检查预览设置所用的 Zed 配置目录，不用于读取模型或密钥。Zed CLI 默认从常见安装路径查找；非标准安装可在独立配置中设置 `zed_cli` 的绝对路径，或传入 `--zed-cli /absolute/path/to/zed`。

从本机终端翻译时，可在 `translate` 命令中添加 `--project-root /absolute/path/to/project`，指定已在 Zed 打开的本机项目，普通文件夹项目也适用。Zed 翻译 Task 会自动传入所属项目；本地文件未指定时，按原文所在目录定位窗口。

从 0.2 或更早版本升级后，请重新运行配置向导，不再沿用 Zed provider 或环境变量中的密钥。旧缓存目录不自动删除；可通过 `--cache-dir` 指向原目录，兼容且原文、服务配置未变的译文可继续使用，其余文档会重新翻译。

## SSH Remote Project

本机与远端均为 macOS / Linux 时，本机按 README 安装程序和普通 Zed Task，远端安装适合其系统的程序并配置服务后，即可使用相同的 Task 和快捷键。普通 Task 会在项目所在机器查找程序，SSH 会话自动使用远端模式，无需逐个远端项目安装 Task。

Windows 本机 Task 使用 PowerShell，不能直接用于 macOS / Linux 远端；Windows 客户端请使用下方的[显式远端项目入口](#显式远端模式与旧入口)。

远端翻译使用远端文件、provider 和缓存，远端必须能访问配置的服务地址。完整文档会发送给该服务；远端不需要安装 Zed CLI，也不会启动远端桌面的 Zed。

### 准备远端程序与服务

在 Zed 的 **Remote Project 终端**中按[下载与安装](#下载与安装)安装程序。macOS / Linux 默认安装后可运行：

```sh
"$HOME/.local/bin/md-translator" configure
```

已有 provider 配置时可省略。也可在当前 Remote Project 的 `task: spawn` 中选择 **Markdown: 配置翻译服务**，保存到远端。远端使用自定义安装目录时，将其加入远端 PATH。

从旧版升级时，两端都需更新程序；在 **本机终端**重新运行 `md-translator install-zed` 刷新已有普通 Task，然后从 `task: spawn` 重新选择任务。旧任务终端的重跑操作可能仍使用原命令。本机的快捷键和 Markdown 预览偏好继续在本机 Zed 设置。

| 操作 | 在 `task: spawn` 中选择 |
| --- | --- |
| 翻译全文或使用缓存 | **Markdown: 翻译全文** |
| 强制重新请求模型 | **Markdown: 重新翻译全文** |
| 修改远端 provider | **Markdown: 配置翻译服务** |

### 点击打开译文

1. 在当前 Remote Project 中打开原文，再运行 **Markdown: 翻译全文** 或使用翻译快捷键。Task 会先保存当前文档。
2. 任务终端出现 `file://...` 译文链接。macOS 按住 **Cmd** 点击；Linux / Windows 按住 **Ctrl** 点击。请在当前 Zed 任务终端点击，让它按所属远端项目打开文件。
3. 译文以标签页打开；尚未完成时显示进度，完成后更新同一文件。未启用默认预览时，手动点击 **Preview**。

任务结束后仍可点击终端中的完整链接，再次打开已有译文也用同样的方式。如果点击未生效，复制终端最后输出的原始绝对路径，在当前 Remote Project 中用打开文件操作选择该文件。

缓存无需加入 Project Panel；从缓存预览重译时，先返回原文。返回原文和普通文件链接按远端路径打开。项目外缓存中的图片仍受 Zed 远端预览的路径解析限制，可能不显示，请返回原文查看；程序不下载或复制图片。

### 显式远端模式与旧入口

也可在远端终端直接运行：

```sh
md-translator translate --remote --file /home/user/project/README.md
md-translator translate --remote --file /home/user/project/README.md --force
```

`--remote` 不启动 Zed，即使远端已安装 Zed CLI 也一样。它输出适合 Zed 终端点击的链接；普通 SSH 终端不保证按同一个 Remote Project 打开。

普通 Zed Task 已启用 `--auto-remote`，按 `SSH_CONNECTION` 判断 SSH 会话；手动运行 CLI 时可添加此参数，未添加时仍按原有本机方式运行。若远端 shell 清除了 SSH 会话信息，可显式使用 `--remote`。

已有的 **Markdown (Remote)** 项目 Task 继续可用；Windows 客户端、需要固定远端模式或项目专用参数时，可在远端运行 `md-translator install-zed --remote --project-root /远端项目目录`。它只管理该项目的 `.zed/tasks.json`，不修改本机快捷键和预览设置。从 `task: spawn` 选择名称含 **(Remote)** 的对应任务；需要快捷键时，在本机 Zed 为该名称另设绑定。macOS / Linux 普通流程无需这一步；旧入口不用时可按[卸载](#卸载)移除。

### 存储位置与旧版本

默认使用远端账户的 `~/.md-translator/`。需要项目专用的其它位置时，在远端配置和安装显式远端 Task 时传入相同参数：

```sh
md-translator --data-dir /absolute/remote/data configure
md-translator --data-dir /absolute/remote/data install-zed --remote --project-root /home/user/project
```

`--data-dir`、`--config`、`--cache-dir` 都指向程序运行所在机器；安装器会把显式参数写进 Task。本机和远端不会同步配置、密钥或缓存。

旧版 `host:path`、`ssh://...` 文件输入和 `--remote-cache-dir` 已停用。请在远端从原文重新翻译；旧的本机 SSH 缓存和远端缓存不会自动迁移或删除。旧配置中的 `ssh` 字段会被忽略，重新保存配置时移除，其 provider 保留。含冒号的普通文件名可用 `./文件名.md` 或绝对路径传入。

## 翻译和失败恢复

每次翻译会将整篇文档发送给配置的服务，可能产生费用；包含敏感内容时，请先确认该服务适合处理这些数据。

### 翻译范围与显示限制

| 文档中的内容 | 译文中的结果 |
| --- | --- |
| 正文、标题、列表和表格中的自然语言 | 英文翻译为简体中文，按 Markdown 阅读；已有中文按原意保留，中英混排文档也会整篇发送给模型 |
| 链接显示文字、图片替代文字和普通引文 | 文字参与翻译；链接和图片仍指向原目标 |
| 行内代码、代码块、公式和文件路径 | 按原文保留，代码中的注释和字符串也不翻译 |
| 文档开头的元数据（Frontmatter）、HTML 块 | 整块按原文保留，其中的英文也不翻译 |
| 正文中的行内 HTML 标签 | 标签保留，标签之间的普通文字可以翻译，例如 `Read <span>Hello</span>` 中的 `Hello` |
| 图片本身 | 不翻译图片中的文字；本机相对资源按原文位置查找，远端预览中的部分图片可能不显示，可返回原文查看 |

标题翻译后，原文中依赖英文标题的 `#fragment` 目录链接目前可能失效。译文也可能出现语义或术语错误，重要内容请对照原文确认。

### 失败后继续翻译

译文未能保留代码、路径等内容时，会自动再请求一次纠正，因此一次翻译最多产生两次模型请求，并可能增加费用。服务错误、额度不足或输出截断时会停止并提示原因，不自动重试、分片或切换模型。在任务终端中按 `Ctrl+C` 可取消。

翻译失败或取消后，上次成功的译文会保留；没有成功过的文档需要重新运行翻译。未完成的内容不会保存为译文。

以下 Task 名称适用于本机项目和 SSH Remote Project；使用旧项目入口时，选择名称含 **(Remote)** 的对应 Task。

| 遇到的问题 | 处理方式 |
| --- | --- |
| 尚未配置服务、鉴权失败 | 运行 **Markdown: 配置翻译服务**，检查地址、模型、API Key 和账户权限，再选择“保存并测试” |
| 连接失败或超时 | 确认程序所在机器能直接访问服务，必要时在向导中增加请求超时，再运行 **Markdown: 翻译全文** |
| 额度不足或请求受限 | 检查服务账户额度和用量限制，恢复后重新运行翻译 |
| 文档超出模型上下文限制 | 选择支持更长输入的模型；程序不自动拆分文档 |
| 输出被截断 | 在[配置文件](#独立配置向导)中提高输出上限，或更换支持更长输出的模型 |
| 纠正后仍无法保留代码、路径等内容 | 尝试更换模型后重新翻译；如需继续阅读，可使用上次成功的译文 |

排除问题后通常运行 **Markdown: 翻译全文** 即可；只想重新生成已有译文时，再选择 **Markdown: 重新翻译全文**。

在终端中使用时，可按需要选择普通翻译、强制重译或仅生成译文文件：

```sh
md-translator translate --file /path/to/README.md
md-translator translate --file /path/to/README.md --force
md-translator translate --file /path/to/README.md --no-open
```

## 问题反馈与诊断

按错误提示处理后仍无法解决，可在 [GitHub Issues](https://github.com/againest1/md-translator/issues) 反馈。请尽量提供：

- 操作步骤、预期结果和实际现象，以及任务终端中的错误文字。
- 操作系统、Zed 版本、`md-translator --version` 输出，以及使用的是本机项目还是 SSH Remote Project。
- 所用翻译服务和模型；配置或路径问题可附上 `md-translator doctor --check-key` 的输出，命令用法见[检查配置与连接](#检查配置与连接)。
- 如问题只在特定文档中出现，提供可公开的最小 Markdown 样例；也可先用[预览示例](examples/preview-demo.md)确认是否出现同样的问题。

这些信息不会自动上传，由你选择是否附在反馈中。`doctor --check-key` 不显示 API Key，但会显示本机或远端路径，分享前请检查是否需要隐藏私人信息。不要附上 `credentials.json` 或密钥；截图、文档样例和服务返回的错误文字也请先检查。

## 更新与回退

先结束正在进行的翻译，然后再次运行 [README 中的下载命令](README.md#开始使用)。安装器替换固定位置的程序，保留配置、密钥和缓存，无需重新填写 API Key。运行 `md-translator --version` 查看当前版本。

更新后在本机重新执行 `md-translator install-zed` 刷新普通 Zed 入口；SSH 远端更新程序即可，无需逐个项目安装 Task。若继续使用旧的 **Markdown (Remote)** 项目入口，则在远端对应项目重新运行 `md-translator install-zed --remote --project-root /远端项目目录`。使用过 `--data-dir`、`--config`、`--cache-dir` 或 `--zed-cli` 时，重新安装入口要带上原参数。原来从源码目录运行的用户，先按新方式安装，再刷新入口，就可以移走旧程序。

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

自定义安装目录时请替换程序路径；安装时指定过 `--zed-config`，卸载入口时也要带上该参数。`uninstall-zed --dry-run` 可以先查看会改动的文件。命令只移除本工具的三个 Task 及指向这些 Task 的快捷键，修改前备份，其它配置保留。**所有 Markdown 默认预览**的偏好保留；需要关闭时，按[预览设置](#预览设置)恢复默认编辑方式。

**SSH Remote Project**：使用普通 Task 时，删除远端程序文件即可；本机入口按上面的步骤另行卸载。若曾安装 **Markdown (Remote)** 项目 Task，在删除远端程序前，对相应项目运行：

```sh
md-translator uninstall-zed --remote --project-root /远端项目目录
```

然后删除远端程序文件。此命令只移除该项目中的 **Markdown (Remote)** Task；本机安装需在本机另行卸载。

若不再需要配置和译文，确认备份后手动删除 `~/.md-translator`（Windows 为 `%USERPROFILE%\.md-translator`）。**删除后 API Key 和全部缓存译文无法恢复**；使用过自定义数据路径时，按原参数找到相应目录。Zed 配置目录中的 `*.md-translator-*.bak` 是安装或卸载时生成的备份，确认不再需要后可单独删除。
