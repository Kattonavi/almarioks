# Ronda 2, puntos 1 a 4 (9): limpieza final. Con la app cerrada borra la carpeta temporal propia (Familia, SinLlave)
# y la de la boveda de muestra "Cuentas" (ya no hay agente siguiente); abre la app y quita de "recientes" SOLO las
# tres bovedas de prueba, por su nombre exacto (las demas no se tocan ni se nombran); comprueba ajustes (5 min, 20 s,
# sin llaves recordadas de prueba), portapapeles e historial, y deja la app cerrada. RF_VARIANT=pz-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$mine = @('Familia.kdbx', 'SinLlave.kdbx', 'Cuentas.kdbx')
Step 'PZ90 Limpieza' {
    if ((Any-AlmarioProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 3 }
    if ((Any-AlmarioProc).Count -gt 0) { throw 'Quedan procesos almarioks* vivos' }
    if ($Pz -ne (Join-Path $env:PUBLIC 'almarioks-pz-20261112') -or $Work -ne (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-py-20261112')) { throw 'Las carpetas no son las esperadas: no se borra nada' }
    foreach ($d in @($Pz, $Work)) { if (Test-Path -LiteralPath $d) { Remove-Item -LiteralPath $d -Recurse -Force } }
    Start-Checked 'limpieza'; Start-Sleep -Milliseconds 900
    foreach ($name in $mine) {
        $btn = Find-El -Like "Quitar $name*" -Type Button -Timeout 2 -Quiet
        if ($null -ne $btn) { Log "   reciente $name : boton `"$($btn.Current.Name)`""; Click-El $btn; Start-Sleep -Milliseconds 800 }
        else { Log "   reciente $name : no hay boton para quitarla (botones con ese nombre: $((@(Get-Names | Where-Object { $_ -like "*$name*" }) | Select-Object -Unique) -join ' | '))" }
    }
    $left = @(Get-Names | Where-Object { $n = $_; @($mine | Where-Object { $n -like "*$_*" }).Count -gt 0 }).Count
    Check 'Recientes de prueba quitadas desde la app' ($left -eq 0) "nombres de prueba que quedan en la bienvenida: $left"
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes en 5 minutos y 20 s' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 300
    $closed = Close-App 20; Start-Sleep -Milliseconds 1500
    # Arrancar la app recrea la carpeta de trabajo con la salida del proceso (app-*.log): se borra otra vez.
    if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    if ("$(Get-Clip)" -match 'FAROPY|FAROPZ|centinela-p|PY-Maestra|PZ-Familia|almarioks-p') { [System.Windows.Forms.Clipboard]::Clear() }
    $gen = @(Gen-List)
    if ($gen -contains "$(Get-Clip)") { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'App cerrada y carpetas temporales borradas' ($closed -and (Any-AlmarioProc).Count -eq 0 -and -not (Test-Path -LiteralPath $Pz) -and -not (Test-Path -LiteralPath $Work))
    $hist = foreach ($k in @('FAROPY', 'FAROPZ', 'centinela-pz', 'PY-Maestra', 'PZ-Familia', 'almarioks-pz', 'almarioks-py')) { "$k=$(History-Has $k)" }
    $genHist = 0; try { $items = (History-Items).Items; foreach ($g in $gen) { if (@($items | Where-Object { $_.Text.Contains($g) }).Count) { $genHist++ } } } catch { $genHist = -1 }
    Check 'Portapapeles e historial sin canarios, centinelas ni contrasenas generadas' (-not ("$(Get-Clip)" -match 'FAROPY|FAROPZ|centinela-p|PY-Maestra|PZ-Familia') -and @($hist | Where-Object { $_ -notlike '*=no' }).Count -eq 0 -and $genHist -eq 0) "borrados ahora del historial: $n; $($hist -join ', '); generadas en el historial: $genHist de $($gen.Count)"
}
$s = Settings-Json
$raw = Get-Content -Raw -Encoding UTF8 $SettingsFile
$kf = if ($null -eq $s.key_files) { 0 } else { @($s.key_files.PSObject.Properties).Count }
Check 'settings.json: 5 min, 20 s, sin bovedas ni llaves de prueba' ($s.auto_lock_minutes -eq 5 -and $s.clipboard_seconds -eq 20 -and $raw -notmatch 'almarioks-p[yz]' -and $kf -eq 0) "auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count) (antes de empezar: $(Get-State 'recientes-antes'), una de ellas la de muestra); llaves recordadas=$kf; rutas de prueba en el archivo = $($raw -match 'almarioks-p[yz]')"
Log "Instalado: $((Get-Item $InstalledExe).Length) bytes, version $([Diagnostics.FileVersionInfo]::GetVersionInfo($InstalledExe).ProductVersion); procesos almarioks* = $((Any-AlmarioProc).Count)"
Log "Documentos\AlmarioKS existe = $(Test-Path $DocsFam) (antes de la corrida: $(Get-State 'familia-carpeta-antes'))"
$rest = @(Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Temp') -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-p[xyz]*' }) + @(Get-ChildItem $env:PUBLIC -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-*' })
Log "Restos de esta QA en %TEMP% y en la carpeta publica: $($rest.Count)"
Summary
