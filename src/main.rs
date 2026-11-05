use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use md_translator::{
    client::translate_whole,
    config::{AuthMode, Preferences, credential, credentials_path, save_key},
    remote::Source,
    remote_workflow::{RemoteTranslateOptions, translate_remote},
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
    #[arg(long, global = true, help = "译文缓存目录")]
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
        #[arg(
            long,
            help = "本地路径、[user@]host:path 或 ssh://[user@]host[:port]/path"
        )]
        file: String,
        #[arg(long, help = "打开译文的本地 Zed 项目目录")]
        project_root: Option<PathBuf>,
        #[arg(long, help = "远端缓存目录（默认远端 ~/.md-translator/cache）")]
        remote_cache_dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        no_open: bool,
    },
    /// 安装 Zed Task、配置向导入口和快捷键，自动备份已有配置
    InstallZed {
        #[arg(long, help = "让所有 Markdown 默认在原生预览中打开")]
        auto_preview: bool,
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
    let explicit_local_cache = cli.cache_dir.is_some();
    let cache_dir = cli.cache_dir.unwrap_or_else(|| root.join("cache"));
    match cli.command {
        Command::Translate {
            file,
            project_root,
            remote_cache_dir,
            force,
            no_open,
        } => match Source::parse(&file)? {
            Source::Local(file) => {
                ensure!(
                    remote_cache_dir.is_none(),
                    "本地文件请使用 --cache-dir，不能使用 --remote-cache-dir"
                );
                let result = translate_file(
                    &TranslateOptions {
                        file,
                        project_root,
                        force,
                        no_open,
                        preferences: config,
                        cache_dir,
                        zed_cli: cli.zed_cli,
                    },
                    cancel,
                )
                .await?;
                println!("{}", result.path.display());
            }
            Source::Remote(file) => {
                ensure!(project_root.is_none(), "--project-root 仅用于本地文件");
                ensure!(
                    !explicit_local_cache,
                    "远端文件请使用 --remote-cache-dir，不能使用本机的 --cache-dir"
                );
                let result = translate_remote(
                    &RemoteTranslateOptions {
                        file,
                        preferences: config,
                        state_dir: root.join("remote-state"),
                        cache_dir: remote_cache_dir,
                        zed_cli: cli.zed_cli,
                        force,
                        no_open,
                    },
                    cancel,
                )
                .await?;
                println!("{}", result.uri);
            }
        },
        Command::InstallZed {
            auto_preview,
            dry_run,
        } => {
            let zed_config = match cli.zed_config {
                Some(path) => path,
                None => storage::zed_config_dir()?,
            };
            let changes = zed::install_with_args(
                &zed_config,
                &std::env::current_exe()?,
                auto_preview,
                dry_run,
                &prefix,
            )?;
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
            println!("Task：Markdown: 配置翻译服务 / Markdown: 翻译全文 / Markdown: 重新翻译全文");
            if cfg!(target_os = "macos") {
                println!("Ctrl+Shift+T：翻译全文；Ctrl+Option+Shift+T：强制重译。");
            } else {
                println!("Ctrl+K 后按 Alt+T：翻译；Ctrl+K 后按 Alt+Shift+T：重译。");
            }
            if auto_preview {
                println!("Markdown 文件默认以原生预览打开。");
            }
        }
        Command::Doctor { check_key } => {
            println!("配置文件：{}", config.display());
            println!("凭据文件：{}", credentials_path(&config).display());
            println!("缓存目录：{}", cache_dir.display());
            let preferences = Preferences::load(&config)?;
            let provider = preferences.provider()?;
            println!("独立 provider：{} / {}", provider.id, provider.model);
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
            no_auth,
            set_key,
            timeout,
            test,
        } => {
            let draft = if base_url.is_none()
                && model.is_none()
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
