# Reverificacion final (7): ventana. Posicion y tamano con que abre, capturas de la boveda con detalle a 1120 x 720,
# 800 x 600 y al minimo, minimo arrastrando la esquina, e instancia unica. Termina con cierre normal.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'RF31 Donde y de que tamano abre la ventana' -Continue {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 2 }
    $ms = Start-AppTimed 'cuarto'
    $i = Get-WinInfo
    $r = [QA.Native]::VisibleRect($i.Hwnd)
    $wa = [System.Windows.Forms.Screen]::FromHandle($i.Hwnd).WorkingArea
    $sb = [System.Windows.Forms.Screen]::FromHandle($i.Hwnd).Bounds
    Log "VENTANA al abrir: interior logico $($i.LogicalW) x $($i.LogicalH); borde visible fisico ($($r.Left), $($r.Top)) a ($($r.Right), $($r.Bottom)); pantalla $($sb.Width) x $($sb.Height); area util hasta x=$($wa.Right), y=$($wa.Bottom); escala $($i.Scale)"
    Check 'Tamano inicial 1120 x 720 logicos' ([math]::Abs($i.LogicalW - 1120) -le 1 -and [math]::Abs($i.LogicalH - 720) -le 1) "$($i.LogicalW) x $($i.LogicalH)"
    Check 'La ventana abre entera dentro del area util de la pantalla (no queda bajo la barra de tareas)' ($r.Bottom -le $wa.Bottom -and $r.Right -le $wa.Right -and $r.Top -ge $wa.Top -and $r.Left -ge $wa.Left) "sobra por abajo: $($r.Bottom - $wa.Bottom) px fisicos; por la derecha: $($r.Right - $wa.Right)"
    Shot 'RF40-bienvenida-posicion-inicial'
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    Start-Sleep -Milliseconds 500
    Shot 'RF41-boveda-detalle-posicion-inicial'
}
Step 'RF32 Capturas de la boveda con detalle en tres tamanos' -Continue {
    Ensure-VaultOpen
    foreach ($size in @(@(1120, 720), @(800, 600), @(480, 360))) {
        $i = Set-WinClient $size[0] $size[1]
        Start-Sleep -Milliseconds 500
        if (-not (Has-El -Name 'Copiar contraseña' -Type Button -Timeout 1)) { Open-Entry $RootName $Rf.Title }
        Start-Sleep -Milliseconds 400
        [void][QA.Native]::SetCursorPos($i.X + 30, $i.Y + 12)
        Start-Sleep -Milliseconds 300
        Shot "RF42-boveda-detalle-$($i.LogicalW)x$($i.LogicalH)"
        $names = @(Get-Names)
        $off = @(Get-All | Where-Object { $_.Current.IsOffscreen -and (Type-Of $_) -in @('Button', 'Edit', 'ComboBox') -and $_.Current.Name } | ForEach-Object { $_.Current.Name } | Select-Object -Unique)
        Log "   [$($i.LogicalW)x$($i.LogicalH)] usuario completo en el arbol = $($names -contains $Rf.User); URL completa = $($names -contains $Rf.Url); 'Volver a la lista' = $(Has-El -Name 'Volver a la lista' -Type Button -Timeout 0.3); controles que piden desplazarse ($($off.Count)): $($off -join ' | ')"
    }
}
Step 'RF33 Minimo de la ventana arrastrando la esquina con el raton' -Continue {
    [void](Focus-App)
    [void](Set-WinClient 800 600)
    $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    $x0 = $r.Right + 3; $y0 = $r.Bottom + 3   # 3 px fuera del borde visible: dentro (-2) el arrastre no agarraba el borde
    [void][QA.Native]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 200
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150
    for ($k = 1; $k -le 25; $k++) {
        [void][QA.Native]::SetCursorPos([int]($x0 - ($x0 - $r.Left - 60) * $k / 25), [int]($y0 - ($y0 - $r.Top - 60) * $k / 25))
        [QA.Native]::mouse_event(0x0001, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
    }
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 500
    $i = Get-WinInfo
    Check 'Arrastrando con el raton, la ventana no baja de 480 x 360' ($i.LogicalW -ge 478 -and $i.LogicalH -ge 358 -and $i.LogicalW -le 500 -and $i.LogicalH -le 380) "$($i.LogicalW) x $($i.LogicalH)"
    Shot 'RF43-minimo-arrastrando'
    [void](Set-WinClient 1120 720)
}
Step 'RF34 Instancia unica (RF-81)' -Continue {
    $before = (Get-AppProc).Count
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $AppExe; $psi.UseShellExecute = $true
    $second = [System.Diagnostics.Process]::Start($psi)
    Start-Sleep -Seconds 4
    $procs = (Get-AppProc).Count
    $wins = 0
    foreach ($p in Get-AppProc) { foreach ($h in [QA.Native]::WindowsOfPid([uint32]$p.Id)) { if ([QA.Native]::ClassOf($h) -eq 'Tauri Window') { $wins++ } } }
    Log "   procesos antes = $before; tras abrir otra vez = $procs; ventanas de Tauri = $wins; la segunda instancia termino = $($second.HasExited); ventana en primer plano = $([QA.Native]::GetForegroundWindow() -eq (Get-AppHwnd)); pantalla = $(Screen-Now)"
    Check 'RF-81: al abrir dos veces queda un solo proceso y una sola ventana, con la boveda como estaba' ($procs -eq 1 -and $wins -eq 1 -and $second.HasExited -and (Screen-Now) -eq 'boveda abierta')
}
Step 'RF35 Cierre normal con una contrasena recien copiada' {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    $had = ((Get-Clip) -ceq $Rf.Pass)
    Start-Sleep -Milliseconds 2500
    $closed = Close-App 20
    Start-Sleep -Seconds 3
    Check 'RF-75: cierre normal con la boveda abierta: el proceso termina y el portapapeles queda vacio' ($had -and $closed -and (Get-Clip) -eq '')
    Check 'No quedan .tmp ni recuperaciones tras cerrar' (@(Get-ChildItem $Work | Where-Object { $_.Name -match '\.tmp|recuper' }).Count -eq 0)
}
Summary
