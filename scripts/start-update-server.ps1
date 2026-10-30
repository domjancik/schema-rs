param([string]$Python = "", [string]$State = "$env:LOCALAPPDATA\Schema-s\updates", [switch]$Foreground)
$ErrorActionPreference = 'Stop'
$entry = Join-Path $PSScriptRoot 'update_server\main.py'
if (!$Python) {
    $savedConfig = Join-Path $State 'config.json'
    if (Test-Path -LiteralPath $savedConfig) { $Python = (Get-Content -LiteralPath $savedConfig -Raw | ConvertFrom-Json).python }
    if (!$Python) { $Python = (Get-Command python -ErrorAction Stop).Source }
}
if (!(Test-Path -LiteralPath $Python)) { throw 'Provide -Python with a Python 3.11+ environment containing paramiko and cryptography.' }
$env:PYTHONUTF8 = '1'
New-Item -ItemType Directory -Force -Path $State | Out-Null
& $Python $entry init --state $State
if ($LASTEXITCODE -ne 0) { throw 'Update server initialization failed' }
$config = Get-Content -LiteralPath (Join-Path $State 'config.json') -Raw | ConvertFrom-Json
$url = "http://127.0.0.1:$($config.port)/"
try { $health = Invoke-RestMethod ($url + 'health') -TimeoutSec 2; if ($health.service -eq 'schema-update-server') { Write-Output $url; exit 0 } } catch {}
if ($Foreground) {
    while ($true) {
        & $Python $entry serve --state $State *>> (Join-Path $State 'supervisor.log')
        $result = $LASTEXITCODE
        if ($result -ne 75) { exit $result }
    }
}
$arguments = @('-NoProfile', '-NonInteractive', '-WindowStyle', 'Hidden', '-File', '"' + $PSCommandPath + '"', '-Python', '"' + $Python + '"', '-State', '"' + $State + '"', '-Foreground')
Start-Process -FilePath 'powershell.exe' -ArgumentList $arguments -WorkingDirectory (Split-Path $PSScriptRoot) -WindowStyle Hidden | Out-Null
Write-Output $url
