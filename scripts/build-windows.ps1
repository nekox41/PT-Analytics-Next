# SPDX-License-Identifier: MIT
param([switch]$Bootstrap)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
. (Join-Path $PSScriptRoot 'checksums.ps1')
function Run([string]$Exe, [string[]]$Arguments) {
    & $Exe @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Exe exited with code $LASTEXITCODE" }
}
# Explicit TLS 1.2 for Windows PowerShell 5.1 on older Windows installations.
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$ProgressPreference = 'SilentlyContinue'
$toolRoot = Join-Path $env:LOCALAPPDATA 'PTAnalyticsNext\build-tools'
$nodeDir = Join-Path $toolRoot 'node22'
$pythonDir = Join-Path $env:LOCALAPPDATA 'Programs\Python\Python313'
$downloads = Join-Path $toolRoot 'downloads'
function Refresh-Path {
    $machine = [Environment]::GetEnvironmentVariable('Path','Machine')
    $user = [Environment]::GetEnvironmentVariable('Path','User')
    $paths = @($nodeDir,$pythonDir,"$env:LOCALAPPDATA\Programs\Python\Python312","$env:USERPROFILE\.cargo\bin","$env:LOCALAPPDATA\Microsoft\WindowsApps","$env:ProgramFiles\nodejs") + ($env:Path -split ';') + ($machine -split ';') + ($user -split ';')
    $env:Path = ($paths | Where-Object { $_ } | Select-Object -Unique) -join ';'
}
function Fetch([string]$Url,[string]$Path) {
    New-Item -ItemType Directory -Force (Split-Path $Path -Parent) | Out-Null
    Write-Host "Downloading $Url"
    Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $Path -TimeoutSec 300
}
function Verify-Sha256([string]$Path,[string]$Expected) {
    if ($Expected -notmatch '^[0-9a-fA-F]{64}$' -or (Get-FileHash $Path -Algorithm SHA256).Hash -ne $Expected) { throw "SHA256 verification failed: $Path" }
}
# Reuse an existing download only if it matches the freshly downloaded official hash.
function Fetch-Verified([string]$Url,[string]$Path,[string]$Expected) {
    if ($Expected -notmatch '^[0-9a-fA-F]{64}$') { throw 'Invalid expected SHA256.' }
    if ((Test-Path -LiteralPath $Path -PathType Leaf) -and (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -eq $Expected) {
        Write-Host "Reusing verified download: $Path"
        return
    }
    Fetch $Url $Path
    Verify-Sha256 $Path $Expected
}
function Verify-Signature([string]$Path,[string]$Publisher) {
    $sig=Get-AuthenticodeSignature -FilePath $Path
    if ($sig.Status -ne 'Valid' -or $sig.SignerCertificate.Subject -notmatch $Publisher) { throw "Installer signature verification failed: $Path ($($sig.Status))" }
}
function Start-Installer([string]$Path,[string[]]$ArgsList,[switch]$Elevate) {
    $options=@{FilePath=$Path;ArgumentList=$ArgsList;Wait=$true;PassThru=$true}
    if ($Elevate) {
        $identity=[Security.Principal.WindowsIdentity]::GetCurrent()
        $principal=New-Object Security.Principal.WindowsPrincipal($identity)
        if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { $options.Verb='RunAs' }
    }
    $process=Start-Process @options
    if ($process.ExitCode -in @(1641,3010)) { throw 'Installation requires a Windows restart. Restart Windows, reopen WSL and rerun the build.' }
    if ($process.ExitCode -ne 0) { throw "Installer failed ($($process.ExitCode)): $Path" }
}
function Find-Winget {
    $cmd=Get-Command winget.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    if (Get-Command Get-AppxPackage -ErrorAction SilentlyContinue) {
        try { $pkg=Get-AppxPackage -Name Microsoft.DesktopAppInstaller -ErrorAction Stop | Select-Object -First 1 } catch { return }
        if ($pkg) {
            $exe=Join-Path $pkg.InstallLocation 'winget.exe'
            if (Test-Path $exe) { return $exe }
        }
    }
}
function Install-Direct([string]$Id) {
    switch ($Id) {
        'OpenJS.NodeJS.LTS' {
            # Use the current Node 22 LTS x64 ZIP from the official release directory.
            $url='https://nodejs.org/dist/latest-v22.x'
            $checksumFile=Join-Path $downloads 'node-SHASUMS256.txt'
            Fetch "$url/SHASUMS256.txt" $checksumFile
            $match=Get-NodeZipMatch (Read-ChecksumText $checksumFile)
            $name=$match.Groups[2].Value
            $zip=Join-Path $downloads $name
            Fetch-Verified "$url/$name" $zip $match.Groups[1].Value
            $unpack=Join-Path $downloads ([guid]::NewGuid().ToString())
            Expand-Archive -LiteralPath $zip -DestinationPath $unpack
            $src=Join-Path $unpack ([IO.Path]::GetFileNameWithoutExtension($name))
            if (-not (Test-Path (Join-Path $src 'node.exe'))) { throw 'Invalid Node.js archive.' }
            if (Test-Path $nodeDir) { Remove-Item -LiteralPath $nodeDir -Recurse -Force }
            Move-Item -LiteralPath $src -Destination $nodeDir
            Remove-Item -LiteralPath $unpack -Recurse -Force
        }
        'Python.Python.3.13' {
            $version='3.13.17'
            $exe=Join-Path $downloads "python-$version-amd64.exe"
            Fetch "https://www.python.org/ftp/python/$version/python-$version-amd64.exe" $exe
            Verify-Signature $exe 'Python Software Foundation'
            Start-Installer $exe @('/quiet','InstallAllUsers=0','PrependPath=1','Include_pip=1','Include_test=0','Include_launcher=0',('TargetDir="'+$pythonDir+'"'))
        }
        'Rustlang.Rustup' {
            $url='https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
            $exe=Join-Path $downloads 'rustup-init.exe'
            $checksumFile=Join-Path $downloads 'rustup-init.exe.sha256'
            Fetch "$url.sha256" $checksumFile
            $sum=Get-RustupSha256 (Read-ChecksumText $checksumFile)
            Fetch-Verified $url $exe $sum
            Run $exe @('-y','--default-host','x86_64-pc-windows-msvc','--profile','minimal','--default-toolchain','stable')
        }
        'Microsoft.VisualStudio.2022.BuildTools' {
            $exe=Join-Path $downloads 'vs_buildtools.exe'
            Fetch 'https://aka.ms/vs/17/release/vs_buildtools.exe' $exe
            Verify-Signature $exe 'Microsoft Corporation'
            Start-Installer $exe @('--wait','--passive','--norestart','--add','Microsoft.VisualStudio.Workload.VCTools','--includeRecommended') -Elevate
        }
        default { throw "Unsupported dependency: $Id" }
    }
}
function Install-Dependency([string]$Id,[string[]]$Extra=@()) {
    if (-not $Bootstrap) { throw "Missing Windows dependency: $Id. Run bash build-release.sh --bootstrap, or install it manually." }
    Write-Host "Preparing missing Windows dependency: $Id"
    $winget=Find-Winget
    if ($winget) {
        try {
            & $winget install --id $Id --exact --accept-package-agreements --accept-source-agreements --disable-interactivity @Extra
            if ($LASTEXITCODE -eq 0) { Refresh-Path; return }
            Write-Warning "winget failed ($LASTEXITCODE); using the official installer."
        } catch { Write-Warning 'winget could not run; using the official installer.' }
    } else { Write-Host 'winget not available; using the official installer.' }
    Install-Direct $Id
    Refresh-Path
}
function Test-Node {
    if (-not (Get-Command node.exe -ErrorAction SilentlyContinue) -or -not (Get-Command npm.cmd -ErrorAction SilentlyContinue)) { return $false }
    & node.exe -e 'if(Number(process.versions.node.split(String.fromCharCode(46))[0])<22)process.exit(1)' 2>$null
    return ($LASTEXITCODE -eq 0)
}
function Test-Python {
    if (-not (Get-Command python.exe -ErrorAction SilentlyContinue)) { return $false }
    & python.exe -c 'import sys; sys.exit(0 if sys.version_info >= (3,11) else 1)' 2>$null
    return ($LASTEXITCODE -eq 0)
}
$vswhere = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft Visual Studio\Installer\vswhere.exe'
function Find-MSVC {
    if (Test-Path $vswhere) {
        & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1
    }
}
Refresh-Path
if (-not (Test-Node)) { Install-Dependency 'OpenJS.NodeJS.LTS' }
if (-not (Test-Node)) { throw 'Node.js 22+ and npm not detected after installation.' }
if (-not (Test-Python)) { Install-Dependency 'Python.Python.3.13' }
if (-not (Test-Python)) { throw 'Python 3.11+ not detected after installation.' }
# Install MSVC before rustup, to avoid rustup's interactive missing-toolchain prompt.
if (-not (Find-MSVC)) {
    Install-Dependency 'Microsoft.VisualStudio.2022.BuildTools' @('--override','--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended')
}
$vs=Find-MSVC
if (-not $vs) { throw 'MSVC x64 build tools not detected. Install the Desktop development with C++ workload and Windows SDK, then rerun.' }
$vcvars=Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "Missing MSVC environment script: $vcvars" }
# Import the native x64 developer environment, including link.exe and SDK paths.
$env:PT_VCVARS=$vcvars
$envScript=Join-Path (Get-Location) '.pt-vcvars-env.cmd'
try {
    Set-Content -LiteralPath $envScript -Encoding ASCII -Value @('@echo off','call "%PT_VCVARS%" >nul','if errorlevel 1 exit /b 1','set')
    $envLines=& cmd.exe /d /c .\.pt-vcvars-env.cmd
    if ($LASTEXITCODE -ne 0) { throw 'Unable to initialize the MSVC x64 environment.' }
} finally {
    Remove-Item -LiteralPath $envScript -Force -ErrorAction SilentlyContinue
    Remove-Item Env:PT_VCVARS -ErrorAction SilentlyContinue
}
foreach ($line in $envLines) {
    if ($line -match '^([^=]+)=(.*)$' -and $Matches[1] -ne 'PT_VCVARS') {
        [Environment]::SetEnvironmentVariable($Matches[1],$Matches[2],'Process')
    }
}
Refresh-Path
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue) -or -not (Get-Command link.exe -ErrorAction SilentlyContinue) -or -not $env:WindowsSdkDir) { throw 'Incomplete MSVC/Windows SDK environment. Modify Visual Studio Build Tools to add the C++ workload and Windows SDK.' }
if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) { Install-Dependency 'Rustlang.Rustup' }
if (-not (Get-Command rustup.exe -ErrorAction SilentlyContinue)) { throw 'Rustup not detected after installation.' }
Write-Host 'Windows build prerequisites ready.'
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
