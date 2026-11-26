#Requires -Version 7.0
<#
.SYNOPSIS
  Starts / stops DevDeck for QA with WebView2 remote debugging (CDP), a disposable WebView2 profile
  and fake package-manager caches. Requires qa-fixtures.ps1 montar and qa-config.ps1 aplicar.
#>
param(
    [Parameter(Mandatory)][ValidateSet('arrancar', 'parar', 'estado')][string]$Accion,
    [ValidateSet('dev', 'release')][string]$Build = 'dev',
    [string]$Exe,
    [string]$Raiz,
    [int]$PuertoCdp = 9222,
    [switch]$SinCompilar
)
. "$PSScriptRoot\qa-common.ps1"
$root = Resolve-QaRoot ($Raiz ? $Raiz : $QaDefaultRoot)
$repo = (Resolve-Path "$PSScriptRoot\..\..").Path
$pidsFile = Join-Path $root 'app-pids.json'
$targetDir = $env:CARGO_TARGET_DIR ? $env:CARGO_TARGET_DIR : (Join-Path $repo 'target')

function Wait-Http([string]$Url, [int]$Seconds) {
    $deadline = (Get-Date).AddSeconds($Seconds)
    while ((Get-Date) -lt $deadline) {
        try { Invoke-WebRequest -Uri $Url -TimeoutSec 2 -UseBasicParsing | Out-Null; return $true }
        catch { Start-Sleep -Milliseconds 500 }
    }
    $false
}

function Save-Records($Records) { ConvertTo-Json @($Records) | Set-Content -LiteralPath $pidsFile }

function Stop-App {
    $records = @(Read-QaJson $pidsFile)
    foreach ($r in $records | Where-Object rol -EQ 'app') {
        $p = Get-QaProcess $r
        if (-not $p) { continue }
        $p.CloseMainWindow() | Out-Null
        if (-not $p.WaitForExit(8000)) { Stop-Process -InputObject $p -Force; $p.WaitForExit(5000) | Out-Null }
    }
    foreach ($r in $records | Where-Object rol -EQ 'vite') {
        if (Get-QaProcess $r) { & taskkill.exe /PID $r.pid /T /F | Out-Null }
    }
    # WebView2 children that outlive the app, identified by the disposable profile path.
    Start-Sleep -Seconds 1
    Get-CimInstance Win32_Process -Filter "Name = 'msedgewebview2.exe'" |
        Where-Object { $_.CommandLine -and $_.CommandLine.Contains("$root\webview2") } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction Ignore }
    if (Test-Path -LiteralPath $pidsFile) { Remove-Item -LiteralPath $pidsFile }
}

function Start-App {
    if (-not (Test-Path -LiteralPath (Join-Path $root $QaMarker))) { throw 'Monta antes los datos: qa-fixtures.ps1 -Accion montar.' }
    if (-not (Test-Path -LiteralPath "$env:TEMP\devdeck-qa-config-backup\manifest.json")) {
        throw 'La config de QA no está aplicada (qa-config.ps1 -Accion aplicar): la app leería datos reales.'
    }
    Assert-DevDeckClosed
    $free = [int]((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1024)
    if ($free -lt 4000) { throw "Memoria libre insuficiente: $free MB (< 4000)." }
    if (-not (Test-QaPortFree $PuertoCdp)) { throw "Puerto CDP $PuertoCdp ocupado." }

    $records = @()
    $logs = Join-Path $root 'logs'
    [IO.Directory]::CreateDirectory($logs) | Out-Null

    if ($Build -eq 'dev') {
        if (-not (Test-QaPortFree 1420)) { throw 'Puerto 1420 ocupado (¿otro vite de DevDeck?).' }
        if (-not $SinCompilar) {
            Push-Location $repo
            try {
                & cargo build -p devdeck
                if ($LASTEXITCODE -ne 0) { throw 'cargo build falló.' }
            }
            finally { Pop-Location }
        }
        $vite = Start-Process -FilePath cmd.exe -ArgumentList "/c pnpm dev > `"$logs\vite.log`" 2>&1" `
            -WorkingDirectory $repo -WindowStyle Hidden -PassThru
        $records += New-QaProcessRecord $vite @{ rol = 'vite' }
        Save-Records $records
        if (-not (Wait-Http 'http://localhost:1420' 60)) { Stop-App; throw "vite no responde; ver $logs\vite.log" }
        if (-not $Exe) { $Exe = Join-Path $targetDir 'debug\devdeck.exe' }
    }
    elseif (-not $Exe) { $Exe = Join-Path $targetDir 'release\devdeck.exe' }

    if (-not (Test-Path -LiteralPath $Exe)) { Stop-App; throw "No existe $Exe (release: pnpm tauri build --no-bundle)." }

    $overrides = @{
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$PuertoCdp"
        WEBVIEW2_USER_DATA_FOLDER             = "$root\webview2"
        LOCALAPPDATA                          = "$root\fake-localappdata"
        CARGO_HOME                            = "$root\fake-cargo-home"
    }
    $previous = @{}
    foreach ($k in $overrides.Keys) {
        $previous[$k] = [Environment]::GetEnvironmentVariable($k)
        [Environment]::SetEnvironmentVariable($k, $overrides[$k])
    }
    try { $app = Start-Process -FilePath $Exe -WorkingDirectory $root -PassThru }
    finally {
        foreach ($k in $previous.Keys) { [Environment]::SetEnvironmentVariable($k, $previous[$k]) }
    }
    $records += New-QaProcessRecord $app @{ rol = 'app'; exe = $Exe }
    Save-Records $records
    if (-not (Wait-Http "http://127.0.0.1:$PuertoCdp/json/version" 60)) { Stop-App; throw 'CDP no responde.' }
    Write-Output "DevDeck ($Build) PID $($app.Id), CDP en http://127.0.0.1:$PuertoCdp"
}

switch ($Accion) {
    'arrancar' { Start-App }
    'parar' {
        Stop-App
        Write-Output 'App de QA parada. Recuerda: qa-config.ps1 -Accion restaurar.'
    }
    'estado' {
        Read-QaJson $pidsFile | ForEach-Object { [pscustomobject]@{ Rol = $_.rol; PID = $_.pid; Vivo = [bool](Get-QaProcess $_) } }
    }
}
