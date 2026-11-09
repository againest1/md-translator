use crate::config::{
    AuthMode, Preferences, REASONING_EFFORTS, clean_key, credential, credentials_path, endpoint,
};
use anyhow::{Result, bail, ensure};
use dialoguer::{
    Input, Select,
    console::{Key, Term},
};
use std::{io::IsTerminal, path::Path};

pub struct Draft {
    pub preferences: Preferences,
    pub key: Option<String>,
    pub test: bool,
}

#[cfg(unix)]
struct HiddenInput(libc::termios);

#[cfg(unix)]
impl HiddenInput {
    fn new() -> Result<Self> {
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        // SAFETY: stdin is a terminal and original is a writable termios allocation.
        ensure!(
            unsafe { libc::tcgetattr(libc::STDIN_FILENO, original.as_mut_ptr()) } == 0,
            "无法隐藏密钥输入，配置未保存。"
        );
        let original = unsafe { original.assume_init() };
        let mut hidden = original;
        hidden.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG);
        hidden.c_cc[libc::VMIN] = 1;
        hidden.c_cc[libc::VTIME] = 0;
        // Disable echo before displaying the prompt, including for immediate paste input.
        ensure!(
            unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &hidden) } == 0,
            "无法隐藏密钥输入，配置未保存。"
        );
        Ok(Self(original))
    }
}

#[cfg(unix)]
impl Drop for HiddenInput {
    fn drop(&mut self) {
        // SAFETY: this restores the termios value read from the same terminal.
        unsafe {
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.0);
        }
    }
}

fn read_secret(prompt: &str) -> Result<String> {
    #[cfg(unix)]
    let _hidden = HiddenInput::new()?;
    let term = Term::stderr();
    term.write_str(&format!("{prompt}: "))?;
    let mut value = String::new();
    loop {
        match term.read_key()? {
            Key::Enter => {
                term.write_line("")?;
                return Ok(value);
            }
            Key::Escape | Key::CtrlC => {
                term.write_line("")?;
                bail!("已取消，配置未修改。");
            }
            Key::Backspace => {
                value.pop();
            }
            Key::Char(c) if !c.is_control() => value.push(c),
            _ => {}
        }
    }
}

pub fn prompt_key(config: &Path, preferences: &Preferences) -> Result<Option<String>> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        "密钥需要在交互终端中隐藏输入，请在 Zed 运行「Markdown: 配置翻译服务」。"
    );
    let provider = preferences.provider()?;
    let existing = credential(&provider, &credentials_path(config)).is_ok();
    loop {
        let key = read_secret(if existing {
            "API Key（已设置，留空保留）"
        } else {
            "API Key（隐藏输入）"
        })?;
        if existing && key.is_empty() {
            return Ok(None);
        }
        match clean_key(&key) {
            Ok(key) => return Ok(Some(key)),
            Err(error) => eprintln!("{error}"),
        }
    }
}

pub fn configure(config: &Path) -> Result<Option<Draft>> {
    ensure!(
        std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        "配置向导需要交互终端，请在 Zed Task 中选择「Markdown: 配置翻译服务」。"
    );
    let mut preferences = Preferences::load(config)?;
    let mut settings = preferences.provider.clone().unwrap_or_default();
    println!("Markdown 翻译服务配置\n配置文件：{}", config.display());
    println!(
        "密钥保存在独立凭据文件中（明文，仅当前用户可读写）；不读取 Zed provider 或环境变量。\n"
    );
    settings.base_url = Input::<String>::new()
        .with_prompt("接口地址（例如 https://example.com/v1）")
        .with_initial_text(&settings.base_url)
        .validate_with(|value: &String| endpoint(value).map(|_| ()).map_err(|e| e.to_string()))
        .interact_text()?
        .trim()
        .into();
    settings.model = Input::<String>::new()
        .with_prompt("模型名称")
        .with_initial_text(&settings.model)
        .validate_with(|value: &String| -> Result<(), &str> {
            if value.trim().is_empty() {
                Err("模型名称不能为空")
            } else {
                Ok(())
            }
        })
        .interact_text()?
        .trim()
        .into();
    let Some(effort) = Select::new()
        .with_prompt(
            "推理强度（effort；default 由服务决定，none 关闭推理，级别越高通常耗时和用量越多）",
        )
        .items(REASONING_EFFORTS)
        .default(
            REASONING_EFFORTS
                .iter()
                .position(|effort| Some(*effort) == settings.reasoning_effort.as_deref())
                .unwrap_or(1), // low
        )
        .interact_opt()?
    else {
        return Ok(None);
    };
    settings.reasoning_effort = Some(REASONING_EFFORTS[effort].into());
    let Some(auth) = Select::new()
        .with_prompt("鉴权方式")
        .items(["保存 API Key", "无需鉴权"])
        .default(usize::from(settings.auth == AuthMode::None))
        .interact_opt()?
    else {
        return Ok(None);
    };
    settings.auth = if auth == 0 {
        AuthMode::ApiKey
    } else {
        AuthMode::None
    };
    preferences.provider = Some(settings);
    let key = if auth == 0 {
        prompt_key(config, &preferences)?
    } else {
        None
    };
    let seconds = Input::<u64>::new()
        .with_prompt("请求超时（秒）")
        .default(preferences.request_timeout_seconds.unwrap_or(120))
        .validate_with(|value: &u64| -> Result<(), &str> {
            if (1..=3600).contains(value) {
                Ok(())
            } else {
                Err("请输入 1 到 3600")
            }
        })
        .interact_text()?;
    preferences.request_timeout_seconds = Some(seconds);
    let provider = preferences.provider()?;
    println!(
        "\n请求地址：{}\n模型：{}\n推理强度（effort）：{}",
        provider.endpoint, provider.model, REASONING_EFFORTS[effort]
    );
    let choice = Select::new()
        .with_prompt("完成配置")
        .items(["保存", "保存并测试（发送一次固定短文本请求）", "取消"])
        .default(0)
        .interact_opt()?;
    match choice {
        Some(0 | 1) => Ok(Some(Draft {
            preferences,
            key,
            test: choice == Some(1),
        })),
        _ => Ok(None),
    }
}
