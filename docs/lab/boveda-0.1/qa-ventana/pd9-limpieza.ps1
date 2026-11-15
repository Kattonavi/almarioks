# Decisiones del dueno (9): limpieza final, como pz9-limpieza.ps1. Con la app cerrada borra la carpeta de la boveda
# de prueba; abre la app y quita de "recientes" SOLO "Decisiones.kdbx" (las demas no se tocan ni se nombran);
# comprueba ajustes (5 min, 20 s, sin llaves recordadas), purga portapapeles e historial de lo de esta corrida y deja
# la app cerrada. La carpeta base (estado y salida de la app) se borra despues, desde fuera. RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$mine = "$PdName.kdbx"
Step 'PD90 Limpieza' {
    if ((Any-AlmarioProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 3 }
    if ((Any-AlmarioProc).Count -gt 0) { throw 'Quedan procesos almarioks* vivos' }
    if ($Pz -ne (Join-Path $env:PD_DIR 'boveda')) { throw 'La carpeta no es la esperada: no se borra nada' }
    if (Test-Path -LiteralPath $Pz) { Remove-Item -LiteralPath $Pz -Recurse -Force }
    Start-Checked 'limpieza'; Start-Sleep -Milliseconds 900
    $btn = Find-El -Like "Quitar $mine*" -Type Button -Timeout 2 -Quiet
    if ($null -ne $btn) { Click-El $btn; Start-Sleep -Milliseconds 800 } else { Log "   reciente $mine : no hay boton para quitarla" }
    $left = @(Get-Names | Where-Object { $_ -like "*$mine*" }).Count
    Check 'Reciente de prueba quitada desde la app' ($left -eq 0) "nombres de prueba que quedan en la bienvenida: $left"
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes en 5 minutos y 20 s' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 300
    $closed = Close-App 20; Start-Sleep -Milliseconds 1500
    $gen = @(Gen-List)
    if ("$(Get-Clip)" -match 'FAROPD|centinela-p|centinela-vn|PD-Decisiones' -or $gen -contains "$(Get-Clip)") { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'App cerrada y carpeta de la boveda borrada' ($closed -and (Any-AlmarioProc).Count -eq 0 -and -not (Test-Path -LiteralPath $Pz))
    $hist = foreach ($k in @('FAROPD', 'centinela-pd', 'centinela-vn', 'PD-Decisiones')) { "$k=$(History-Has $k)" }
    $genHist = 0; try { $items = (History-Items).Items; foreach ($g in $gen) { if (@($items | Where-Object { $_.Text.Contains($g) }).Count) { $genHist++ } } } catch { $genHist = -1 }
    Check 'Portapapeles e historial sin canarios, centinelas ni generadas' (-not ("$(Get-Clip)" -match 'FAROPD|centinela-p|centinela-vn|PD-Decisiones') -and @($hist | Where-Object { $_ -notlike '*=no' }).Count -eq 0 -and $genHist -eq 0) "borrados ahora del historial: $n; $($hist -join ', '); generadas en el historial: $genHist de $($gen.Count)"
}
$s = Settings-Json
$raw = Get-Content -Raw -Encoding UTF8 $SettingsFile
$kf = if ($null -eq $s.key_files) { 0 } else { @($s.key_files.PSObject.Properties).Count }
Check 'settings.json: 5 min, 20 s, sin la boveda ni llaves de prueba' ($s.auto_lock_minutes -eq 5 -and $s.clipboard_seconds -eq 20 -and $raw -notmatch [regex]::Escape($mine) -and $kf -eq 0) "auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count) (antes: $(Get-State 'recientes-antes')); llaves recordadas=$kf"
Log "Instalado: version $([Diagnostics.FileVersionInfo]::GetVersionInfo($InstalledExe).ProductVersion); procesos almarioks* = $((Any-AlmarioProc).Count)"
Summary
