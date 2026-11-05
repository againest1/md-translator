use crate::{
    cache::{CachePaths, Cached, DocumentLock},
    client::translate_whole,
    config::{Preferences, credential, credentials_path},
    remote::{RemoteFile, RemoteFs},
    storage::{atomic_write, hash, zed_cli},
    workflow::{escape, header_with_url},
    zed::open_remote_in_zed,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

pub struct RemoteTranslateOptions {
    pub file: RemoteFile,
    pub preferences: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: Option<PathBuf>,
    pub zed_cli: Option<PathBuf>,
    pub force: bool,
    pub no_open: bool,
}

pub struct RemoteOutcome {
    pub status: &'static str,
    pub uri: String,
    pub requests: usize,
}

#[derive(Serialize, Deserialize)]
struct Pending {
    origin: String,
    key: String,
    result_hash: String,
    markdown: String,
    warnings: Vec<String>,
}

fn is_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

async fn cached(remote: &RemoteFs, paths: &CachePaths) -> Result<Option<(String, Vec<String>)>> {
    let Some(bytes) = remote
        .read_optional(&paths.entry.join("manifest.json"))
        .await?
    else {
        return Ok(None);
    };
    let Ok(metadata) = serde_json::from_slice::<Cached>(&bytes) else {
        return Ok(None);
    };
    if metadata.key != paths.key || !is_digest(&metadata.result_hash) {
        return Ok(None);
    }
    let Some(bytes) = remote
        .read_optional(
            &paths
                .entry
                .join(format!("result-{}.md", metadata.result_hash)),
        )
        .await?
    else {
        return Ok(None);
    };
    let Ok(markdown) = String::from_utf8(bytes) else {
        return Ok(None);
    };
    Ok((hash(&markdown) == metadata.result_hash).then_some((markdown, metadata.warnings)))
}

fn load_pending(path: &Path, origin: &str, key: &str) -> Result<Option<Pending>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let pending: Pending = serde_json::from_slice(&bytes)
        .context("待上传译文记录损坏，请检查本机 remote-state/pending 目录")?;
    ensure!(
        pending.origin == origin
            && pending.key == key
            && hash(&pending.markdown) == pending.result_hash,
        "待上传译文摘要不一致，未发送新的模型请求；请检查本机 pending 文件。"
    );
    Ok(Some(pending))
}

fn remove_pending(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn render(heading: &str, markdown: &str, warnings: &[String]) -> String {
    let notices = if warnings.is_empty() {
        String::new()
    } else {
        format!(
            "> {}\n\n",
            warnings
                .iter()
                .map(|s| escape(s))
                .collect::<Vec<_>>()
                .join("；")
        )
    };
    // SSH documents keep their original resource destinations; no image downloading or rebasing.
    format!("{heading}{notices}{markdown}")
}

async fn publish(
    remote: &mut RemoteFs,
    paths: &CachePaths,
    heading: &str,
    pending: &Pending,
) -> Result<()> {
    remote
        .atomic_write(
            &paths
                .entry
                .join(format!("result-{}.md", pending.result_hash)),
            pending.markdown.as_bytes(),
        )
        .await?;
    let metadata = Cached {
        key: paths.key.clone(),
        result_hash: pending.result_hash.clone(),
        warnings: pending.warnings.clone(),
    };
    remote
        .atomic_write(
            &paths.entry.join("manifest.json"),
            &serde_json::to_vec_pretty(&metadata)?,
        )
        .await?;
    remote
        .atomic_write(
            &paths.view,
            render(heading, &pending.markdown, &pending.warnings).as_bytes(),
        )
        .await?;
    Ok(())
}

async fn show(uri: &str, binary: Option<&Path>) -> Result<()> {
    if let Some(binary) = binary {
        open_remote_in_zed(uri, binary).await
            .with_context(|| format!("译文已保存在远端，打开 Zed 失败。重新运行普通翻译会复用缓存，或在本机用 Zed 打开：{uri}"))?;
    }
    Ok(())
}

pub async fn translate_remote(
    options: &RemoteTranslateOptions,
    cancel: &CancellationToken,
) -> Result<RemoteOutcome> {
    let preferences = Preferences::load(&options.preferences)?;
    let provider = preferences.provider()?;
    let timeout = preferences.timeout()?;
    let ssh = preferences.ssh.clone().unwrap_or_default();
    let binary = if options.no_open {
        None
    } else {
        Some(
            options
                .zed_cli
                .clone()
                .or(preferences.zed_cli.clone())
                .map(Ok)
                .unwrap_or_else(zed_cli)?,
        )
    };
    let mut remote = RemoteFs::connect(&options.file, &ssh, cancel).await?;
    let result = translate_connected(
        options,
        &preferences,
        &provider,
        timeout,
        binary.as_deref(),
        &mut remote,
        cancel,
    )
    .await;
    // Closing the SSH channel must not turn a completely committed translation into a failed one.
    if let Err(error) = remote.close().await
        && result.is_ok()
    {
        eprintln!("译文处理已完成，关闭 SSH 会话时收到提示：{error}");
    }
    result
}

async fn translate_connected(
    options: &RemoteTranslateOptions,
    preferences: &Preferences,
    provider: &crate::config::Provider,
    timeout: std::time::Duration,
    binary: Option<&Path>,
    remote: &mut RemoteFs,
    cancel: &CancellationToken,
) -> Result<RemoteOutcome> {
    let configured_root = options
        .cache_dir
        .clone()
        .or_else(|| preferences.ssh.as_ref().and_then(|s| s.cache_dir.clone()))
        .unwrap_or_else(|| PathBuf::from("~/.md-translator/cache"));
    let cache_root = remote.expand(&configured_root)?;
    ensure!(
        cache_root != Path::new("/") && cache_root != remote.home,
        "远端缓存目录不能是根目录或整个用户目录"
    );
    let requested = remote.expand(&options.file.path)?;
    let mut source_path = remote.canonicalize(&requested).await?;
    // Preserve the existing CLI behavior when a user explicitly passes a generated preview.
    let canonical_cache = remote
        .canonicalize(&cache_root)
        .await
        .unwrap_or_else(|_| cache_root.clone());
    if source_path.starts_with(canonical_cache.join("views")) {
        let record = remote
            .read(
                &source_path
                    .parent()
                    .context("预览缺少目录")?
                    .join("source.json"),
            )
            .await?;
        let record: serde_json::Value = serde_json::from_slice(&record)?;
        let original = PathBuf::from(
            record["path"]
                .as_str()
                .context("预览缺少原文路径，请从原文发起翻译")?,
        );
        ensure!(
            original.is_absolute() && !original.starts_with(&canonical_cache),
            "预览中的原文路径无效"
        );
        source_path = remote.canonicalize(&original).await?;
    }
    ensure!(
        source_path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| ["md", "markdown"].contains(&s.to_ascii_lowercase().as_str())),
        "仅支持 .md / .markdown 文件"
    );
    let original = String::from_utf8(remote.read(&source_path).await?)
        .context("远端 Markdown 不是 UTF-8 文本")?;
    let origin = options.file.uri_for(&source_path)?;
    remote.private_dir(&cache_root).await?;
    let cache_root = remote.canonicalize(&cache_root).await?;
    let paths = CachePaths::new(&cache_root, &source_path, &original, provider);
    let uri = options.file.uri_for(&paths.view)?;
    let heading = header_with_url(&options.file.source_link(&source_path)?, &provider.model);
    let document_id = hash(serde_json::to_vec(&json!([
        origin,
        cache_root,
        provider.fingerprint()
    ]))?);
    let pending_path = options
        .state_dir
        .join("pending")
        .join(&document_id)
        .join(format!("{}.json", paths.key));
    let lock_path = options
        .state_dir
        .join("locks")
        .join(format!("{document_id}.lock"));
    let Some(_lock) = DocumentLock::acquire(&lock_path)? else {
        eprintln!("同一远端文档正在本机翻译，没有重复发出请求。");
        if remote.read_optional(&paths.view).await?.is_some() {
            show(&uri, binary).await?;
        }
        return Ok(RemoteOutcome {
            status: "busy",
            uri,
            requests: 0,
        });
    };
    let cached = cached(remote, &paths).await?;
    remote
        .atomic_write(
            &paths.source_record,
            &serde_json::to_vec(&json!({"path":source_path,"uri":origin}))?,
        )
        .await?;
    if let Some(pending) = load_pending(&pending_path, &origin, &paths.key)? {
        eprintln!("找到已完成但尚未发布的译文，正在继续写回远端，未请求模型。");
        publish(remote, &paths, &heading, &pending).await?;
        remove_pending(&pending_path)?;
        show(&uri, binary).await?;
        return Ok(RemoteOutcome {
            status: "uploaded",
            uri,
            requests: 0,
        });
    }
    if !options.force
        && let Some((markdown, warnings)) = &cached
    {
        remote
            .atomic_write(&paths.view, render(&heading, markdown, warnings).as_bytes())
            .await?;
        eprintln!("已打开远端完整缓存，未请求模型，未读取 API Key。");
        show(&uri, binary).await?;
        return Ok(RemoteOutcome {
            status: "cached",
            uri,
            requests: 0,
        });
    }
    let mut preview_opened = false;
    let run = async {
        let key = if original.trim().is_empty() {
            String::new()
        } else {
            credential(provider, &credentials_path(&options.preferences))?.key
        };
        remote
            .atomic_write(
                &paths.view,
                format!("{heading}> 正在翻译全文…\n").as_bytes(),
            )
            .await?;
        // Preview activation is optional; a failure must not discard a completed model result.
        if let Some(binary) = binary {
            match open_remote_in_zed(&uri, binary).await {
                Ok(()) => preview_opened = true,
                Err(error) => eprintln!("暂时无法打开等待预览：{error}；完成后会再尝试打开。"),
            }
        }
        let translated = translate_whole(provider, &key, &original, timeout, cancel, &|message| {
            eprintln!("{message}")
        })
        .await?;
        let pending = Pending {
            origin: origin.clone(),
            key: paths.key.clone(),
            result_hash: hash(&translated.markdown),
            markdown: translated.markdown,
            warnings: translated.warnings,
        };
        // Keep the already-paid result until all remote writes have succeeded, including on cancellation.
        atomic_write(&pending_path, serde_json::to_vec(&pending)?)?;
        ensure!(
            !cancel.is_cancelled(),
            "翻译已取消，已完成结果暂存在本机，后续可继续上传。"
        );
        publish(remote, &paths, &heading, &pending)
            .await
            .with_context(|| {
                format!(
                    "译文已完成但远端写回失败；结果暂存在 {}，再次运行相同命令会继续上传。",
                    pending_path.display()
                )
            })?;
        remove_pending(&pending_path)?;
        Ok::<_, anyhow::Error>(translated.requests)
    }
    .await;
    match run {
        Ok(requests) => {
            eprintln!("整篇翻译完成，结果已写入远端缓存（{requests} 次请求）。");
            if !preview_opened {
                show(&uri, binary).await?;
            }
            Ok(RemoteOutcome {
                status: "translated",
                uri,
                requests,
            })
        }
        Err(error) => {
            let detail = format!(
                "{heading}> **翻译未完成**：{}\n\n",
                escape(&format!("{error:#}"))
            );
            let content = if let Some((markdown, warnings)) = cached {
                render(
                    &format!("{detail}> 以下保留上次成功译文。\n\n"),
                    &markdown,
                    &warnings,
                )
            } else {
                detail
            };
            if cancel.is_cancelled() {
                remote
                    .finish_cancelled_view(&paths.view, content.as_bytes())
                    .await;
            } else {
                let _ = remote.atomic_write(&paths.view, content.as_bytes()).await;
            }
            Err(error)
        }
    }
}
