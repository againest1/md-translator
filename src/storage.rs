use anyhow::{Context, Result};
#[cfg(not(unix))]
use directories::BaseDirs;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

pub fn home_dir() -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::{
            ffi::{CStr, OsStr},
            os::unix::ffi::OsStrExt,
        };
        // Use the OS account record, not HOME/XDG overrides, to locate application data.
        let mut buffer = vec![0_u8; 4096];
        loop {
            let mut record = std::mem::MaybeUninit::<libc::passwd>::uninit();
            let mut result = std::ptr::null_mut();
            // SAFETY: all output pointers reference live, writable allocations for this call.
            let status = unsafe {
                libc::getpwuid_r(
                    libc::geteuid(),
                    record.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                )
            };
            if status == libc::ERANGE && buffer.len() < 1024 * 1024 {
                buffer.resize(buffer.len() * 2, 0);
                continue;
            }
            anyhow::ensure!(
                status == 0 && !result.is_null(),
                "无法从系统账户获取用户目录，请使用 --data-dir 和 --zed-config 指定路径。"
            );
            // SAFETY: a successful getpwuid_r initialized record and its null-terminated pw_dir.
            let record = unsafe { record.assume_init() };
            anyhow::ensure!(!record.pw_dir.is_null(), "系统账户缺少用户目录");
            let bytes = unsafe { CStr::from_ptr(record.pw_dir) }.to_bytes();
            return Ok(PathBuf::from(OsStr::from_bytes(bytes)));
        }
    }
    #[cfg(not(unix))]
    Ok(BaseDirs::new()
        .context("无法确定用户目录")?
        .home_dir()
        .to_path_buf())
}

pub fn data_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".md-translator"))
}

pub fn zed_config_dir() -> Result<PathBuf> {
    let home = home_dir()?;
    Ok(if cfg!(target_os = "windows") {
        home.join("AppData/Roaming/Zed")
    } else {
        home.join(".config/zed")
    })
}

pub fn zed_cli() -> Result<PathBuf> {
    let home = home_dir()?;
    let candidates = if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/Applications/Zed.app/Contents/MacOS/cli"),
            home.join("Applications/Zed.app/Contents/MacOS/cli"),
            home.join(".local/bin/zed"),
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            home.join("AppData/Local/Programs/Zed/zed.exe"),
            PathBuf::from("C:/Program Files/Zed/zed.exe"),
        ]
    } else {
        vec![
            home.join(".local/bin/zed"),
            PathBuf::from("/usr/local/bin/zed"),
            PathBuf::from("/usr/bin/zed"),
        ]
    };
    candidates
        .into_iter()
        .find(|p| p.is_file())
        .context("找不到 Zed CLI，请在 config.json 设置 zed_cli 的绝对路径，或使用 --zed-cli。")
}

pub fn parse_jsonc(text: &str) -> Result<Value> {
    let value = jsonc_parser::parse_to_serde_value::<Value>(text, &Default::default())?;
    anyhow::ensure!(!value.is_null(), "配置文件中缺少 JSON 值");
    Ok(value)
}

pub fn read_jsonc(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("无法读取 {}", path.display()))?;
    parse_jsonc(&text).with_context(|| format!("配置格式错误：{}", path.display()))
}

pub fn private_dir(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}

pub fn atomic_write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    let parent = path.parent().context("文件缺少父目录")?;
    private_dir(parent)?;
    let mut temporary = tempfile::Builder::new()
        .prefix(".md-translator-")
        .tempfile_in(parent)?;
    temporary.write_all(bytes.as_ref())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .with_context(|| format!("无法保存 {}", path.display()))?;
    Ok(())
}

pub fn redact(message: &str, secrets: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut safe = message.to_owned();
    for secret in secrets {
        if !secret.as_ref().is_empty() {
            safe = safe.replace(secret.as_ref(), "[REDACTED]");
        }
    }
    safe
}
