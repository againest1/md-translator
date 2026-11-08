#!/usr/bin/env python3
"""Release operations; version selection, review and recovery live in docs/release.md."""

import argparse
import hashlib
import json
import platform
import re
import shlex
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
NOTES = ".github/release-notes.md"
WORKFLOW = ".github/workflows/release.yml"
RUST_TESTS = ("workflow", "remote", "translation")


class ReleaseError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise ReleaseError(message)


def run(*args, capture=False, check=True):
    if not capture:
        print("+ " + shlex.join(args), flush=True)
    return subprocess.run(args, cwd=ROOT, text=True, capture_output=capture, check=check)


def output(*args):
    return run(*args, capture=True).stdout.strip()


def tag_version(tag):
    require(re.fullmatch(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", tag),
            "版本须为 vX.Y.Z，例如 v0.4.4。")
    return tag[1:]


def package_version():
    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]
    lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    versions = [p["version"] for p in lock["package"] if p["name"] == package["name"]]
    require(versions == [package["version"]], "Cargo.toml 与 Cargo.lock 的项目版本不一致。")
    return package["version"]


def check_notes(tag):
    require(f"**{tag} 更新**" in (ROOT / NOTES).read_text(),
            f"请先编辑 {NOTES}，写明 **{tag} 更新**、实际变化和必要的升级操作。")


def clean_tree():
    require(not output("git", "status", "--porcelain"),
            "工作区存在未提交文件；先审查并提交本次改动，保留无关改动。")


def github_repo(url):
    match = re.fullmatch(r"(?:https://github\.com/|git@github\.com:)([^/]+/[^/]+?)(?:\.git)?/?", url)
    require(match, f"需要 GitHub 仓库地址，当前为：{url}")
    return match[1]


def repository():
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text())
    return github_repo(manifest["package"]["repository"])


def api(path):
    return json.loads(output("gh", "api", f"repos/{repository()}/{path}"))


def source(tag, path):
    return subprocess.check_output(["git", "show", f"{tag}:{path}"], cwd=ROOT)


def prepare(args):
    version = tag_version(args.tag)
    previous = package_version()
    require(tuple(map(int, version.split("."))) > tuple(map(int, previous.split("."))),
            f"新版本须高于 {previous}；已经准备过的版本无需再次 prepare。")
    clean_tree()
    manifest = (ROOT / "Cargo.toml").read_text()
    lock = (ROOT / "Cargo.lock").read_text()
    old = f'name = "md-translator"\nversion = "{previous}"'
    require(old in lock, "无法定位 Cargo.lock 中的项目版本。")
    manifest = manifest.replace(f'version = "{previous}"', f'version = "{version}"', 1)
    require(tomllib.loads(manifest)["package"]["version"] == version,
            "无法定位 Cargo.toml 中的项目版本。")
    lock = lock.replace(old, f'name = "md-translator"\nversion = "{version}"', 1)
    (ROOT / "Cargo.toml").write_text(manifest)
    (ROOT / "Cargo.lock").write_text(lock)
    print(f"已准备 {args.tag}。接下来编辑 {NOTES}，然后运行 check。")


def check(args):
    check_notes("v" + package_version())
    for name in [*(f"tests/{suite}.rs" for suite in RUST_TESTS), "tests/release.py"]:
        require((ROOT / name).is_file(), f"缺少本地回归文件 {name}；按 docs/testing-guide.md 从原工作区恢复。")
    run("git", "diff", "--check")
    run("git", "diff", "--cached", "--check")
    run("cargo", "fmt", "--check")
    run("cargo", "clippy", "--locked", "--all-targets", "--", "-D", "warnings")
    test_args = [arg for suite in RUST_TESTS for arg in ("--test", suite)]
    run("cargo", "test", "--locked", *test_args)
    run(sys.executable, "tests/release.py")
    run("sh", "-n", "install.sh")
    print("本地发布检查通过。请完成审查、提交和 main 的线性集成，再运行 push。")


def push(args):
    version = tag_version(args.tag)
    require(package_version() == version, "标签与项目版本不一致。")
    check_notes(args.tag)
    clean_tree()
    require(output("git", "branch", "--show-current") == "main", "请先按发布指南将改动线性合入 main。")
    repo = repository()
    for options in [(), ("--push",)]:
        url = output("git", "remote", "get-url", *options, "origin")
        require(github_repo(url) == repo, f"origin 与 Cargo.toml 的发布仓库不一致：{url}")
    print(f"发布目标：https://github.com/{repo}；标签：{args.tag}", flush=True)
    run("git", "fetch", "origin", "--tags")
    ancestor = run("git", "merge-base", "--is-ancestor", "origin/main", "HEAD", capture=True, check=False)
    require(ancestor.returncode == 0, "远端 main 有未集成的更新；按发布指南处理后重试。")
    require(not output("git", "rev-list", "--merges", "origin/main..HEAD"),
            "待推送历史包含 merge commit；请先恢复线性历史。")
    existing = run("git", "rev-parse", "--verify", "--quiet", f"refs/tags/{args.tag}", capture=True, check=False)
    if existing.returncode == 0:
        require(output("git", "rev-parse", f"{args.tag}^{{commit}}") == output("git", "rev-parse", "HEAD"),
                "版本标签已指向其他提交；不得移动已发布标签，请检查版本选择。")
    else:
        run("git", "tag", "-a", args.tag, "-m", f"Markdown Translator {args.tag}")
    run("git", "push", "--atomic", "origin", "HEAD:refs/heads/main", f"refs/tags/{args.tag}")
    print(f"已推送。运行 status {args.tag} 查看 GitHub 构建，成功后运行 verify {args.tag}。")


def workflow_run(tag):
    commit = output("git", "rev-parse", f"{tag}^{{commit}}")
    runs = json.loads(output("gh", "run", "list", "--repo", repository(), "--workflow", "release.yml",
                             "--branch", tag, "--limit", "20", "--json",
                             "databaseId,headSha,status,conclusion,url"))
    return next((item for item in runs if item["headSha"] == commit), None)


def status(args):
    tag_version(args.tag)
    item = workflow_run(args.tag)
    require(item, "尚未发现这个标签和提交的发布运行；稍后重试 status。")
    print(f"{item['status']} / {item['conclusion'] or '等待结果'}\n{item['url']}", flush=True)
    if args.watch and item["status"] != "completed":
        run("gh", "run", "watch", str(item["databaseId"]), "--repo", repository(), "--exit-status", "--interval", "30")
    else:
        require(item["status"] != "completed" or item["conclusion"] == "success",
                "发布运行未成功；请按 docs/release.md 排查。")


def native_target():
    machine = platform.machine().lower()
    arch = {"arm64": "aarch64", "aarch64": "aarch64", "amd64": "x86_64", "x86_64": "x86_64"}.get(machine)
    suffix = {"Darwin": "apple-darwin", "Linux": "unknown-linux-musl", "Windows": "pc-windows-msvc"}.get(platform.system())
    require(arch and suffix, f"当前机器没有已支持的发布目标：{platform.system()} {machine}")
    return f"{arch}-{suffix}"


def verify(args):
    version = tag_version(args.tag)
    item = workflow_run(args.tag)
    require(item and item["status"] == "completed" and item["conclusion"] == "success",
            "对应标签和提交的发布运行尚未成功；先运行 status。")
    release = api(f"releases/tags/{args.tag}")
    require(release["tag_name"] == args.tag and not release["draft"] and not release["prerelease"],
            "目标版本尚未作为正式 Release 公开。")
    require(api("releases/latest")["tag_name"] == args.tag, "目标版本不是 Latest；请核对发布状态。")
    require(release["body"].strip() == source(args.tag, NOTES).decode().strip(), "公开发布说明与标签中的说明不一致。")
    targets = re.findall(r"^\s+target: ([\w-]+)\s*$", source(args.tag, WORKFLOW).decode(), re.MULTILINE)
    require(targets and len(set(targets)) == len(targets), "无法从标签的 release.yml 读取唯一的构建目标。")
    archives = {target: f"md-translator-{target}." + ("zip" if "windows" in target else "tar.gz") for target in targets}
    expected = {"install.sh", "install.ps1", "SHA256SUMS", *archives.values(), *(name + ".sha256" for name in archives.values())}
    require({asset["name"] for asset in release["assets"]} == expected, "Release 附件不齐全或存在未预期附件。")
    host = native_target()
    require(host in archives, f"此发布没有当前机器的程序包：{host}")
    examples = output("git", "ls-tree", "-r", "--name-only", args.tag, "--", "examples").splitlines()
    with tempfile.TemporaryDirectory(prefix="md-translator-release-") as directory:
        destination = Path(directory)
        run("gh", "release", "download", args.tag, "--repo", repository(), "--dir", directory)
        for asset in release["assets"]:
            payload = (destination / asset["name"]).read_bytes()
            require(asset["state"] == "uploaded" and len(payload) == asset["size"] > 0, f"附件大小或状态异常：{asset['name']}")
            digest = "sha256:" + hashlib.sha256(payload).hexdigest()
            require(not asset.get("digest") or asset["digest"] == digest, f"附件摘要不符：{asset['name']}")
        checksums = {}
        for line in (destination / "SHA256SUMS").read_text().splitlines():
            match = re.fullmatch(r"([0-9a-f]{64})  (\S+)", line)
            require(match and match[2] not in checksums, "SHA256SUMS 格式错误或存在重复文件。")
            checksums[match[2]] = match[1]
        require(set(checksums) == set(archives.values()), "SHA256SUMS 未覆盖全部安装包。")
        for target, filename in archives.items():
            archive = destination / filename
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            require(digest == checksums[filename], f"安装包校验失败：{filename}")
            require((destination / (filename + ".sha256")).read_bytes() == f"{digest}  {filename}\n".encode(),
                    f"独立校验文件不符：{filename}")
            windows = filename.endswith(".zip")
            binary_name = "md-translator.exe" if windows else "md-translator"
            installer = "install.ps1" if windows else "install.sh"
            with zipfile.ZipFile(archive) if windows else tarfile.open(archive) as package:
                def read(name):
                    return package.read(name) if windows else package.extractfile(name).read()

                for name in ["README.md", "USAGE.md", "LICENSE", installer, *examples]:
                    actual = read(name)
                    expected_content = source(args.tag, name)
                    # Windows checkout uses CRLF for text; compare content, not line endings.
                    if windows and (name == "LICENSE" or Path(name).suffix in (".md", ".ps1")):
                        actual = actual.replace(b"\r\n", b"\n")
                        expected_content = expected_content.replace(b"\r\n", b"\n")
                    require(actual == expected_content, f"包内文件与标签不符：{filename}/{name}")
                executable = read(binary_name)
                require(executable, f"安装包缺少程序内容：{filename}")
                if target == host:
                    binary = destination / binary_name
                    binary.write_bytes(executable)
                    binary.chmod(0o755)
                    require(output(str(binary), "--version") == f"md-translator {version}", "发布程序运行失败或版本不符。")
            print(f"安装包校验通过：{filename}", flush=True)
        for name in ("install.sh", "install.ps1"):
            require((destination / name).read_bytes() == source(args.tag, name), f"安装脚本与标签不符：{name}")
    print(f"发布验收通过（{len(expected)} 个附件，本机程序 {host}）。\n{release['html_url']}")


def main():
    parser = argparse.ArgumentParser(description="Markdown Translator 发布操作；先阅读 docs/release.md。")
    commands = parser.add_subparsers(dest="command", required=True)
    for name, handler, help_text in [
        ("prepare", prepare, "更新两个 Cargo 文件的版本；发布说明由维护者编写"),
        ("check", check, "运行本地完整回归和严格校验"),
        ("push", push, "检查 main 和标签，原子推送到 origin，触发正式发布"),
        ("status", status, "查看对应提交的发布运行；--watch 等待完成"),
        ("verify", verify, "下载并验收最新正式 Release；临时文件自动清理"),
    ]:
        command = commands.add_parser(name, help=help_text)
        if name != "check":
            command.add_argument("tag", help="版本标签，如 v0.4.4")
        if name == "status":
            command.add_argument("--watch", action="store_true")
        command.set_defaults(handler=handler)
    args = parser.parse_args()
    try:
        args.handler(args)
    except (ReleaseError, OSError, ValueError, KeyError, subprocess.CalledProcessError, tarfile.TarError, zipfile.BadZipFile) as error:
        print(f"发布操作停止：{error}", file=sys.stderr)
        if isinstance(error, subprocess.CalledProcessError) and error.stderr:
            print(error.stderr.strip(), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
