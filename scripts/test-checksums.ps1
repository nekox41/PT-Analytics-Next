# SPDX-License-Identifier: MIT
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'checksums.ps1')
$hash = 'a' * 64
$temp = [System.IO.Path]::GetTempFileName()
try {
    # A byte-backed checksum file (the failing HTTP response was binary).
    [System.IO.File]::WriteAllBytes($temp, [System.Text.Encoding]::ASCII.GetBytes("$hash  rustup-init.exe" + [Environment]::NewLine))
    if ((Get-RustupSha256 (Read-ChecksumText $temp)) -ne $hash) { throw 'Rustup checksum file regression.' }
    [System.IO.File]::WriteAllBytes($temp, [System.Text.Encoding]::ASCII.GetBytes($hash))
    if ((Get-RustupSha256 (Read-ChecksumText $temp)) -ne $hash) { throw 'Bare checksum regression.' }
    $node = Get-NodeZipMatch ("$hash  node-v22.23.5-win-x64.zip" + [Environment]::NewLine)
    if ($node.Groups[1].Value -ne $hash -or $node.Groups[2].Value -ne 'node-v22.23.5-win-x64.zip') { throw 'Node checksum manifest regression.' }
    foreach ($bad in @('not a checksum',"$hash  other.exe",('a' * 63))) {
        $rejected = $false
        try { $null = Get-RustupSha256 $bad } catch { $rejected = $true }
        if (-not $rejected) { throw 'Invalid checksum accepted.' }
    }
    Write-Host 'Checksum regression tests OK.'
} finally {
    Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
}
