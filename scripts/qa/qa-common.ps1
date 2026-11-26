# Shared helpers for the QA scripts (dot-sourced, never run directly).
Set-StrictMode -Version 3.0
$ErrorActionPreference = 'Stop'

$QaMarker = '.devdeck-qa-root'
$QaDefaultRoot = Join-Path $env:TEMP 'devdeck-qa'

function Resolve-QaRoot([string]$Root) {
    $full = [IO.Path]::GetFullPath($Root).TrimEnd('\')
    if ($full -match '~\d') { throw "Ruta con nombre corto 8.3 ($full): pasa -Raiz con la ruta larga." }
    $forbidden = @($env:TEMP, $env:USERPROFILE, $env:APPDATA, $env:LOCALAPPDATA) |
        ForEach-Object { [IO.Path]::GetFullPath($_).TrimEnd('\') }
    $isAncestor = $forbidden | Where-Object { $_ -eq $full -or $_.StartsWith("$full\", 'OrdinalIgnoreCase') }
    if ($isAncestor -or $full.Length -le 3) { throw "Raíz de QA no permitida: $full" }
    $full
}

# A recorded process is only "ours" when PID and start time both match (PID reuse).
function Get-QaProcess($Record) {
    $p = Get-Process -Id $Record.pid -ErrorAction Ignore
    if (-not $p) { return $null }
    try { $ticks = $p.StartTime.ToUniversalTime().Ticks } catch { return $null }
    if ([Math]::Abs($ticks - [long]$Record.inicio) -gt [TimeSpan]::TicksPerSecond) { return $null }
    $p
}

function New-QaProcessRecord([Diagnostics.Process]$Process, [hashtable]$Extra = @{}) {
    $r = [ordered]@{ pid = $Process.Id; inicio = $Process.StartTime.ToUniversalTime().Ticks }
    foreach ($k in $Extra.Keys) { $r[$k] = $Extra[$k] }
    [pscustomobject]$r
}

function Read-QaJson([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return @() }
    @(Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json)
}

function Test-QaPortFree([int]$Port) {
    -not (Get-NetTCPConnection -State Listen -LocalPort $Port -ErrorAction Ignore) -and
    -not (Get-NetUDPEndpoint -LocalPort $Port -ErrorAction Ignore)
}

function Assert-DevDeckClosed {
    $running = @(Get-Process -Name devdeck -ErrorAction Ignore)
    if ($running.Count) {
        throw "DevDeck está abierto (PID $($running.Id -join ', ')). Ciérralo antes (qa-app.ps1 -Accion parar)."
    }
}
