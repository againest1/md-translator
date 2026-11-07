#!/bin/sh
set -eu

fail() { printf '%s\n' "$*" >&2; exit 1; }
version=latest
bin_dir="${HOME:?}/.local/bin"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version|--bin-dir)
            [ "$#" -ge 2 ] || fail "$1 需要一个值。"
            case "$1" in
                --version) version=$2 ;;
                --bin-dir) bin_dir=$2 ;;
            esac
            shift 2 ;;
        --help|-h)
            printf '%s\n' '安装或更新 md-translator；保留配置、密钥和缓存。' \
                '用法：sh install.sh [--version v0.4.1] [--bin-dir /absolute/path]' \
                '默认安装到 ~/.local/bin；不需要 sudo。'
            exit 0 ;;
        *) fail "未知选项：$1（使用 --help 查看帮助）" ;;
    esac
done
case "$bin_dir" in /*) ;; *) fail '--bin-dir 必须是绝对路径。' ;; esac
if [ "$version" != latest ]; then
    printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || fail '版本格式应为 v0.4.1。'
fi
case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) target=aarch64-apple-darwin ;;
    Darwin-x86_64) target=x86_64-apple-darwin ;;
    Linux-aarch64|Linux-arm64) target=aarch64-unknown-linux-musl ;;
    Linux-x86_64) target=x86_64-unknown-linux-musl ;;
    *) fail '此系统或架构没有预编译包。Windows 请使用 install.ps1。' ;;
esac
for tool in curl tar; do
    command -v "$tool" >/dev/null 2>&1 || fail "缺少 $tool，请先通过系统软件管理器安装。"
done
if command -v sha256sum >/dev/null 2>&1; then
    checksum=sha256sum
elif command -v shasum >/dev/null 2>&1; then
    checksum=shasum
else
    fail '缺少 SHA-256 校验工具，请安装 sha256sum 或 shasum。'
fi
base=https://github.com/againest1/md-translator/releases
if [ "$version" = latest ]; then
    # Resolve once so the archive and checksum always come from the same release.
    release_url=$(curl -fsSL --retry 3 --connect-timeout 15 -o /dev/null -w '%{url_effective}' "$base/latest")
    version=${release_url##*/}
    printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || fail '无法确定最新版本，请稍后重试。'
fi
archive="md-translator-$target.tar.gz"
download="$base/download/$version"
temporary=$(mktemp -d)
staged=
cleanup() {
    rm -rf "$temporary"
    if [ -n "$staged" ]; then rm -f "$staged"; fi
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
printf '正在下载 %s（%s）…\n' "$version" "$target"
curl -fsSL --retry 3 --connect-timeout 15 "$download/$archive" -o "$temporary/$archive"
curl -fsSL --retry 3 --connect-timeout 15 "$download/$archive.sha256" -o "$temporary/$archive.sha256"
(
    cd "$temporary"
    if [ "$checksum" = sha256sum ]; then
        sha256sum -c "$archive.sha256"
    else
        shasum -a 256 -c "$archive.sha256"
    fi
)
tar -xzf "$temporary/$archive" -C "$temporary" md-translator
chmod 755 "$temporary/md-translator"
[ "$("$temporary/md-translator" --version)" = "md-translator ${version#v}" ] || fail '下载的程序无法运行或版本不符；原安装未改变。'
mkdir -p "$bin_dir"
[ ! -d "$bin_dir/md-translator" ] || fail '安装位置已存在同名目录，请选择其它 --bin-dir。'
staged=$(mktemp "$bin_dir/.md-translator-install.XXXXXX")
cp "$temporary/md-translator" "$staged"
chmod 755 "$staged"
mv -f "$staged" "$bin_dir/md-translator"
staged=
printf '\n已安装 %s：%s/md-translator\n' "$version" "$bin_dir"
printf '%s\n' '更新已保留配置、密钥和缓存。' \
    '本机使用：运行 install-zed 安装或刷新入口；首次使用还需 configure 配置服务。'
printf '  "%s/md-translator" install-zed --auto-preview\n  "%s/md-translator" configure\n' "$bin_dir" "$bin_dir"
printf '%s\n' '随后在 Zed 的 task: spawn 选择「Markdown: 翻译全文」，或使用翻译快捷键。' \
    'macOS / Linux SSH Remote Project 使用相同入口和快捷键；远端只需安装程序并配置服务，无需逐个项目安装 Task。' \
    '远端译文在当前 Zed 任务终端 Cmd+Click / Ctrl+Click 链接打开。' \
    '使用指南：https://github.com/againest1/md-translator/blob/main/USAGE.md'
