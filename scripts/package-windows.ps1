$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
cargo build --release --locked
if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed' }
New-Item -ItemType Directory -Force -Path dist | Out-Null
Copy-Item target/release/clickup-tracker.exe 'dist/ClickUp Tracker.exe' -Force
$targetTriple = (rustc -vV | Select-String '^host: ').ToString().Substring(6)
Compress-Archive -Path 'dist/ClickUp Tracker.exe', LICENSE, README.md -DestinationPath "dist/clickup-tracker-$targetTriple.zip" -Force
Write-Host 'Ready: dist/ClickUp Tracker.exe'
