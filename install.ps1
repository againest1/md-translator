param(
    [string]$Version = 'latest',
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\md-translator')
)
$ErrorActionPreference = 'Stop'
if (-not [Environment]::Is64BitOperatingSystem) { throw '需要 64 位 Windows。' }
if ($Version -ne 'latest' -and $Version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') {
    throw '版本格式应为 v0.4.1。'
}
if (-not [IO.Path]::IsPathRooted($InstallDir)) { throw 'InstallDir 必须是绝对路径。' }
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$base = 'https://github.com/againest1/md-translator/releases'
if ($Version -eq 'latest') {
    $release = Invoke-RestMethod 'https://api.github.com/repos/againest1/md-translator/releases/latest'
    $Version = $release.tag_name
    if ($Version -notmatch '^v[0-9]+\.[0-9]+\.[0-9]+$') { throw '无法确定最新版本，请稍后重试。' }
}
$archive = 'md-translator-x86_64-pc-windows-msvc.zip'
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('md-translator-' + [guid]::NewGuid())
$staged = $null
try {
    New-Item -ItemType Directory -Path $temporary | Out-Null
    Write-Host "正在下载 $Version（Windows x64）…"
    Invoke-WebRequest -UseBasicParsing "$base/download/$Version/$archive" -OutFile (Join-Path $temporary $archive)
    Invoke-WebRequest -UseBasicParsing "$base/download/$Version/$archive.sha256" -OutFile (Join-Path $temporary "$archive.sha256")
    $expected = ((Get-Content (Join-Path $temporary "$archive.sha256") -Raw).Trim() -split '\s+')[0]
    $actual = (Get-FileHash (Join-Path $temporary $archive) -Algorithm SHA256).Hash
    if ($expected -notmatch '^[0-9a-fA-F]{64}$' -or $actual -ne $expected) { throw '下载校验失败，原安装未改变。' }
    Expand-Archive -LiteralPath (Join-Path $temporary $archive) -DestinationPath (Join-Path $temporary 'package')
    $binary = Join-Path $temporary 'package\md-translator.exe'
    $reported = & $binary --version
    if ($LASTEXITCODE -ne 0 -or $reported -ne "md-translator $($Version.Substring(1))") {
        throw '下载的程序无法运行或版本不符；原安装未改变。'
    }
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    $destination = Join-Path $InstallDir 'md-translator.exe'
    if (Test-Path -LiteralPath $destination -PathType Container) { throw '安装位置已存在同名目录。' }
    $staged = Join-Path $InstallDir ('.md-translator-install-' + [guid]::NewGuid() + '.exe')
    Copy-Item -LiteralPath $binary -Destination $staged
    if (Test-Path -LiteralPath $destination) {
        # Replace only after download, verification and execution have succeeded.
        [IO.File]::Replace($staged, $destination, [NullString]::Value)
    } else {
        [IO.File]::Move($staged, $destination)
    }
    $staged = $null
    Write-Host "已安装 $Version：$destination"
    Write-Host '更新已保留配置、密钥和缓存。'
    Write-Host '本机使用：运行 install-zed 安装或刷新入口；首次使用还需 configure 配置服务。'
    Write-Host ('  & "{0}" install-zed --auto-preview' -f $destination)
    Write-Host ('  & "{0}" configure' -f $destination)
    Write-Host '随后在 Zed 的 task: spawn 选择「Markdown: 翻译全文」。'
    Write-Host 'macOS / Linux SSH Remote Project 请按使用指南在远端安装显式 --remote 项目 Task，选择 Markdown (Remote) 入口。'
    Write-Host '远端译文在当前 Zed 任务终端 Cmd+Click / Ctrl+Click 链接打开。'
} catch {
    throw "安装未完成：$_。请结束正在运行的翻译任务后重试，或到 GitHub Releases 手动下载。"
} finally {
    if ($staged -and (Test-Path -LiteralPath $staged)) { Remove-Item -LiteralPath $staged -Force }
    if (Test-Path -LiteralPath $temporary) { Remove-Item -LiteralPath $temporary -Recurse -Force }
}
