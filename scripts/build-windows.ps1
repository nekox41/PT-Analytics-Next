# SPDX-License-Identifier: MIT
param([switch]$Bootstrap)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
function Run([string]$Exe, [string[]]$Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Exe exited with code $LASTEXITCODE" }
}
function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path','Machine')
    $user = [Environment]::GetEnvironmentVariable('Path','User')
    $env:Path = "$env:LOCALAPPDATA\Programs\Python\Python312;$env:USERPROFILE\.cargo\bin;$env:Path;$machine;$user"
}
function Install-Dependency([string]$Id, [string[]]$Extra = @()) {
    if (-not $Bootstrap) { throw "Missing Windows dependency: $Id. Run bash build-release.sh --bootstrap, or install it manually." }
    if (-not (Get-Command winget.exe -ErrorAction SilentlyContinue)) { throw 'Windows winget is required for --bootstrap. Install App Installer or the dependencies manually.' }
    Run 'winget.exe' (@('install','--id',$Id,'--exact','--accept-package-agreements','--accept-source-agreements','--disable-interactivity') + $Extra)
    Refresh-Path
}
Refresh-Path
if (-not (Get-Command node.exe -ErrorAction SilentlyContinue)) { Install-Dependency 'OpenJS.NodeJS.LTS' }
function Test-Python {
    if (-not (Get-Command python.exe -ErrorAction SilentlyContinue)) { return $false }
    & python.exe -c 'import sys; sys.exit(0 if sys.version_info >= (3,11) else 1)' 2>$null
    return ($LASTEXITCODE -eq 0)
}
if (-not (Test-Python)) { Install-Dependency 'Python.Python.3.12' }
if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) { Install-Dependency 'Rustlang.Rustup' }
$vswhere = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft Visual Studio\Installer\vswhere.exe'
function Find-MSVC {
    if (Test-Path $vswhere) {
        & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    }
}
if (-not (Find-MSVC)) {
    Install-Dependency 'Microsoft.VisualStudio.2022.BuildTools' @('--override','--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
}
if (-not (Find-MSVC)) { throw 'MSVC x64 build tools not detected. Install the Desktop development with C++ workload and Windows SDK, then rerun.' }
Run 'node.exe' @('-e','if(Number(process.versions.node.split(String.fromCharCode(46))[0])<22)process.exit(1)')
Run 'python.exe' @('-c','import sys; assert sys.version_info >= (3,11)')
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-msvc'
Run 'rustup.exe' @('toolchain','install',$env:RUSTUP_TOOLCHAIN,'--profile','minimal')
$target = 'x86_64-pc-windows-msvc'
$env:CARGO_TARGET_DIR = Join-Path (Get-Location) 'src-tauri\target'
Run 'python.exe' @('-m','venv','.build-venv')
$venv = Join-Path (Get-Location) '.build-venv\Scripts'
$env:Path = "$venv;$env:Path"
Run 'python.exe' @('-m','pip','install','--disable-pip-version-check','Pillow==11.3.0')
if (Test-Path 'package-lock.json') { Run 'npm.cmd' @('ci','--no-audit','--no-fund') }
else { Run 'npm.cmd' @('install','--no-audit','--no-fund') }
Run 'npm.cmd' @('run','icons')
Run 'npm.cmd' @('run','check')
Run 'cargo.exe' @('test','--manifest-path','src-tauri/Cargo.toml','--release','--target',$target)
Run 'npm.cmd' @('run','build','--','--target',$target,'--bundles','nsis')
$version = (Get-Content 'src-tauri/tauri.conf.json' -Raw | ConvertFrom-Json).version
$releaseDir = Join-Path (Get-Location) "src-tauri\target\$target\release"
$exe = Join-Path $releaseDir 'pt-analytics-next.exe'
if (-not (Test-Path $exe)) { throw "Expected executable not found: $exe" }
$installers = @(Get-ChildItem (Join-Path $releaseDir 'bundle\nsis') -Filter '*.exe' -File)
if ($installers.Count -ne 1) { throw "Expected exactly one NSIS installer, found $($installers.Count)" }
New-Item -ItemType Directory -Force 'dist' | Out-Null
Copy-Item $exe "dist/PT-Analytics-Next_$($version)_x64_portable.exe" -Force
Copy-Item $installers[0].FullName "dist/PT-Analytics-Next_$($version)_x64_setup.exe" -Force
Write-Host "Windows x64 executable and installer built successfully."
