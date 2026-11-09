use crate::storage::{atomic_write, parse_jsonc};
use anyhow::{Context, Result, bail, ensure};
use jsonc_parser::cst::{CstInputValue, CstObject, CstRootNode};
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const TRANSLATE: &str = "Markdown: 翻译全文";
pub const RETRANSLATE: &str = "Markdown: 重新翻译全文";
pub const CONFIGURE: &str = "Markdown: 配置翻译服务";
pub const REMOTE_TRANSLATE: &str = "Markdown (Remote): 翻译全文";
pub const REMOTE_RETRANSLATE: &str = "Markdown (Remote): 重新翻译全文";
pub const REMOTE_CONFIGURE: &str = "Markdown (Remote): 配置翻译服务";
const CONTEXT: &str =
    "(Editor && mode == full && (extension == md || extension == markdown)) || MarkdownPreview";

// Resolve on the machine running the Task, never on the machine installing it.
#[cfg(not(windows))]
fn task_launcher() -> (&'static str, Value) {
    (
        r#"exec "$(command -v md-translator || printf '%s/.local/bin/md-translator' "$HOME")""#,
        json!({"program":"sh"}),
    )
}

#[cfg(windows)]
fn task_launcher() -> (&'static str, Value) {
    (
        r#"& { $ErrorActionPreference = 'Stop'; $tool = Get-Command md-translator -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1; if ($tool) { $binary = $tool.Source } else { $binary = Join-Path $env:LOCALAPPDATA 'Programs\md-translator\md-translator.exe' }; & $binary @args; exit $LASTEXITCODE }"#,
        json!({"with_arguments":{"program":"powershell","args":["-NoProfile"]}}),
    )
}

fn task_literal(value: &str) -> String {
    // Zed joins Task arguments as shell text. Quote literal paths before writing them.
    let escaped = if cfg!(windows) {
        value.replace('\'', "''")
    } else {
        value.replace('\'', "'\\''")
    };
    format!("'{escaped}'")
}

pub async fn open_in_zed(project_dir: Option<&Path>, path: &Path, binary: &Path) -> Result<()> {
    // Select the project first. Adding the preview in that request would make it a visible root.
    if let Some(project_dir) = project_dir {
        open_target(&[OsStr::new("--add"), project_dir.as_os_str()], binary).await?;
    }
    open_target(&[OsStr::new("--existing"), path.as_os_str()], binary).await
}

async fn open_target(args: &[&OsStr], binary: &Path) -> Result<()> {
    ensure!(binary.is_absolute(), "zed_cli 必须是绝对路径");
    let mut command = tokio::process::Command::new(binary);
    command.args(args).kill_on_drop(true);
    let result = tokio::time::timeout(Duration::from_secs(15), command.output())
        .await
        .context("打开 Zed 超时")?
        .context("无法启动 Zed CLI，请检查 config.json 的 zed_cli 或 --zed-cli。")?;
    ensure!(
        result.status.success(),
        "Zed CLI 打开文件失败，请检查 zed_cli 配置或 Zed 安装。"
    );
    Ok(())
}

fn input(value: &Value) -> CstInputValue {
    match value {
        Value::Null => CstInputValue::Null,
        Value::Bool(v) => CstInputValue::Bool(*v),
        Value::Number(v) => CstInputValue::Number(v.to_string()),
        Value::String(v) => CstInputValue::String(v.clone()),
        Value::Array(v) => CstInputValue::Array(v.iter().map(input).collect()),
        Value::Object(v) => {
            CstInputValue::Object(v.iter().map(|(k, v)| (k.clone(), input(v))).collect())
        }
    }
}

fn set(object: &CstObject, key: &str, value: &Value) {
    if let Some(property) = object.get(key) {
        property.set_value(input(value));
    } else {
        object.append(key, input(value));
    }
}

pub fn merge_tasks(text: &str, binary: &Path) -> Result<String> {
    merge_tasks_with_args(text, binary, &[], false)
}

fn merge_tasks_with_args(
    text: &str,
    binary: &Path,
    prefix: &[String],
    remote: bool,
) -> Result<String> {
    let root = CstRootNode::parse(text, &Default::default())?;
    let array = root.array_value().context("Zed tasks.json 必须是数组")?;
    let labels = if remote {
        [REMOTE_TRANSLATE, REMOTE_RETRANSLATE, REMOTE_CONFIGURE]
    } else {
        [TRANSLATE, RETRANSLATE, CONFIGURE]
    };
    for label in labels {
        let configuring = label == labels[2];
        let command = if remote {
            binary.to_string_lossy().into_owned()
        } else {
            task_launcher().0.to_owned()
        };
        let mut args: Vec<String> = prefix
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                if !remote && index % 2 == 1 {
                    task_literal(arg)
                } else {
                    arg.clone()
                }
            })
            .collect();
        args.extend(
            if configuring {
                vec!["configure"]
            } else {
                vec![
                    "translate",
                    "--file",
                    "$ZED_FILE",
                    "--project-root",
                    "$ZED_WORKTREE_ROOT",
                ]
            }
            .into_iter()
            .map(String::from),
        );
        if remote && !configuring {
            args.push("--remote".into());
        } else if !configuring {
            args.push("--auto-remote".into());
        }
        if label == labels[1] {
            args.push("--force".into());
        }
        if !remote {
            for arg in &mut args {
                let name = match arg.as_str() {
                    "$ZED_FILE" => "MD_TRANSLATOR_TASK_FILE",
                    "$ZED_WORKTREE_ROOT" => "MD_TRANSLATOR_TASK_ROOT",
                    _ => continue,
                };
                *arg = if cfg!(windows) {
                    format!("\"$env:{name}\"")
                } else {
                    format!("\"${name}\"")
                };
            }
        }
        let mut value = json!({"label":label,"command":command,"args":args,"save":"current",
            "reveal":if configuring {"always"} else {"no_focus"},"hide":"never","use_new_terminal":false,"allow_concurrent_runs":false,
            "show_command":false,"show_summary":true});
        if !remote {
            value["shell"] = task_launcher().1;
            if !configuring {
                // Pass file paths as environment data, not shell source after Zed interpolation.
                value["env"] = json!({"MD_TRANSLATOR_TASK_FILE":"$ZED_FILE","MD_TRANSLATOR_TASK_ROOT":"$ZED_WORKTREE_ROOT"});
            }
        }
        let existing = array
            .elements()
            .into_iter()
            .find(|node| node.to_serde_value().is_some_and(|v| v["label"] == label));
        if let Some(node) = existing {
            let object = node.as_object().context("已有翻译任务格式错误")?;
            for (key, v) in value.as_object().unwrap() {
                if key == "env" {
                    let env = object.object_value_or_set("env");
                    for (key, value) in v.as_object().unwrap() {
                        set(&env, key, value);
                    }
                } else {
                    set(&object, key, v);
                }
            }
        } else {
            array.append(input(&value));
        }
    }
    Ok(root.to_string())
}

pub fn merge_keymap(text: &str) -> Result<String> {
    let value = parse_jsonc(text)?;
    let entries = value.as_array().context("Zed keymap.json 必须是数组")?;
    let (normal, force) = if cfg!(target_os = "macos") {
        ("ctrl-shift-t", "ctrl-alt-shift-t")
    } else {
        ("ctrl-k alt-t", "ctrl-k alt-shift-t")
    };
    let bindings = json!({normal:["task::Spawn",{"task_name":TRANSLATE}],force:["task::Spawn",{"task_name":RETRANSLATE}]});
    for entry in entries {
        for key in [normal, force] {
            if let Some(old) = entry.get("bindings").and_then(|v| v.get(key)) {
                ensure!(
                    old[0] == "task::Spawn"
                        && [TRANSLATE, RETRANSLATE]
                            .iter()
                            .any(|name| old[1]["task_name"] == *name),
                    "快捷键 {key} 已被使用，未覆盖原配置。请先为翻译选择可用的快捷键。"
                );
            }
        }
    }
    let root = CstRootNode::parse(text, &Default::default())?;
    let array = root.array_value().unwrap();
    let legacy_normal = cfg!(target_os = "macos").then_some("ctrl-alt-t");
    let is_managed = |key: &str, action: &Value| {
        action[0] == "task::Spawn"
            && if Some(key) == legacy_normal {
                action[1]["task_name"] == TRANSLATE
            } else {
                [normal, force].contains(&key)
                    && [TRANSLATE, RETRANSLATE]
                        .iter()
                        .any(|name| action[1]["task_name"] == *name)
            }
    };
    let mut installed = false;
    for node in array.elements() {
        let Some(value) = node.to_serde_value() else {
            continue;
        };
        let Some(old_bindings) = value["bindings"].as_object() else {
            continue;
        };
        let managed: Vec<_> = old_bindings
            .iter()
            .filter(|(key, action)| is_managed(key, action))
            .map(|(key, _)| key)
            .collect();
        if managed.is_empty() {
            continue;
        }
        let object = node.as_object().context("已有快捷键配置格式错误")?;
        let existing_bindings = object.object_value_or_set("bindings");
        // Only a group containing managed bindings alone may change context.
        let reuse = !installed && managed.len() == old_bindings.len();
        if reuse {
            set(&object, "context", &json!(CONTEXT));
            for (key, value) in bindings.as_object().unwrap() {
                set(&existing_bindings, key, value);
            }
            installed = true;
        }
        for key in managed {
            if (!reuse || Some(key.as_str()) == legacy_normal)
                && let Some(property) = existing_bindings.get(key)
            {
                property.remove();
            }
        }
    }
    if !installed {
        array.append(input(&json!({"context":CONTEXT,"bindings":bindings})));
    }
    Ok(root.to_string())
}

pub struct Change {
    pub path: PathBuf,
    pub before: Option<String>,
    pub after: String,
    pub backup: Option<PathBuf>,
}

pub fn install(
    config_dir: &Path,
    binary: &Path,
    auto_preview: bool,
    dry_run: bool,
) -> Result<Vec<Change>> {
    install_with_args(config_dir, binary, auto_preview, dry_run, &[])
}

pub fn install_with_args(
    config_dir: &Path,
    binary: &Path,
    auto_preview: bool,
    dry_run: bool,
    prefix: &[String],
) -> Result<Vec<Change>> {
    install_files(config_dir, binary, auto_preview, dry_run, prefix, false)
}

pub fn install_remote_tasks(
    project: &Path,
    binary: &Path,
    dry_run: bool,
    prefix: &[String],
) -> Result<Vec<Change>> {
    let project = project
        .canonicalize()
        .context("无法读取 --project-root 目录")?;
    ensure!(project.is_dir(), "--project-root 必须是目录");
    install_files(&project.join(".zed"), binary, false, dry_run, prefix, true)
}

pub fn uninstall(config_dir: &Path, remote: bool, dry_run: bool) -> Result<Vec<Change>> {
    let labels = if remote {
        [REMOTE_TRANSLATE, REMOTE_RETRANSLATE, REMOTE_CONFIGURE]
    } else {
        [TRANSLATE, RETRANSLATE, CONFIGURE]
    };
    let mut changes = Vec::new();
    for name in ["tasks.json", "keymap.json"] {
        if remote && name == "keymap.json" {
            continue;
        }
        let path = config_dir.join(name);
        let before = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        };
        let root = CstRootNode::parse(&before, &Default::default())?;
        let array = root
            .array_value()
            .with_context(|| format!("Zed {name} 必须是数组"))?;
        let mut changed = false;
        for node in array.elements() {
            let Some(value) = node.to_serde_value() else {
                continue;
            };
            if name == "tasks.json" {
                if value["label"]
                    .as_str()
                    .is_some_and(|label| labels.contains(&label))
                {
                    node.remove();
                    changed = true;
                }
            } else if let Some(bindings) = value["bindings"].as_object() {
                for (key, action) in bindings {
                    if action[0] == "task::Spawn"
                        && action[1]["task_name"]
                            .as_str()
                            .is_some_and(|label| labels.contains(&label))
                    {
                        let object = node.as_object().context("快捷键配置格式错误")?;
                        if let Some(property) = object.object_value_or_set("bindings").get(key) {
                            property.remove();
                            changed = true;
                        }
                    }
                }
            }
        }
        if changed {
            let mut after = root.to_string();
            if !after.ends_with('\n') {
                after.push('\n');
            }
            changes.push(Change {
                path,
                before: Some(before),
                after,
                backup: None,
            });
        }
    }
    write_changes(changes, dry_run)
}

fn install_files(
    config_dir: &Path,
    binary: &Path,
    auto_preview: bool,
    dry_run: bool,
    prefix: &[String],
    remote: bool,
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for name in ["tasks.json", "keymap.json", "settings.json"] {
        if remote && name != "tasks.json" {
            continue;
        }
        if name == "settings.json" && !auto_preview {
            continue;
        }
        let path = config_dir.join(name);
        let before = match fs::read_to_string(&path) {
            Ok(text) => Some(text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        let text = before.as_deref().unwrap_or(if name == "settings.json" {
            "{}\n"
        } else {
            "[]\n"
        });
        let mut after = match name {
            "tasks.json" => merge_tasks_with_args(text, binary, prefix, remote)?,
            "keymap.json" => merge_keymap(text)?,
            "settings.json" => {
                ensure!(
                    parse_jsonc(text)?.is_object(),
                    "Zed settings.json 必须是对象"
                );
                let root = CstRootNode::parse(text, &Default::default())?;
                let settings = root
                    .object_value()
                    .unwrap()
                    .object_value_or_set("markdown_preview");
                set(
                    &settings,
                    "open_markdown_files_in_preview",
                    &Value::Bool(true),
                );
                root.to_string()
            }
            _ => unreachable!(),
        };
        if !after.ends_with('\n') {
            after.push('\n');
        }
        if before.as_deref() != Some(&after) {
            changes.push(Change {
                path,
                before,
                after,
                backup: None,
            });
        }
    }
    write_changes(changes, dry_run)
}

fn write_changes(mut changes: Vec<Change>, dry_run: bool) -> Result<Vec<Change>> {
    if !dry_run {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        for change in &mut changes {
            // Abort rather than overwrite concurrent settings edits.
            let current = match fs::read_to_string(&change.path) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.into()),
            };
            if current != change.before {
                bail!("{} 在操作期间已改变，请重试。", change.path.display());
            }
            if let Some(before) = &change.before {
                let backup = change.path.with_file_name(format!(
                    "{}.md-translator-{stamp}.bak",
                    change.path.file_name().unwrap().to_string_lossy()
                ));
                atomic_write(&backup, before)?;
                change.backup = Some(backup);
            }
        }
        for change in &changes {
            atomic_write(&change.path, &change.after)?;
        }
    }
    Ok(changes)
}
