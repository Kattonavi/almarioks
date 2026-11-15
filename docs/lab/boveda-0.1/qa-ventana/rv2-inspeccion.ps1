# Reverificacion (2/3), con la app cerrada: todo lo que WebView2 dejo en disco tras el recorrido con canarios nuevos.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rv-lib.ps1'), [Text.Encoding]::UTF8)))
if ((Get-AppProc).Count -gt 0) { throw 'La app debe estar cerrada' }
$secrets = Rv-NewSecrets
$start = [datetime](Get-Content (Join-Path $Work 'inicio.txt'))
$since = [string][DateTimeOffset]::new($start).ToUnixTimeSeconds()
Step 'RV5 Control positivo con los canarios nuevos' {
    $dir = Join-Path $Work 'control'
    New-Item -ItemType Directory -Force $dir | Out-Null
    $sha = [Security.Cryptography.SHA1]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($RvMaster))
    $big = [byte[]]$sha[0..7]; [Array]::Reverse($big)
    $dec = [BitConverter]::ToUInt64($big, 0).ToString()
    [IO.File]::WriteAllBytes((Join-Path $dir 'plantado.bin'), [byte[]]([Text.Encoding]::UTF8.GetBytes("aa $($Rv.Title) bb $dec cc") + [Text.Encoding]::Unicode.GetBytes("dd $($Rv.Notes) ee")))
    $lines = Run-Inspector $secrets @("control=$dir")
    Remove-Item -LiteralPath $dir -Recurse -Force
    $kinds = @($lines | Where-Object { $_ -like 'APARICION|*' } | ForEach-Object { ($_ -split '\|')[3] })
    Check 'CONTROL POSITIVO: ve un canario nuevo en UTF-8, otro en UTF-16LE y el SHA-1 de 8 bytes de la maestra nueva en decimal' (($kinds -contains 'titulo, en claro (UTF-8)') -and ($kinds -contains 'notas, en claro (UTF-16LE)') -and ($kinds -contains 'maestra nueva, SHA-1 8 bytes en decimal (UTF-8)')) ($kinds -join ' ; ')
}
Step 'RV6 Carpeta de datos de WebView2 completa' -Continue {
    Log "Carpeta de datos: $((Get-ChildItem $DataDir -Force | ForEach-Object { $_.Name }) -join ', ')"
    if (Test-Path $EbDir) { Log "EBWebView: $((Get-ChildItem $EbDir -Force | ForEach-Object { $_.Name }) -join ', ')" }
    $def = Join-Path $EbDir 'Default'
    if (Test-Path $def) { Log "EBWebView\Default: $((Get-ChildItem $def -Force | ForEach-Object { $_.Name }) -join ', ')" }
    $wd = Join-Path $EbDir 'Default\Web Data'
    Log "Web Data existe = $(Test-Path $wd)$(if (Test-Path $wd) { " ($((Get-Item $wd).Length) bytes, modificado $((Get-Item $wd).LastWriteTime.ToString('HH:mm:ss')))" })"
    $lines = Run-Inspector $secrets @("webview2=$DataDir") -Tree
    $script:WebLines = $lines
    Check 'D-2: ningun canario nuevo en claro en la carpeta de datos de WebView2' (@($lines | Where-Object { $_ -like 'APARICION|*en claro*' }).Count -eq 0)
    Check 'D-1: ni el SHA-1 de 8 bytes de la maestra o de su repeticion, en ninguna forma' (@($lines | Where-Object { $_ -like 'APARICION|*SHA-1*' -or $_ -like 'COINCIDE_HASH|*' }).Count -eq 0)
    $auto = @($lines | Where-Object { $_ -like 'SQLITE|*filas autofill=*' })
    $rows = 0; foreach ($l in $auto) { if ($l -match 'filas autofill=(\d+)') { $rows += [int]$Matches[1] } }
    Check 'Las tablas autofill* de todas las bases SQLite del perfil estan vacias' ($rows -eq 0) "bases con tablas autofill: $(@($auto | Where-Object { $_ -notlike '*tablas autofill=0|*' }).Count); filas en total: $rows"
}
Step 'RV7 Ajustes, carpeta de trabajo y temporales' -Continue {
    $l2 = Run-Inspector $secrets @("ajustes=$ConfigDir", "trabajo=$Work") '' @($OldCopy)
    Check 'Nada en la carpeta de ajustes' ((Hits-Of $l2 'ajustes') -eq 0)
    Check 'Nada en la carpeta de trabajo (boveda, .bak, salida del proceso)' ((Hits-Of $l2 'trabajo') -eq 0)
    $temp = Join-Path $env:LOCALAPPDATA 'Temp'
    $l3 = Run-Inspector $secrets @("temp=$temp") $since @((Join-Path $temp 'claude'), $Work)
    Check 'Nada en %TEMP% entre los archivos escritos durante la prueba (sin %TEMP%\claude ni la carpeta de trabajo)' ((Hits-Of $l3 'temp') -eq 0)
    $newDirs = @(Get-ChildItem $env:LOCALAPPDATA -Directory -Force -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $start }) + @(Get-ChildItem $temp -Directory -Force -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $start -and $_.FullName -ne $Work })
    Log "Carpetas nuevas en %LOCALAPPDATA% y %TEMP% desde el inicio: $(($newDirs | ForEach-Object { $_.Name }) -join ', ')"
    $webDirs = @(Get-ChildItem $env:LOCALAPPDATA -Directory -Recurse -Depth 2 -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'EBWebView*' -and $_.LastWriteTime -ge $start })
    Log "Carpetas EBWebView tocadas durante la prueba: $(($webDirs | ForEach-Object { Redact $_.FullName }) -join ' ; ')"
    $raw = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
    Log "settings.json: auto_lock_minutes=$($raw.auto_lock_minutes) clipboard_seconds=$($raw.clipboard_seconds) recientes=$(@($raw.recent_vaults).Count)"
}
Summary
