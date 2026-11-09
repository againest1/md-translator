use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use md_translator::{
    client::translate_whole,
    config::{AuthMode, Preferences, REASONING_EFFORTS, credential, credentials_path, save_key},
    storage, wizard,
    workflow::{TranslateOptions, translate_file},
    zed,
};
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    version,
    about = "使用独立配置整篇英译中，通过 Zed 展示持久缓存译文；不使用环境变量配置"
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "配置与缓存的根目录（默认 ~/.md-translator）"
    )]
    data_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Zed 配置目录，仅用于安装 Task/快捷键和检查预览设置"
    )]
    zed_config: Option<PathBuf>,
    #[arg(long, global = true, help = "独立配置文件；凭据文件放在相同目录")]
    config: Option<PathBuf>,
    #[arg(long, global = true, help = "程序运行所在机器上的译文缓存目录")]
    cache_dir: Option<PathBuf>,
    #[arg(long, global = true, help = "Zed CLI 的绝对路径")]
    zed_cli: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 翻译整个 Markdown 文件；缓存命中时不发送请求
    Translate {
        #[arg(long, value_parser = storage::source_path, help = "程序运行所在机器上的 Markdown 文件路径")]
        file: PathBuf,
        #[arg(long, help = "原文所在的 Zed 项目目录")]
        project_root: Option<PathBuf>,
        #[arg(long, hide = true, help = "已停用；请在远端使用 --cache-dir")]
        remote_cache_dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        no_open: bool,
        #[arg(
            long,
            help = "在 Remote Project 中运行：输出可点击译文链接，不启动 Zed"
        )]
        remote: bool,
        #[arg(long, help = "按 SSH 会话自动选择远端模式（普通 Zed Task 已启用）")]
        auto_remote: bool,
    },
    /// 安装 Zed Task、配置向导入口和快捷键，自动备份已有配置
    InstallZed {
        #[arg(long, help = "让所有 Markdown 默认在原生预览中打开")]
        auto_preview: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// 移除翻译 Task 与对应快捷键；保留服务配置、密钥、缓存和预览偏好
    UninstallZed {
        #[arg(long)]
        dry_run: bool,
    },
    /// 检查独立配置、缓存与预览；不请求模型，不显示密钥
    Doctor {
        #[arg(long)]
        check_key: bool,
    },
    /// 打开交互配置向导；也可以通过参数更新非敏感配置
    Configure {
        #[arg(long)]
        base_url: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long, value_parser = REASONING_EFFORTS, help = "推理强度；default 不发送此参数，未配置时默认 low")]
        effort: Option<String>,
        #[arg(long, conflicts_with = "set_key", help = "当前服务无需鉴权")]
        no_auth: bool,
        #[arg(long, help = "在终端中隐藏输入 API Key，留空可保留已有密钥")]
        set_key: bool,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(long, help = "保存后发送一次固定短文本请求测试连接")]
        test: bool,
    },
}

#[tokio::main]
async fn main() {
    let cancel = CancellationToken::new();
    let listener = cancel.clone();
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            if let Ok(mut term) =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            {
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            } else {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        listener.cancel();
    });
    if let Err(error) = run(Cli::parse(), &cancel).await {
        eprintln!("{error:#}");
        std::process::exit(if cancel.is_cancelled() { 130 } else { 1 });
    }
}

async fn run(cli: Cli, cancel: &CancellationToken) -> Result<()> {
    let mut prefix = Vec::new();
    for (flag, value) in [
        ("--data-dir", &cli.data_dir),
        ("--config", &cli.config),
        ("--cache-dir", &cli.cache_dir),
        ("--zed-cli", &cli.zed_cli),
    ] {
        if let Some(value) = value {
            let absolute = if value.is_absolute() {
                value.clone()
            } else {
                std::env::current_dir()?.join(value)
            };
            prefix.extend([flag.into(), absolute.to_string_lossy().into_owned()]);
        }
    }
    let root = match cli.data_dir {
        Some(path) => path,
        None => storage::data_dir()?,
    };
    let config = cli.config.unwrap_or_else(|| root.join("config.json"));
    let cache_dir = cli.cache_dir.unwrap_or_else(|| root.join("cache"));
    match cli.command {
        Command::Translate {
            file,
            project_root,
            remote_cache_dir,
            force,
            no_open,
            remote,
            auto_remote,
        } => {
            ensure!(
                remote_cache_dir.is_none(),
                "--remote-cache-dir 已停用；请在远端运行程序，用 --cache-dir 指定远端缓存目录。"
            );
            let result = translate_file(
                &TranslateOptions {
                    file,
                    project_root,
                    force,
                    no_open,
                    remote: remote
                        || (auto_remote
                            && std::env::var_os("SSH_CONNECTION")
                                .is_some_and(|value| !value.is_empty())),
                    preferences: config,
                    cache_dir,
                    zed_cli: cli.zed_cli,
                },
                cancel,
            )
            .await?;
            println!("{}", result.path.display());
        }
        Command::InstallZed {
            auto_preview,
            dry_run,
        } => {
            let zed_config = match cli.zed_config {
                Some(path) => path,
                None => storage::zed_config_dir()?,
            };
            let changes = zed::install_with_args(&zed_config, auto_preview, dry_run, &prefix)?;
            println!(
                "{}",
                if dry_run {
                    "计划更新："
                } else {
                    "已更新："
                }
            );
            if changes.is_empty() {
                println!("配置已是最新，无需改动。");
            }
            for change in changes {
                println!("{}", change.path.display());
                if let Some(backup) = change.backup {
                    println!("  备份：{}", backup.display());
                }
            }
            if cfg!(target_os = "macos") {
                println!("Ctrl+Shift+T：翻译全文；Ctrl+Option+Shift+T：强制重译。");
            } else {
                println!("Ctrl+K 后按 Alt+T：翻译；Ctrl+K 后按 Alt+Shift+T：重译。");
            }
            println!("Task：Markdown: 配置翻译服务 / Markdown: 翻译全文 / Markdown: 重新翻译全文");
            if cfg!(windows) {
                println!(
                    "macOS / Linux SSH Remote Project 请在远端终端运行 md-translator translate --remote --file \"/远端/原文.md\"，替换为实际路径后翻译并点击输出的链接打开。"
                );
            } else {
                println!(
                    "同一 Task 和快捷键适用于本机与 macOS / Linux SSH Remote Project；远端需安装 md-translator 并配置服务，无需逐个项目安装 Task。"
                );
            }
            println!(
                "程序在执行机器的 PATH 或默认安装目录查找；远端译文在当前 Zed 终端 Cmd+Click / Ctrl+Click 打开。"
            );
            if auto_preview {
                println!("Markdown 文件默认以原生预览打开。");
            }
        }
        Command::UninstallZed { dry_run } => {
            let directory = match cli.zed_config {
                Some(path) => path,
                None => storage::zed_config_dir()?,
            };
            let changes = zed::uninstall(&directory, dry_run)?;
            println!(
                "{}",
                if dry_run {
                    "计划移除翻译入口："
                } else {
                    "已移除翻译入口："
                }
            );
            if changes.is_empty() {
                println!("没有需要移除的翻译入口。");
            }
            for change in changes {
                println!("{}", change.path.display());
                if let Some(backup) = change.backup {
                    println!("  备份：{}", backup.display());
                }
            }
            println!("服务配置、密钥、缓存和 Markdown 预览偏好已保留。");
        }
        Command::Doctor { check_key } => {
            println!("配置文件：{}", config.display());
            println!("凭据文件：{}", credentials_path(&config).display());
            println!("缓存目录：{}", cache_dir.display());
            let preferences = Preferences::load(&config)?;
            let provider = preferences.provider()?;
            println!("独立 provider：{} / {}", provider.id, provider.model);
            println!(
                "推理强度（effort）：{}",
                provider
                    .request_options
                    .get("reasoning_effort")
                    .and_then(|value| value.as_str())
                    .unwrap_or("default（由服务决定）")
            );
            println!("请求超时：{} 秒", preferences.timeout()?.as_secs());
            if check_key {
                let credential = credential(&provider, &credentials_path(&config))?;
                println!("鉴权：{}（未显示密钥）", credential.source);
            }
            let zed_config = match cli.zed_config {
                Some(path) => path,
                None => storage::zed_config_dir()?,
            };
            let preview = storage::read_jsonc(&zed_config.join("settings.json"))
                .ok()
                .and_then(|s| {
                    s.pointer("/markdown_preview/open_markdown_files_in_preview")
                        .and_then(|v| v.as_bool())
                });
            println!(
                "自动预览：{}",
                if preview == Some(true) {
                    "已开启"
                } else {
                    "未开启或未找到 Zed 设置"
                }
            );
        }
        Command::Configure {
            base_url,
            model,
            effort,
            no_auth,
            set_key,
            timeout,
            test,
        } => {
            let draft = if base_url.is_none()
                && model.is_none()
                && effort.is_none()
                && !no_auth
                && !set_key
                && timeout.is_none()
                && !test
            {
                wizard::configure(&config)?
            } else {
                let mut preferences = Preferences::load(&config)?;
                let provider = preferences.provider.get_or_insert_default();
                if let Some(url) = base_url {
                    provider.base_url = url;
                }
                if let Some(model) = model {
                    provider.model = model;
                }
                if let Some(effort) = effort {
                    provider.reasoning_effort = Some(effort);
                }
                if no_auth {
                    provider.auth = AuthMode::None;
                }
                if set_key {
                    provider.auth = AuthMode::ApiKey;
                }
                if let Some(seconds) = timeout {
                    preferences.request_timeout_seconds = Some(seconds);
                }
                preferences.provider()?;
                preferences.timeout()?;
                let key = if set_key {
                    wizard::prompt_key(&config, &preferences)?
                } else {
                    None
                };
                Some(wizard::Draft {
                    preferences,
                    key,
                    test,
                })
            };
            let Some(draft) = draft else {
                println!("已取消，配置未修改。");
                return Ok(());
            };
            ensure!(!cancel.is_cancelled(), "配置已取消，未保存。");
            let provider = draft.preferences.provider()?;
            draft.preferences.timeout()?;
            if let Some(key) = draft.key {
                save_key(&provider, &credentials_path(&config), &key)?;
            }
            draft.preferences.save(&config)?;
            println!("已保存：{}", config.display());
            if draft.test {
                let credentials = credential(&provider, &credentials_path(&config))?;
                println!("正在使用固定短文本测试连接…");
                // There are no protected tokens, so this test needs only the main request.
                translate_whole(
                    &provider,
                    &credentials.key,
                    "Hello.",
                    draft.preferences.timeout()?,
                    cancel,
                    &|_| {},
                )
                .await?;
                println!("测试成功。");
            }
        }
    }
    Ok(())
}
