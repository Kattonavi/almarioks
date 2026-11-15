# Pase de interaccion (8), con la app cerrada: rastro en disco al final del recorrido. Agujas: los canarios FAROPX,
# las dos contrasenas maestras de prueba, datos de muestra y TODAS las contrasenas generadas que estuvieron a la vista
# en el campo del dialogo. Carpetas: perfil de WebView2, ajustes, carpeta de trabajo, la carpeta de Familia y %TEMP%
# (solo lo escrito desde el inicio de la corrida). Con control positivo del inspector y del filtro por fecha.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
if ((Any-AlmarioProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 3 }
if ((Any-AlmarioProc).Count -gt 0) { throw 'La app debe estar cerrada' }
$secrets = Rf-Secrets
$since = Start-Epoch
$gen = @((Get-State 'generadas') -split "`n" | Where-Object { $_ })
Log "Agujas en claro: canarios + datos de muestra + $($gen.Count) contrasenas generadas vistas en el dialogo; con huella SHA-1: 2 maestras y la repeticion"
Step 'PX80 Control positivo del inspector' {
    New-Item -ItemType Directory -Force $Work | Out-Null
    $dir = Join-Path $Work 'control'
    New-Item -ItemType Directory -Force $dir | Out-Null
    $sha = [Security.Cryptography.SHA1]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($RfMaster))
    $big = [byte[]]$sha[0..7]; [Array]::Reverse($big)
    $dec = [BitConverter]::ToUInt64($big, 0).ToString()
    $g1 = if ($gen.Count) { $gen[0] } else { 'sin-generadas' }
    [IO.File]::WriteAllBytes((Join-Path $dir 'plantado.bin'), [byte[]]([Text.Encoding]::UTF8.GetBytes("aa $($Rf.Title) bb $dec cc $g1 dd") + [Text.Encoding]::Unicode.GetBytes("ee $($Rf.Notes) ff")))
    $env:RV_SECRETS = $secrets
    $planted = @(& py -3 -I (Join-Path $QaDir 'rf-plantar-control.py') $dir 2>&1 | ForEach-Object { "$_" })
    foreach ($l in $planted) { Log "   $l" }
    $lines = Run-Inspector $secrets @("control=$dir")
    Remove-Item -LiteralPath $dir -Recurse -Force
    $kinds = @($lines | Where-Object { $_ -like 'APARICION|*' } | ForEach-Object { ($_ -split '\|')[3] })
    Check 'CONTROL POSITIVO: ve un canario en UTF-8, otro en UTF-16LE, una contrasena generada y el SHA-1 de 8 bytes de la maestra' (($kinds -contains 'titulo, en claro (UTF-8)') -and ($kinds -contains 'notas, en claro (UTF-16LE)') -and ($kinds -contains 'contrasena generada 1, en claro (UTF-8)') -and ($kinds -contains 'maestra, SHA-1 8 bytes en decimal (UTF-8)')) "$($kinds.Count) apariciones"
    $sq = @($lines | Where-Object { $_ -like 'SQLITE|control|Web Data|*autofill_edge_field_values=2*' -or $_ -like 'SQLITE|control|Web Data|*en claro=1, enmascaradas=1*' }).Count
    $hh = @($lines | Where-Object { $_ -like 'COINCIDE_HASH|control|*maestra*' }).Count
    Check 'CONTROL POSITIVO: en una base SQLite como Web Data cuenta las filas autofill y reconoce la fila enmascarada' ($sq -eq 2 -and $hh -eq 1) "lineas de conteo: $sq de 2; coincidencias de hash: $hh"
}
Step 'PX81 Perfil de WebView2' -Continue {
    $wd = Join-Path $EbDir 'Default\Web Data'
    Log "EBWebView existe = $(Test-Path $EbDir); Web Data existe = $(Test-Path $wd)"
    $lines = Run-Inspector $secrets @("webview2=$DataDir") -Tree
    Check '(iii) Perfil de WebView2: 0 apariciones en claro' (@($lines | Where-Object { $_ -like 'APARICION|*en claro*' }).Count -eq 0 -and (Hits-Of $lines 'webview2') -eq 0) "apariciones: $(Hits-Of $lines 'webview2'); $(($lines | Where-Object { $_ -like 'RESUMEN|webview2|*' }) -join '')"
    Check '(iii) Perfil de WebView2: ninguna forma del SHA-1 de las maestras' (@($lines | Where-Object { $_ -like 'APARICION|*SHA-1*' -or $_ -like 'COINCIDE_HASH|*' }).Count -eq 0)
    $auto = @($lines | Where-Object { $_ -like 'SQLITE|*filas autofill=*' })
    $rows = 0; foreach ($l in $auto) { if ($l -match 'filas autofill=(\d+)') { $rows += [int]$Matches[1] } }
    $bad = @($lines | Where-Object { $_ -like 'SQLITE|*no se pudo abrir*' -or $_ -like 'SQLITE|*error al leer*' })
    Check '(iii) Las tablas autofill* de las bases SQLite del perfil estan vacias y todas se pudieron leer' ($rows -eq 0 -and $bad.Count -eq 0) "bases SQLite: $($auto.Count); filas autofill: $rows; sin leer: $($bad.Count)"
}
Step 'PX82 Ajustes, carpeta de trabajo, carpeta de Familia y temporales' -Continue {
    $roots = @("ajustes=$ConfigDir", "trabajo=$Work")
    if (Test-Path $FamDir) { $roots += "familia=$FamDir" }
    $l2 = Run-Inspector $secrets $roots
    Check '(iii) Ajustes: 0 apariciones' ((Hits-Of $l2 'ajustes') -eq 0) "apariciones: $(Hits-Of $l2 'ajustes')"
    Check '(iii) Carpeta de trabajo (boveda, .bak, salida del proceso): 0 apariciones' ((Hits-Of $l2 'trabajo') -eq 0) "apariciones: $(Hits-Of $l2 'trabajo'); archivos: $((Get-ChildItem $Work -Force | ForEach-Object { $_.Name }) -join ', ')"
    if (Test-Path $FamDir) { Check '(iii) Carpeta de Familia: 0 apariciones' ((Hits-Of $l2 'familia') -eq 0) "apariciones: $(Hits-Of $l2 'familia')" }
    $temp = Join-Path $env:LOCALAPPDATA 'Temp'
    $plant = Join-Path $temp 'almarioks-px-control-plantado.bin'
    [IO.File]::WriteAllBytes($plant, [Text.Encoding]::UTF8.GetBytes("xx $($Rf.Group) yy"))
    $l3 = Run-Inspector $secrets @("temp=$temp") $since @((Join-Path $temp 'claude'), $Work)
    Remove-Item -LiteralPath $plant -Force
    $own = @($l3 | Where-Object { $_ -like 'APARICION|temp|almarioks-px-control-plantado.bin|*' }).Count
    $other = @($l3 | Where-Object { $_ -like 'APARICION|temp|*' -and $_ -notlike 'APARICION|temp|almarioks-px-control-plantado.bin|*' }).Count
    Check 'CONTROL POSITIVO en %TEMP%: el archivo plantado ahora si aparece' ($own -ge 1) "apariciones del plantado: $own"
    Check '(iii) %TEMP%: 0 apariciones entre lo escrito durante la corrida (sin %TEMP%\claude ni la carpeta de trabajo)' ($other -eq 0 -and (Hits-Of $l3 'temp') -eq $own) "fuera del plantado: $other; $(($l3 | Where-Object { $_ -like 'RESUMEN|temp|*' }) -join '')"
    $raw = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
    Log "settings.json: claves = $(($raw.PSObject.Properties.Name) -join ', '); auto_lock_minutes=$($raw.auto_lock_minutes) clipboard_seconds=$($raw.clipboard_seconds) recientes=$(@($raw.recent_vaults).Count)"
}
Summary
