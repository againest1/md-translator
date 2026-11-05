use anyhow::{Context, Result, bail, ensure};
use openssh_sftp_client::{
    Sftp, SftpOptions,
    error::{Error as SftpError, SftpErrorKind},
};
use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use std::{
    future::Future,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::{Child, Command};
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteFile {
    pub host: String,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Local(PathBuf),
    Remote(RemoteFile),
}

impl Source {
    pub fn parse(input: &str) -> Result<Self> {
        ensure!(
            !input.is_empty() && !input.contains(['\0', '\n', '\r']),
            "文件地址不能为空或包含控制字符"
        );
        if input.starts_with("ssh://") {
            let uri = Url::parse(input).context("SSH 文件地址无效")?;
            ensure!(
                uri.password().is_none() && uri.query().is_none() && uri.fragment().is_none(),
                "SSH 地址不能包含密码、查询参数或片段；路径中的特殊字符请使用 URL 编码"
            );
            let host = uri
                .host_str()
                .context("SSH 地址缺少主机")?
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string();
            let user = if uri.username().is_empty() {
                None
            } else {
                Some(
                    percent_decode_str(uri.username())
                        .decode_utf8()
                        .context("SSH 用户名不是 UTF-8")?
                        .into_owned(),
                )
            };
            let path = percent_decode_str(uri.path())
                .decode_utf8()
                .context("SSH 文件路径不是 UTF-8")?
                .into_owned();
            ensure!(path != "/" && !path.is_empty(), "SSH 地址缺少文件路径");
            return Ok(Self::Remote(RemoteFile::new(
                host,
                user,
                uri.port(),
                path.into(),
            )?));
        }
        if input.starts_with(['/', '.', '~'])
            || (input.as_bytes().get(1) == Some(&b':')
                && input.as_bytes()[0].is_ascii_alphabetic()
                && (input.as_bytes().get(2) == Some(&b'\\') || cfg!(windows)))
        {
            return Ok(Self::Local(input.into()));
        }
        ensure!(
            !input.contains("://"),
            "远端文件地址只支持 ssh:// 或 [user@]host:path"
        );
        // Colons after a directory separator belong to a local filename.
        let split = if let Some(bracket) = input.find('[') {
            input[bracket..].find("]:").map(|i| bracket + i + 1)
        } else {
            input.find(':')
        };
        if let Some(index) = split.filter(|i| !input[..*i].contains(['/', '\\'])) {
            let authority = &input[..index];
            let path = &input[index + 1..];
            let (user, host) = match authority.rsplit_once('@') {
                Some((user, host)) => (Some(user.to_owned()), host),
                None => (None, authority),
            };
            let host = host
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned();
            return Ok(Self::Remote(RemoteFile::new(
                host,
                user,
                None,
                path.into(),
            )?));
        }
        Ok(Self::Local(input.into()))
    }
}

impl RemoteFile {
    fn new(host: String, user: Option<String>, port: Option<u16>, path: PathBuf) -> Result<Self> {
        ensure!(
            !host.is_empty()
                && !host.starts_with('-')
                && host
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "._-:".contains(c)),
            "SSH 主机或别名无效"
        );
        if host.contains(':') {
            host.parse::<std::net::Ipv6Addr>()
                .context("IPv6 地址无效，请使用方括号包围地址")?;
        }
        if let Some(user) = &user {
            ensure!(
                !user.is_empty()
                    && !user.chars().any(|c| c.is_control() || c.is_whitespace())
                    && !user.starts_with('-'),
                "SSH 用户名无效"
            );
        }
        ensure!(port != Some(0), "SSH 端口必须大于零");
        ensure!(
            !path.as_os_str().is_empty() && !path.to_string_lossy().chars().any(char::is_control),
            "远端文件路径不能为空或包含控制字符"
        );
        Ok(Self {
            host,
            user,
            port,
            path,
        })
    }

    pub fn uri_for(&self, path: &Path) -> Result<String> {
        ensure!(path.is_absolute(), "打开远端文件需要绝对路径");
        let mut uri = Url::parse("ssh://placeholder/")?;
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        uri.set_host(Some(&host))
            .map_err(|_| anyhow::anyhow!("SSH 主机无效"))?;
        let user = percent_encoding::utf8_percent_encode(
            self.user.as_deref().unwrap_or(""),
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        uri.set_username(&user)
            .map_err(|_| anyhow::anyhow!("SSH 用户名无效"))?;
        uri.set_port(self.port)
            .map_err(|_| anyhow::anyhow!("SSH 端口无效"))?;
        uri.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("SSH 路径无法编码"))?
            .clear()
            .extend(
                path.to_str()
                    .context("远端路径必须是 UTF-8")?
                    .split('/')
                    .skip(1),
            );
        Ok(uri.to_string())
    }

    pub fn source_link(&self, path: &Path) -> Result<String> {
        Ok(self.uri_for(path)?.replacen("ssh://", "zed://ssh/", 1))
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SshSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_file: Option<PathBuf>,
    // Retained only to locate legacy remote previews; new caches are local.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connect_timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_timeout_seconds: Option<u64>,
}

pub struct RemoteFs {
    sftp: Option<Sftp>,
    child: Child,
    timeout: Duration,
    cancel: CancellationToken,
    pub home: PathBuf,
}

fn missing(error: &SftpError) -> bool {
    matches!(error, SftpError::SftpError(SftpErrorKind::NoSuchFile, _))
}

pub async fn bounded<T>(
    cancel: &CancellationToken,
    timeout: Duration,
    operation: &str,
    future: impl Future<Output = Result<T>>,
) -> Result<T> {
    tokio::select! { biased;
        _ = cancel.cancelled() => bail!("SSH 操作已取消"),
        result = tokio::time::timeout(timeout, future) => result.with_context(|| format!("{operation}超时（{} 秒）", timeout.as_secs()))?,
    }
}

impl RemoteFs {
    pub async fn connect(
        file: &RemoteFile,
        settings: &SshSettings,
        cancel: &CancellationToken,
    ) -> Result<Self> {
        let connect = settings.connect_timeout_seconds.unwrap_or(15);
        let operation = settings.operation_timeout_seconds.unwrap_or(30);
        ensure!(
            (1..=3600).contains(&connect) && (1..=3600).contains(&operation),
            "SSH 超时必须在 1 到 3600 秒之间"
        );
        let binary = settings.binary.clone().unwrap_or_else(|| {
            if cfg!(windows) {
                "C:/Windows/System32/OpenSSH/ssh.exe".into()
            } else {
                "/usr/bin/ssh".into()
            }
        });
        ensure!(binary.is_absolute(), "ssh.binary 必须是绝对路径");
        let mut command = Command::new(binary);
        command
            .args([
                "-T",
                "-o",
                "BatchMode=yes",
                "-o",
                "ClearAllForwardings=yes",
                "-o",
                "ServerAliveInterval=15",
                "-o",
                "ServerAliveCountMax=3",
            ])
            .arg("-o")
            .arg(format!("ConnectTimeout={connect}"));
        if let Some(path) = &settings.config_file {
            ensure!(path.is_absolute(), "ssh.config_file 必须是绝对路径");
            command.arg("-F").arg(path);
        }
        if let Some(path) = &settings.identity_file {
            ensure!(path.is_absolute(), "ssh.identity_file 必须是绝对路径");
            command.arg("-i").arg(path);
        }
        if let Some(user) = &file.user {
            command.arg("-l").arg(user);
        }
        if let Some(port) = file.port {
            command.arg("-p").arg(port.to_string());
        }
        command
            .arg("-s")
            .arg("--")
            .arg(&file.host)
            .arg("sftp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .context("无法启动系统 SSH 客户端，请检查 ssh.binary")?;
        let input = child.stdin.take().context("无法打开 SSH 输入")?;
        let output = child.stdout.take().context("无法打开 SSH 输出")?;
        let sftp = bounded(
            cancel,
            Duration::from_secs(connect + 5),
            "建立 SFTP 连接",
            async {
                Ok(Sftp::new(
                    input,
                    output,
                    SftpOptions::new().flush_interval(Duration::ZERO),
                )
                .await?)
            },
        )
        .await
        .context("无法建立 SFTP 连接。请先确认本机 ssh 能登录目标，且服务器已启用 SFTP。")?;
        let mut remote = Self {
            sftp: Some(sftp),
            child,
            timeout: Duration::from_secs(operation),
            cancel: cancel.clone(),
            home: PathBuf::new(),
        };
        remote.home = remote.canonicalize(Path::new(".")).await?;
        Ok(remote)
    }

    fn client(&self) -> &Sftp {
        self.sftp.as_ref().expect("SFTP session is open")
    }

    pub fn expand(&self, path: &Path) -> Result<PathBuf> {
        let text = path.to_str().context("远端路径必须是 UTF-8")?;
        let path = if text == "~" {
            self.home.clone()
        } else if let Some(relative) = text.strip_prefix("~/") {
            self.home.join(relative)
        } else {
            ensure!(!text.starts_with('~'), "仅支持当前 SSH 用户的 ~/ 路径");
            if path.is_absolute() {
                path.to_owned()
            } else {
                self.home.join(path)
            }
        };
        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    ensure!(clean.pop(), "路径越过远端根目录");
                }
                _ => clean.push(component),
            }
        }
        ensure!(clean.is_absolute(), "远端路径必须解析为绝对路径");
        Ok(clean)
    }

    pub async fn canonicalize(&self, path: &Path) -> Result<PathBuf> {
        let mut fs = self.client().fs();
        bounded(&self.cancel, self.timeout, "解析远端路径", async {
            Ok(fs.canonicalize(path).await?)
        })
        .await
    }

    pub async fn read_optional(&self, path: &Path) -> Result<Option<Vec<u8>>> {
        let mut fs = self.client().fs();
        bounded(&self.cancel, self.timeout, "读取远端文件", async {
            match fs.read(path).await {
                Ok(bytes) => Ok(Some(bytes.to_vec())),
                Err(error) if missing(&error) => Ok(None),
                Err(error) => Err(error.into()),
            }
        })
        .await
        .with_context(|| format!("读取远端文件失败：{}", path.display()))
    }

    pub async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        self.read_optional(path)
            .await?
            .with_context(|| format!("远端文件不存在：{}", path.display()))
    }

    pub async fn close(mut self) -> Result<()> {
        if let Some(client) = self.sftp.take() {
            bounded(&self.cancel, self.timeout, "关闭 SFTP 会话", async {
                Ok(client.close().await?)
            })
            .await?;
        }
        let status = tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await??;
        ensure!(status.success(), "SSH 进程异常退出：{status}");
        Ok(())
    }
}

pub async fn read_document(
    mut file: RemoteFile,
    settings: &SshSettings,
    cancel: &CancellationToken,
) -> Result<(RemoteFile, String)> {
    let remote = RemoteFs::connect(&file, settings, cancel).await?;
    let result = async {
        file.path = remote.canonicalize(&remote.expand(&file.path)?).await?;
        // Old remote preview URLs still resolve back to their source; new caches are local.
        let old_cache = remote.expand(
            settings
                .cache_dir
                .as_deref()
                .unwrap_or_else(|| Path::new("~/.md-translator/cache")),
        )?;
        if file.path.starts_with(old_cache.join("views")) {
            let record = remote
                .read(
                    &file
                        .path
                        .parent()
                        .context("旧预览缺少目录")?
                        .join("source.json"),
                )
                .await?;
            let record: serde_json::Value = serde_json::from_slice(&record)?;
            let original = Path::new(
                record["path"]
                    .as_str()
                    .context("旧预览缺少原文路径，请从原文发起翻译")?,
            );
            ensure!(
                original.is_absolute() && !original.starts_with(&old_cache),
                "旧预览中的原文路径无效"
            );
            file.path = remote.canonicalize(original).await?;
        }
        let content = String::from_utf8(remote.read(&file.path).await?)
            .context("远端 Markdown 不是 UTF-8 文本")?;
        Ok((file, content))
    }
    .await;
    if let Err(error) = remote.close().await
        && result.is_ok()
    {
        eprintln!("远端原文已读取，关闭 SSH 会话时收到提示：{error}");
    }
    result
}
