param([string]$Python = "", [string]$State = "$env:LOCALAPPDATA\Schema-s\updates")
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
$arguments = @('"' + $entry + '"', 'serve', '--state', '"' + $State + '"')
Start-Process -FilePath $Python -ArgumentList $arguments -WorkingDirectory (Split-Path $PSScriptRoot) -WindowStyle Hidden -RedirectStandardOutput (Join-Path $State 'server.log') -RedirectStandardError (Join-Path $State 'server-error.log') | Out-Null
Write-Output $url
