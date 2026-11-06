use crate::{
    cache::{CachePaths, source_from_cache},
    client::translate_whole,
    config::{Preferences, credential, credentials_path},
    markdown::{preview_markdown, remote_preview_markdown},
    storage::{atomic_write, zed_cli},
    zed::open_in_zed,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;
use url::Url;

pub struct TranslateOptions {
    pub file: PathBuf,
    pub project_root: Option<PathBuf>,
    pub cache_dir: PathBuf,
    pub preferences: PathBuf,
    pub zed_cli: Option<PathBuf>,
    pub force: bool,
    pub no_open: bool,
    pub remote: bool,
}

pub struct Outcome {
    pub status: &'static str,
    pub path: PathBuf,
    pub requests: usize,
}

pub(crate) fn escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| {
            if "\\`*[]<>|".contains(c) {
                vec!['\\', c]
            } else if c == '\n' || c == '\r' {
                vec![' ']
            } else {
                vec![c]
            }
        })
        .collect()
}

fn header(source: &Path, model: &str, remote: bool) -> Result<String> {
    let url = Url::from_file_path(source).map_err(|_| anyhow::anyhow!("无法构造原文路径 URL"))?;
    // Plain paths are opened within the preview's remote workspace.
    Ok(header_with_url(
        if remote { url.path() } else { url.as_str() },
        model,
    ))
}

fn terminal_preview(path: &Path, remote: bool) -> Result<()> {
    if remote {
        let url = Url::from_file_path(path).map_err(|_| anyhow::anyhow!("无法构造译文链接"))?;
        let url = url
            .as_str()
            .replace('\'', "%27")
            .replace('(', "%28")
            .replace(')', "%29");
        eprintln!(
            "在当前 Zed 任务终端 Cmd+Click（macOS）或 Ctrl+Click（Linux/Windows）打开译文：\n{url}"
        );
    }
    Ok(())
}

pub(crate) fn header_with_url(url: &str, model: &str) -> String {
    format!(
        "> **简体中文译文** · {} · [返回原文]({})\n\n---\n\n",
        escape(model),
        url.replace('(', "%28").replace(')', "%29")
    )
}

fn show_result(
    paths: &CachePaths,
    source: &Path,
    heading: &str,
    markdown: &str,
    warnings: &[String],
    remote: bool,
) -> Result<()> {
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
    let preview = if remote {
        remote_preview_markdown(markdown, source)?
    } else {
        preview_markdown(markdown, source)?
    };
    atomic_write(&paths.view, format!("{heading}{notices}{preview}"))
}

pub async fn translate_file(
    options: &TranslateOptions,
    cancel: &CancellationToken,
) -> Result<Outcome> {
    let file = source_from_cache(&options.file, &options.cache_dir)?;
    let preferences = Preferences::load(&options.preferences)?;
    let source = fs::read_to_string(&file).context("无法读取 UTF-8 Markdown 原文")?;
    ensure!(
        file.extension()
            .and_then(|s| s.to_str())
            .is_some_and(|s| ["md", "markdown"].contains(&s.to_ascii_lowercase().as_str())),
        "仅支持 .md / .markdown 文件"
    );
    let project_dir = match &options.project_root {
        Some(root) => {
            let root = root
                .canonicalize()
                .context("无法读取 --project-root 目录")?;
            ensure!(root.is_dir(), "--project-root 必须是目录");
            Some(root)
        }
        None => Some(file.parent().context("原文路径缺少父目录")?.to_path_buf()),
    };
    let provider = preferences.provider()?;
    let no_open = options.no_open || options.remote;
    let zed_binary = if no_open {
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
    let timeout = preferences.timeout()?;
    let cache_dir = std::path::absolute(&options.cache_dir)?;
    let paths = CachePaths::new(&cache_dir, &file, &source, &provider);
    let heading = header(&file, &provider.model, options.remote)?;
    paths.record_source(&file)?;
    let cached = paths.read();
    if !options.force
        && let Some((markdown, warnings)) = &cached
    {
        show_result(&paths, &file, &heading, markdown, warnings, options.remote)?;
        terminal_preview(&paths.view, options.remote)?;
        if !no_open {
            open_in_zed(
                project_dir.as_deref(),
                &paths.view,
                zed_binary.as_ref().unwrap(),
            )
            .await?;
        }
        eprintln!("完整缓存译文已就绪，未请求模型。");
        return Ok(Outcome {
            status: "cached",
            path: paths.view,
            requests: 0,
        });
    }
    let Some(_lock) = paths.acquire()? else {
        eprintln!("同一份原文正在翻译，没有重复发出请求。");
        if !paths.view.exists() {
            atomic_write(
                &paths.view,
                format!("{heading}> 同一份文档正在翻译，请等待当前任务。\n"),
            )?;
        }
        terminal_preview(&paths.view, options.remote)?;
        if !no_open {
            open_in_zed(
                project_dir.as_deref(),
                &paths.view,
                zed_binary.as_ref().unwrap(),
            )
            .await?;
        }
        return Ok(Outcome {
            status: "busy",
            path: paths.view,
            requests: 0,
        });
    };
    // The first worker may have completed between the initial cache lookup and acquiring the lock.
    if !options.force
        && let Some((markdown, warnings)) = paths.read()
    {
        show_result(
            &paths,
            &file,
            &heading,
            &markdown,
            &warnings,
            options.remote,
        )?;
        terminal_preview(&paths.view, options.remote)?;
        if !no_open {
            open_in_zed(
                project_dir.as_deref(),
                &paths.view,
                zed_binary.as_ref().unwrap(),
            )
            .await?;
        }
        return Ok(Outcome {
            status: "cached",
            path: paths.view,
            requests: 0,
        });
    }
    let execute = async {
        ensure!(!cancel.is_cancelled(), "翻译已取消");
        let key = if source.trim().is_empty() {
            String::new()
        } else {
            let credentials = credential(&provider, &credentials_path(&options.preferences))?;
            eprintln!(
                "Provider：{} / {}（{}）",
                provider.id, provider.model, credentials.source
            );
            credentials.key
        };
        atomic_write(&paths.view, format!("{heading}> 正在翻译全文…\n"))?;
        terminal_preview(&paths.view, options.remote)?;
        if !no_open {
            open_in_zed(
                project_dir.as_deref(),
                &paths.view,
                zed_binary.as_ref().unwrap(),
            )
            .await?;
        }
        let progress = |message: &str| {
            eprintln!("{message}");
            if let Err(error) =
                atomic_write(&paths.view, format!("{heading}> {}\n", escape(message)))
            {
                eprintln!("无法更新预览状态：{error}");
            }
        };
        let translated =
            translate_whole(&provider, &key, &source, timeout, cancel, &progress).await?;
        ensure!(!cancel.is_cancelled(), "翻译已取消");
        paths.save(&translated.markdown, &translated.warnings)?;
        show_result(
            &paths,
            &file,
            &heading,
            &translated.markdown,
            &translated.warnings,
            options.remote,
        )?;
        Ok::<_, anyhow::Error>(translated.requests)
    };
    match execute.await {
        Ok(requests) => {
            eprintln!("整篇翻译完成，成功结果已缓存（{requests} 次请求）。");
            terminal_preview(&paths.view, options.remote)?;
            Ok(Outcome {
                status: "translated",
                path: paths.view,
                requests,
            })
        }
        Err(error) => {
            let status = if cancel.is_cancelled() {
                "翻译已取消"
            } else {
                "翻译失败"
            };
            let detail = format!("> **{status}**：{}\n\n", escape(&format!("{error:#}")));
            if let Some((markdown, warnings)) = cached {
                let _ = show_result(
                    &paths,
                    &file,
                    &format!("{heading}{detail}> 以下保留上次成功译文。\n\n"),
                    &markdown,
                    &warnings,
                    options.remote,
                );
            } else {
                let _ = atomic_write(
                    &paths.view,
                    format!("{heading}{detail}请检查错误后从原文重新发起翻译。\n"),
                );
            }
            if paths.view.is_file() {
                let _ = terminal_preview(&paths.view, options.remote);
            }
            Err(error)
        }
    }
}
