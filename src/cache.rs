use crate::{
    client::{PIPELINE_VERSION, SYSTEM_PROMPT},
    config::Provider,
    remote::{RemoteFile, Source},
    storage::{atomic_write, hash, private_dir},
};
use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub struct CachePaths {
    pub key: String,
    pub entry: PathBuf,
    pub view: PathBuf,
    pub source_record: PathBuf,
}

#[derive(Serialize, Deserialize)]
pub struct Cached {
    pub key: String,
    pub result_hash: String,
    pub warnings: Vec<String>,
}

pub struct DocumentLock {
    _file: File,
}

impl DocumentLock {
    pub fn acquire(path: &Path) -> Result<Option<Self>> {
        private_dir(path.parent().context("锁文件缺少父目录")?)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { _file: file })),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for DocumentLock {
    fn drop(&mut self) {
        // Release explicitly: a concurrently spawned child may briefly inherit the descriptor.
        let _ = FileExt::unlock(&self._file);
    }
}

impl CachePaths {
    pub fn new(root: &Path, source_path: &Path, source: &str, provider: &Provider) -> Self {
        Self::with_identity(root, source_path, &json!(source_path), source, provider)
    }

    pub fn remote(
        root: &Path,
        file: &RemoteFile,
        source: &str,
        provider: &Provider,
    ) -> Result<Self> {
        Ok(Self::with_identity(
            root,
            &file.path,
            &json!(file.uri_for(&file.path)?),
            source,
            provider,
        ))
    }

    fn with_identity(
        root: &Path,
        source_path: &Path,
        identity: &serde_json::Value,
        source: &str,
        provider: &Provider,
    ) -> Self {
        let key = hash(
            serde_json::to_vec(
                &json!({"source":hash(source),"provider":provider.fingerprint(),
            "pipeline":PIPELINE_VERSION,"prompt":hash(SYSTEM_PROMPT),"target":"zh-CN"}),
            )
            .unwrap(),
        );
        let view_key =
            hash(serde_json::to_vec(&json!([identity, provider.fingerprint()])).unwrap());
        let view_dir = root.join("views").join(view_key);
        let stem = source_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        Self {
            entry: root.join("translations").join(&key),
            key,
            view: view_dir.join(format!("{stem} · 简体中文.md")),
            source_record: view_dir.join("source.json"),
        }
    }

    pub fn acquire(&self) -> Result<Option<DocumentLock>> {
        let directory = self.view.parent().context("预览文件缺少目录")?;
        DocumentLock::acquire(&directory.join("request.lock"))
    }

    pub fn read(&self) -> Option<(String, Vec<String>)> {
        let metadata: Cached =
            serde_json::from_slice(&fs::read(self.entry.join("manifest.json")).ok()?).ok()?;
        let markdown = fs::read_to_string(
            self.entry
                .join(format!("result-{}.md", metadata.result_hash)),
        )
        .ok()?;
        (metadata.key == self.key && hash(&markdown) == metadata.result_hash)
            .then_some((markdown, metadata.warnings))
    }

    pub fn save(&self, markdown: &str, warnings: &[String]) -> Result<()> {
        // Generation files keep the previous completed result valid if a forced refresh fails mid-write.
        let digest = hash(markdown);
        let generation = self.entry.join(format!("result-{digest}.md"));
        atomic_write(&generation, markdown)?;
        atomic_write(
            &self.entry.join("manifest.json"),
            serde_json::to_vec_pretty(&Cached {
                key: self.key.clone(),
                result_hash: digest,
                warnings: warnings.to_vec(),
            })?,
        )?;
        Ok(())
    }

    pub fn record_source(&self, source: &Source) -> Result<()> {
        let record = match source {
            Source::Local(path) => json!({"path":path}),
            Source::Remote(file) => json!({"uri":file.uri_for(&file.path)?}),
        };
        atomic_write(&self.source_record, serde_json::to_vec(&record)?)
    }
}

pub fn source_from_cache(path: &Path, root: &Path) -> Result<Source> {
    let path = fs::canonicalize(path).with_context(|| format!("无法打开 {}", path.display()))?;
    let root = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    if path.starts_with(root.join("views")) {
        let source: serde_json::Value =
            serde_json::from_slice(&fs::read(path.parent().unwrap().join("source.json"))?)?;
        if let Some(uri) = source["uri"].as_str() {
            let original = Source::parse(uri)?;
            anyhow::ensure!(
                matches!(original, Source::Remote(_)),
                "缓存中的 SSH 原文地址无效"
            );
            return Ok(original);
        }
        let original = PathBuf::from(
            source["path"]
                .as_str()
                .context("缓存缺少原文路径，请从原文发起翻译")?,
        );
        anyhow::ensure!(!original.starts_with(&root), "缓存原文路径无效");
        return Ok(Source::Local(fs::canonicalize(original)?));
    }
    Ok(Source::Local(path))
}
