# SPDX-License-Identifier: MIT
# Parse scripts without executing them; compatible with Windows PowerShell 5.1.
$ErrorActionPreference = 'Stop'
$failed = $false
foreach ($file in (Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1' -File)) {
    $tokens = $null
    $errors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseFile($file.FullName, [ref]$tokens, [ref]$errors)
    if ($errors.Count -gt 0) {
        $failed = $true
        foreach ($problem in $errors) {
            Write-Host ("{0}:{1}:{2}: {3}" -f $file.Name,$problem.Extent.StartLineNumber,$problem.Extent.StartColumnNumber,$problem.Message)
        }
    } else {
        Write-Host ("PowerShell syntax OK: {0}" -f $file.Name)
    }
}
if ($failed) { exit 1 }
exit 0
