use crate::storage::{atomic_write, hash, read_jsonc};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

#[derive(Clone)]
pub struct Provider {
    pub id: String,
    pub model: String,
    pub base_url: String,
    pub endpoint: String,
    pub headers: BTreeMap<String, String>,
    pub request_options: BTreeMap<String, Value>,
    pub auth: AuthMode,
}

impl Provider {
    pub fn auth_id(&self) -> String {
        hash(format!("{}\n{}", self.id, self.endpoint))
    }
    pub fn fingerprint(&self) -> Value {
        json!({"id": self.id, "endpoint": self.endpoint, "model": self.model,
            "options": self.request_options, "headers_hash": hash(serde_json::to_vec(&self.headers).unwrap())})
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    ApiKey,
    None,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderSettings {
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub auth: AuthMode,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub custom_headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u64>,
}

impl ProviderSettings {
    pub fn resolve(&self) -> Result<Provider> {
        ensure!(
            !self.model.trim().is_empty(),
            "缺少模型名称，请运行 configure 配置翻译服务。"
        );
        ensure!(
            !self.base_url.trim().is_empty(),
            "缺少接口地址，请运行 configure 配置翻译服务。"
        );
        ensure!(
            self.max_tokens.is_none() || self.max_completion_tokens.is_none(),
            "max_tokens 和 max_completion_tokens 只能配置一项"
        );
        let mut options = BTreeMap::new();
        if let Some(effort) = &self.reasoning_effort {
            ensure!(!effort.trim().is_empty(), "reasoning_effort 不能为空");
            if effort != "default" {
                options.insert("reasoning_effort".into(), json!(effort));
            }
        }
        for (field, value) in [
            ("max_tokens", self.max_tokens),
            ("max_completion_tokens", self.max_completion_tokens),
        ] {
            if let Some(limit) = value {
                ensure!(limit > 0, "{field} 必须是正整数");
                options.insert(field.into(), json!(limit));
            }
        }
        for name in self.custom_headers.keys() {
            ensure!(
                !["authorization", "content-type"].contains(&name.to_ascii_lowercase().as_str()),
                "请通过鉴权配置设置密钥，不要覆盖 Authorization 或 Content-Type"
            );
        }
        Ok(Provider {
            id: if self.name.trim().is_empty() {
                "default".into()
            } else {
                self.name.trim().into()
            },
            model: self.model.trim().into(),
            base_url: self.base_url.trim().into(),
            endpoint: endpoint(&self.base_url)?,
            headers: self.custom_headers.clone(),
            request_options: options,
            auth: self.auth,
        })
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub provider: Option<ProviderSettings>,
    pub request_timeout_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zed_cli: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssh: Option<crate::remote::SshSettings>,
}

impl Preferences {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        // Do not interpolate environment variables or silently accept legacy auth settings.
        serde_json::from_value(read_jsonc(path)?)
            .context("配置格式错误；请使用独立 provider 配置，环境变量鉴权已移除。")
    }
    pub fn provider(&self) -> Result<Provider> {
        self.provider.as_ref().context("尚未配置翻译服务。请运行 configure，或在 Zed Task 中选择「Markdown: 配置翻译服务」。")?.resolve()
    }
    pub fn timeout(&self) -> Result<Duration> {
        let seconds = self.request_timeout_seconds.unwrap_or(120);
        ensure!(
            (1..=3600).contains(&seconds),
            "request_timeout_seconds 必须在 1 到 3600 之间"
        );
        Ok(Duration::from_secs(seconds))
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        self.provider()?;
        self.timeout()?;
        atomic_write(path, serde_json::to_vec_pretty(self)?)
    }
}

pub fn select_provider(config: &Value) -> Result<Provider> {
    serde_json::from_value::<Preferences>(config.clone())?.provider()
}

pub fn endpoint(base: &str) -> Result<String> {
    let mut url = Url::parse(base.trim()).context("Provider URL 无效")?;
    ensure!(
        ["http", "https"].contains(&url.scheme())
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Provider URL 必须是无账号密码、查询参数和片段的 HTTP(S) 地址"
    );
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/chat/completions") {
        path.to_owned()
    } else {
        format!("{path}/chat/completions")
    };
    url.set_path(&path);
    Ok(url.to_string())
}

pub struct Credential {
    pub key: String,
    pub source: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    keys: BTreeMap<String, String>,
}

pub fn credentials_path(config: &Path) -> PathBuf {
    config.with_file_name("credentials.json")
}

pub fn clean_key(key: &str) -> Result<String> {
    ensure!(
        !key.contains(['\r', '\n']),
        "API Key 包含换行，请重新填写。"
    );
    let key = key.trim();
    ensure!(
        !key.is_empty(),
        "API Key 不能为空；无需鉴权的服务请选择「无需鉴权」。"
    );
    Ok(key.into())
}

fn load_credentials(path: &Path) -> Result<Credentials> {
    if !path.exists() {
        return Ok(Credentials::default());
    }
    serde_json::from_value(read_jsonc(path)?).context("凭据文件格式错误，请重新配置 API Key。")
}

pub fn credential(provider: &Provider, path: &Path) -> Result<Credential> {
    if provider.auth == AuthMode::None {
        return Ok(Credential {
            key: String::new(),
            source: "无需鉴权".into(),
        });
    }
    let credentials = load_credentials(path)?;
    let key = credentials.keys.get(&provider.auth_id())
        .context("没有找到当前服务的独立凭据。请运行 configure 保存 API Key；不会读取环境变量或 Zed 钥匙串。")?;
    Ok(Credential {
        key: clean_key(key)?,
        source: "独立凭据文件".into(),
    })
}

pub fn save_key(provider: &Provider, path: &Path, key: &str) -> Result<()> {
    let mut credentials = load_credentials(path)?;
    credentials.keys.insert(provider.auth_id(), clean_key(key)?);
    atomic_write(path, serde_json::to_vec_pretty(&credentials)?)
}
