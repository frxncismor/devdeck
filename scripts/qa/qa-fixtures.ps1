#Requires -Version 7.0
<#
.SYNOPSIS
  Disposable QA data for DevDeck. Everything lives under -Raiz (default %TEMP%\devdeck-qa).
  See docs/testing/qa-fixtures.md.
#>
param(
    [Parameter(Mandatory)][ValidateSet('montar', 'desmontar', 'estado')][string]$Accion,
    [string]$Raiz,
    [int]$JsonGrandeMB = 110,
    [int]$PuertoBase = 47010,
    [switch]$SinProcesos
)
. "$PSScriptRoot\qa-common.ps1"
$root = Resolve-QaRoot ($Raiz ? $Raiz : $QaDefaultRoot)
$pidsFile = Join-Path $root 'pids.json'
$utf8 = [Text.UTF8Encoding]::new($false)

function Invoke-Git([string]$Dir, [Parameter(ValueFromRemainingArguments)][string[]]$GitArgs) {
    $out = & git -C $Dir @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) { throw "git $($GitArgs -join ' ') en $Dir falló: $out" }
    $out
}

function Set-File([string]$Path, [string]$Content) {
    [IO.Directory]::CreateDirectory((Split-Path $Path)) | Out-Null
    [IO.File]::WriteAllText($Path, $Content, $utf8)
}

function Add-Filler([string]$Path, [int]$MB) {
    [IO.Directory]::CreateDirectory((Split-Path $Path)) | Out-Null
    $block = [byte[]]::new(1MB)
    $fs = [IO.File]::Create($Path)
    try { for ($i = 0; $i -lt $MB; $i++) { $fs.Write($block, 0, $block.Length) } }
    finally { $fs.Dispose() }
}

# Commit with author and committer dates $DaysAgo in the past.
function Add-Commit([string]$Dir, [string]$File, [string]$Message, [int]$DaysAgo = 0) {
    Set-File (Join-Path $Dir $File) "$Message`n"
    Invoke-Git $Dir add -A | Out-Null
    $date = [DateTimeOffset]::Now.AddDays(-$DaysAgo).ToString('yyyy-MM-ddTHH:mm:sszzz')
    $env:GIT_AUTHOR_DATE = $date
    $env:GIT_COMMITTER_DATE = $date
    try { Invoke-Git $Dir commit -q -m $Message | Out-Null }
    finally { Remove-Item Env:GIT_AUTHOR_DATE, Env:GIT_COMMITTER_DATE }
}

function New-Repo([string]$Name, [hashtable]$Files) {
    $repo = Join-Path $root "projects\$Name"
    $bare = Join-Path $root "remotes\$Name.git"
    [IO.Directory]::CreateDirectory($repo) | Out-Null
    [IO.Directory]::CreateDirectory($bare) | Out-Null
    Invoke-Git $bare init -q --bare -b main | Out-Null
    Invoke-Git $repo init -q -b main | Out-Null
    Set-File (Join-Path $repo '.gitignore') "node_modules/`ntarget/`ndist/`n.venv/`n"
    foreach ($f in $Files.Keys) { Set-File (Join-Path $repo $f) $Files[$f] }
    Add-Commit $repo 'README.md' "chore: initial $Name" 120
    Invoke-Git $repo remote add origin $bare | Out-Null
    Invoke-Git $repo push -q -u origin main | Out-Null
    $repo
}

function New-Worktree([string]$Repo, [string]$Name, [int]$DaysAgo = 0, [switch]$Push) {
    $path = "$Repo-wt\$Name"
    Invoke-Git $Repo worktree add -q -b "feature/$Name" $path main | Out-Null
    Add-Commit $path "$Name.txt" "feat: work on $Name" $DaysAgo
    if ($Push) { Invoke-Git $path push -q -u origin "feature/$Name" | Out-Null }
    $path
}

function New-Repos {
    $web = New-Repo 'webshop' @{ 'package.json' = '{ "name": "webshop", "version": "1.0.0" }'; 'src/app.js' = "console.log('webshop');`n" }

    New-Worktree $web 'limpio' -Push | Out-Null

    $p = New-Worktree $web 'sucio' -Push
    Set-File "$p\sucio.txt" "changed`n"
    Set-File "$p\README.md" "changed`n"
    Set-File "$p\nuevo-sin-seguimiento.txt" "untracked`n"

    New-Worktree $web 'sin-upstream' | Out-Null

    $p = New-Worktree $web 'adelantado' -Push
    Add-Commit $p 'a1.txt' 'feat: not pushed 1'
    Add-Commit $p 'a2.txt' 'feat: not pushed 2'

    $p = New-Worktree $web 'atrasado'
    Add-Commit $p 'b1.txt' 'feat: remote only 1'
    Add-Commit $p 'b2.txt' 'feat: remote only 2'
    Invoke-Git $p push -q -u origin feature/atrasado | Out-Null
    Invoke-Git $p reset -q --hard HEAD~2 | Out-Null

    $p = New-Worktree $web 'divergente'
    Add-Commit $p 'd1.txt' 'feat: remote side'
    Invoke-Git $p push -q -u origin feature/divergente | Out-Null
    Invoke-Git $p reset -q --hard HEAD~1 | Out-Null
    Add-Commit $p 'd2.txt' 'feat: local side'

    $p = New-Worktree $web 'olvidado' -DaysAgo 90 -Push
    Add-Filler "$p\node_modules\filler\blob.bin" 6

    $p = New-Worktree $web 'olvidado-sucio' -DaysAgo 60
    Set-File "$p\pendiente.txt" "uncommitted work`n"

    New-Worktree $web 'mergeado' -Push | Out-Null
    Invoke-Git $web merge -q --no-ff feature/mergeado -m 'merge: feature/mergeado' | Out-Null
    Add-Commit $web 'CHANGELOG.md' 'docs: recent commit on main'
    Invoke-Git $web push -q origin main | Out-Null

    Invoke-Git $web worktree add -q --detach "$web-wt\detached" main | Out-Null

    # Folder deleted by hand: git still lists it ("Missing folder").
    $p = New-Worktree $web 'borrado'
    Remove-Item -LiteralPath $p -Recurse -Force

    # Regenerable folders (positive cases) and look-alikes that must NOT be listed.
    Add-Filler "$web\node_modules\filler\blob.bin" 8
    Add-Filler "$web\dist\bundle.js" 3
    Add-Filler "$web\docs\target\not-cargo.bin" 1
    Add-Filler "$web\docs\dist\not-a-package.bin" 1
    (Get-Item "$web\dist").LastWriteTime = (Get-Date).AddDays(-200)

    $api = New-Repo 'api-b' @{ 'Cargo.toml' = "[package]`nname = `"api-b`"`nversion = `"0.1.0`"`n" }
    New-Worktree $api 'auth' | Out-Null
    New-Worktree $api 'legacy' -DaysAgo 45 | Out-Null
    Add-Filler "$api\target\debug\filler.bin" 12
    Add-Filler "$api\.venv\Lib\filler.bin" 2
    Set-File "$api\.venv\pyvenv.cfg" "home = C:\Python`n"
}

function New-FakeCaches {
    # Seen by the app only when launched through qa-app.ps1 (LOCALAPPDATA / CARGO_HOME overrides).
    Add-Filler "$root\fake-localappdata\npm-cache\_cacache\blob.bin" 3
    Add-Filler "$root\fake-localappdata\pnpm\store\v10\blob.bin" 3
    Add-Filler "$root\fake-cargo-home\registry\cache\blob.bin" 3
    Add-Filler "$root\fake-cargo-home\git\db\blob.bin" 2
}

function New-JsonFiles {
    $dir = "$root\json"
    Set-File "$dir\pequeno.json" '{"nombre":"demo","activo":true,"n":42,"nulo":null,"lista":[1,2,3],"a b":{"unicode":"canción ñ 漢字 🚀"}}'
    $depth = 200
    Set-File "$dir\anidado.json" (('{"nivel":[' * $depth) + '"fondo"' + (']}' * $depth))
    $bad = @{
        'invalido-truncado'         = '{"a": [1, 2, 3'
        'invalido-coma-final'       = '{"a": 1, "b": 2,}'
        'invalido-comillas-simples' = "{'a': 1}"
        'invalido-clave-sin-comillas' = '{a: 1}'
        'invalido-basura-final'     = '{"a": 1} extra'
        'invalido-nan'              = '{"a": NaN}'
        'invalido-escape'           = '{"a": "\q"}'
        'invalido-vacio'            = ''
        'invalido-tarde'            = '{"items":[' + ((1..20000 | ForEach-Object { "{`"id`":$_}" }) -join ',') + ',oops]}'
    }
    foreach ($k in $bad.Keys) { Set-File "$dir\$k.json" $bad[$k] }

    if ($JsonGrandeMB -gt 0) {
        $target = [long]$JsonGrandeMB * 1MB
        $w = [IO.StreamWriter]::new("$dir\grande.json", $false, $utf8, 1MB)
        try {
            $w.Write('{"meta":{"generator":"qa-fixtures"},"orders":[')
            $pad = 'lorem ipsum dolor sit amet ' * 4
            for ($i = 0; ; $i++) {
                if ($i) { $w.Write(',') }
                $w.Write("{`"id`":$i,`"customer`":`"customer-$($i % 5000)`",`"total`":$($i % 997).5,`"paid`":$(($i % 3) ? 'true' : 'false'),`"tags`":[`"qa`",`"t$($i % 10)`"],`"note`":`"$pad`"}")
                if (($i % 2000) -eq 0 -and $w.BaseStream.Length -ge $target) { break }
            }
            $w.Write('],"needle":"aguja-al-final"}')
        }
        finally { $w.Dispose() }
    }
}

function Start-Fixtures {
    $node = (Get-Command node -CommandType Application | Select-Object -First 1).Source
    $bin = "$root\bin"
    Set-File "$bin\server.js" @'
const [port, proto] = [Number(process.argv[2]), process.argv[3]];
if (proto === "udp") require("dgram").createSocket("udp4").bind(port, "127.0.0.1");
else require("http").createServer((_, res) => res.end("devdeck-qa " + port)).listen(port, "127.0.0.1");
'@
    # Renamed copies of node.exe so Reclaim shows other dev-process kinds.
    Copy-Item -LiteralPath $node "$bin\vite.exe"
    Copy-Item -LiteralPath $node "$bin\rust-analyzer.exe"
    [IO.Directory]::CreateDirectory("$root\loose") | Out-Null

    $web = "$root\projects\webshop"
    $api = "$root\projects\api-b"

    # Orphan: Windows will not delete a folder that is a process cwd, so the worktree is a junction
    # while the process starts and the junction is removed afterwards.
    $orphan = New-Worktree $web 'huerfano'
    $store = "$root\orphan-store"
    Move-Item -LiteralPath $orphan $store
    New-Item -ItemType Junction -Path $orphan -Target $store | Out-Null

    $specs = @(
        @{ puerto = $PuertoBase; exe = $node; cwd = $web; proto = 'tcp'; caso = 'node en el repo principal webshop' }
        @{ puerto = $PuertoBase + 1; exe = $node; cwd = "$web-wt\limpio"; proto = 'tcp'; caso = 'node en worktree limpio' }
        @{ puerto = $PuertoBase + 2; exe = "$bin\vite.exe"; cwd = "$web-wt\olvidado"; proto = 'tcp'; caso = 'vite en worktree olvidado (Processes running)' }
        @{ puerto = $PuertoBase + 3; exe = $node; cwd = $orphan; proto = 'tcp'; caso = 'node huérfano (Deleted worktree)' }
        @{ puerto = $PuertoBase + 4; exe = "$bin\rust-analyzer.exe"; cwd = $api; proto = 'tcp'; caso = 'rust-analyzer en api-b' }
        @{ puerto = $PuertoBase + 5; exe = $node; cwd = "$root\loose"; proto = 'tcp'; caso = 'node fuera de proyectos (No project)' }
        @{ puerto = $PuertoBase + 6; exe = $node; cwd = "$api-wt\auth"; proto = 'udp'; caso = 'node UDP en api-b/auth' }
    )
    $records = @()
    foreach ($s in $specs) {
        if (-not (Test-QaPortFree $s.puerto)) { throw "Puerto $($s.puerto) ocupado: usa -PuertoBase." }
        $p = Start-Process -FilePath $s.exe -ArgumentList "`"$bin\server.js`" $($s.puerto) $($s.proto)" `
            -WorkingDirectory $s.cwd -WindowStyle Hidden -PassThru
        $records += New-QaProcessRecord $p @{ puerto = $s.puerto; proto = $s.proto; cwd = $s.cwd; nombre = (Split-Path $s.exe -Leaf); caso = $s.caso }
        # Saved after every start so desmontar can always find them.
        ConvertTo-Json @($records) | Set-Content -LiteralPath $pidsFile
    }
    $deadline = (Get-Date).AddSeconds(20)
    do {
        Start-Sleep -Milliseconds 300
        $pending = @($specs | Where-Object { Test-QaPortFree $_.puerto })
    } while ($pending.Count -and (Get-Date) -lt $deadline)
    if ($pending.Count) { throw "No escuchan los puertos: $($pending.puerto -join ', ')" }

    [IO.Directory]::Delete($orphan)   # removes the junction only, not its target
    if (Test-Path -LiteralPath $orphan) { throw "No se pudo quitar el junction $orphan" }
}

function Mount-Fixtures {
    if (Test-Path -LiteralPath $root) { throw "$root ya existe: ejecuta -Accion desmontar primero." }
    [IO.Directory]::CreateDirectory($root) | Out-Null
    Set-File (Join-Path $root $QaMarker) "DevDeck QA fixtures. Safe to delete with qa-fixtures.ps1 -Accion desmontar.`n"

    # Isolate git from the user's global config (identity, signing, hooks, LFS).
    $gitconfig = Join-Path $root 'gitconfig'
    Set-File $gitconfig "[user]`n`tname = QA Fixture`n`temail = qa@example.invalid`n[core]`n`tautocrlf = false`n"
    $previous = $env:GIT_CONFIG_GLOBAL
    $env:GIT_CONFIG_GLOBAL = $gitconfig
    try {
        New-Repos
        if (-not $SinProcesos) { Start-Fixtures }
    }
    finally { $env:GIT_CONFIG_GLOBAL = $previous }
    New-FakeCaches
    New-JsonFiles
    Show-State
}

function Dismount-Fixtures {
    if (-not (Test-Path -LiteralPath $root)) { Write-Output "Nada que desmontar: $root no existe."; return }
    if (-not (Test-Path -LiteralPath (Join-Path $root $QaMarker))) {
        throw "$root no tiene la marca $QaMarker : no es una raíz de QA, no se borra."
    }
    $appAlive = @(Read-QaJson (Join-Path $root 'app-pids.json') | Where-Object { Get-QaProcess $_ })
    if ($appAlive.Count) { throw 'La app de QA sigue en marcha: qa-app.ps1 -Accion parar.' }

    foreach ($r in Read-QaJson $pidsFile) {
        $p = Get-QaProcess $r
        if ($p) { Stop-Process -InputObject $p -Force; $p.WaitForExit(10000) | Out-Null }
    }
    # Links are removed first, without following them.
    Get-ChildItem -LiteralPath $root -Recurse -Force -Directory -Attributes ReparsePoint -ErrorAction Ignore |
        ForEach-Object { [IO.Directory]::Delete($_.FullName) }
    for ($try = 1; ; $try++) {
        try { Remove-Item -LiteralPath $root -Recurse -Force; break }
        catch {
            if ($try -ge 5) { throw "No se pudo borrar $root (¿terminal o VS Code abierto dentro?): $_" }
            Start-Sleep -Seconds 1
        }
    }
    Write-Output "Desmontado: $root"
}

function Show-State {
    if (-not (Test-Path -LiteralPath $root)) { Write-Output "Sin montar: $root no existe."; return }
    Write-Output "Raíz: $root  (raíz de proyectos para la app: $root\projects)"
    foreach ($repo in Get-ChildItem -LiteralPath "$root\projects" -Directory | Where-Object { Test-Path "$($_.FullName)\.git" -PathType Container }) {
        Write-Output "`n== $($repo.Name)"
        Invoke-Git $repo.FullName worktree list
    }
    Write-Output "`n== Procesos"
    Read-QaJson $pidsFile | ForEach-Object {
        [pscustomobject]@{
            Puerto = $_.puerto; Proto = $_.proto; PID = $_.pid; Nombre = $_.nombre
            Vivo = [bool](Get-QaProcess $_); Caso = $_.caso
        }
    } | Format-Table -AutoSize | Out-String -Width 200
    Write-Output '== JSON'
    Get-ChildItem -LiteralPath "$root\json" -ErrorAction Ignore |
        Select-Object Name, Length | Format-Table -AutoSize | Out-String
}

switch ($Accion) {
    'montar' { Mount-Fixtures }
    'desmontar' { Dismount-Fixtures }
    'estado' { Show-State }
}
