# SPDX-License-Identifier: MIT
# Read checksum files as text, independent of HTTP Content-Type and PowerShell version.
function Read-ChecksumText([string]$Path) {
    return [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::ASCII)
}
function Get-RustupSha256([string]$Text) {
    $match = [regex]::Match($Text, '\A\s*([a-fA-F0-9]{64})(?:[ \t]+\*?rustup-init\.exe)?\s*\z')
    if (-not $match.Success) { throw 'Invalid Rustup SHA256 manifest.' }
    return $match.Groups[1].Value
}
function Get-NodeZipMatch([string]$Text) {
    $match = [regex]::Match($Text, '(?m)^([a-fA-F0-9]{64})[ \t]+\*?(node-v22\.\d+\.\d+-win-x64\.zip)[ \t\r]*$')
    if (-not $match.Success) { throw 'Node.js x64 ZIP not found in official checksum manifest.' }
    return $match
}
