#Requires -Version 7.0
<#
.SYNOPSIS
  Points DevDeck's real config (%APPDATA%\dev.devdeck.app\config.json) at the QA data and puts the
  original back byte for byte. The backup lives outside the QA root. DevDeck must be closed.
#>
param(
    [Parameter(Mandatory)][ValidateSet('aplicar', 'restaurar', 'estado')][string]$Accion,
    [string]$Raiz,
    [ValidateSet('en', 'es')][string]$Idioma = 'en',
    [int]$DiasOlvido = 30,
    [switch]$SinRaiz
)
. "$PSScriptRoot\qa-common.ps1"
$root = Resolve-QaRoot ($Raiz ? $Raiz : $QaDefaultRoot)
$config = Join-Path $env:APPDATA 'dev.devdeck.app\config.json'
$backupDir = Join-Path $env:TEMP 'devdeck-qa-config-backup'
$backup = Join-Path $backupDir 'config.json'
$manifest = Join-Path $backupDir 'manifest.json'

function Get-Sha([string]$Path) {
    (Test-Path -LiteralPath $Path) ? (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash : '(no existe)'
}

switch ($Accion) {
    'aplicar' {
        Assert-DevDeckClosed
        if (-not $SinRaiz -and -not (Test-Path -LiteralPath "$root\projects")) {
            throw "No hay datos de prueba en $root : ejecuta qa-fixtures.ps1 -Accion montar."
        }
        # With a manifest already present the original is safe in the backup: never overwrite it.
        if (-not (Test-Path -LiteralPath $manifest)) {
            [IO.Directory]::CreateDirectory($backupDir) | Out-Null
            $existed = Test-Path -LiteralPath $config
            if ($existed) {
                Copy-Item -LiteralPath $config $backup -Force
                if ((Get-Sha $config) -ne (Get-Sha $backup)) { throw 'La copia de seguridad no coincide con el original.' }
            }
            @{ existed = $existed; sha256 = (Get-Sha $config) } | ConvertTo-Json | Set-Content -LiteralPath $manifest
        }
        $cfg = [ordered]@{
            baseBranch = 'main'; forgottenDays = $DiasOlvido; language = $Idioma
            scanDepth = 2; scope = @{ kind = 'global' }
        }
        if (-not $SinRaiz) { $cfg['root'] = "$root\projects" }
        [IO.Directory]::CreateDirectory((Split-Path $config)) | Out-Null
        [IO.File]::WriteAllText($config, (@{ config = $cfg } | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
        Write-Output "Config de QA aplicada ($config). Original: $((Get-Content $manifest -Raw | ConvertFrom-Json).sha256)"
    }
    'restaurar' {
        if (-not (Test-Path -LiteralPath $manifest)) { Write-Output "Nada que restaurar. Config actual: $(Get-Sha $config)"; return }
        Assert-DevDeckClosed
        $m = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
        if ($m.existed) {
            if ((Get-Sha $backup) -ne $m.sha256) { throw "La copia $backup no coincide con el hash original; no se toca nada." }
            Copy-Item -LiteralPath $backup $config -Force
        }
        elseif (Test-Path -LiteralPath $config) { Remove-Item -LiteralPath $config -Force }
        $now = Get-Sha $config
        if ($now -ne $m.sha256) { throw "Restauración incorrecta: $now != $($m.sha256). Copia en $backup." }
        Remove-Item -LiteralPath $manifest
        Write-Output "Config original restaurada y verificada: $now"
    }
    'estado' {
        Write-Output "Config: $config  sha256=$(Get-Sha $config)"
        if (Test-Path -LiteralPath $manifest) {
            Write-Output "QA APLICADA. Original sha256=$((Get-Content $manifest -Raw | ConvertFrom-Json).sha256), copia en $backup"
        }
        else { Write-Output 'Config de QA no aplicada.' }
    }
}
