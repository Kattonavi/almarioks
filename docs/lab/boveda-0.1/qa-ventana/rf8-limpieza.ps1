# Reverificacion final (8): limpieza. Borra la carpeta de trabajo, quita la boveda de prueba de "recientes" desde la
# app, comprueba los ajustes (5 minutos, 20 s) y deja la app de release instalada y cerrada, con el portapapeles vacio.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'RF36 Limpieza' {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 2 }
    if ($Work -ne (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa2-20261111')) { throw 'La carpeta de trabajo no es la esperada' }
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Start-App
    [void](Find-El -Id 'welcome-title' -Timeout 30)
    Start-Sleep -Milliseconds 800
    Shot 'RF50-reciente-que-ya-no-existe'
    # Segundo intento del minimo arrastrando la esquina (en rf7 el arrastre no agarro el borde): se prueba agarrando
    # justo fuera y justo dentro del borde visible.
    foreach ($delta in @(3, -3, 6)) {
        [void](Focus-App)
        [void](Set-WinClient 800 600)
        $r = [QA.Native]::VisibleRect((Get-AppHwnd))
        $x0 = $r.Right + $delta; $y0 = $r.Bottom + $delta
        [void][QA.Native]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 400
        [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 250
        for ($k = 1; $k -le 30; $k++) {
            [void][QA.Native]::SetCursorPos([int]($x0 - ($x0 - $r.Left - 60) * $k / 30), [int]($y0 - ($y0 - $r.Top - 60) * $k / 30))
            [QA.Native]::mouse_event(0x0001, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
        }
        [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600
        $i = Get-WinInfo
        Log "MINIMO arrastrando la esquina (agarre a $delta px del borde visible): interior logico $($i.LogicalW) x $($i.LogicalH)"
        if ($i.LogicalW -lt 790) { break }
    }
    $i = Get-WinInfo
    Check 'Arrastrando la esquina con el raton, la ventana se detiene en 480 x 360' ($i.LogicalW -ge 478 -and $i.LogicalW -le 500 -and $i.LogicalH -ge 358 -and $i.LogicalH -le 380) "$($i.LogicalW) x $($i.LogicalH)"
    Shot 'RF52-minimo-arrastrando'
    [void](Set-WinClient 1120 720)
    $missing = (Get-Names | Where-Object { $_ -like '*ya no está en*' -or $_ -like '*Puede que se haya movido*' }) -join ' '
    Log "   texto de la reciente que ya no existe: $missing"
    if (Has-El -Name 'Quitar rf-uno.kdbx de la lista' -Type Button -Timeout 3) { Click-Name -Name 'Quitar rf-uno.kdbx de la lista'; Start-Sleep -Milliseconds 600 }
    Check 'Recientes de prueba quitadas desde la app' (-not (Has-El -Like '*rf-uno.kdbx*' -Timeout 1))
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes en 5 minutos y 20 s' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    Shot 'RF51-bienvenida-final'
    $closed = Close-App 20
    Start-Sleep -Milliseconds 1500
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Check 'App cerrada y carpeta de trabajo borrada' ($closed -and -not (Test-Path $Work))
    if ((Get-Clip) -ne '') { [System.Windows.Forms.Clipboard]::Clear() }
}
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Log "Estado final: procesos almarioks = $((Get-AppProc).Count); carpeta de trabajo existe = $(Test-Path $Work); settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
Log "Instalado: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($AppExe).ProductVersion); portapapeles: $((Get-Clip).Length) caracteres; EBWebView existe = $(Test-Path $EbDir)"
$left = @(Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Temp') -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-qa2*' })
Log "Restos de esta QA en %TEMP%: $($left.Count)"
Summary
