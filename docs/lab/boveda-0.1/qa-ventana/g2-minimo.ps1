# G (segunda parte): el minimo de la ventana arrastrando el borde como una persona, capturas a 480x360 reales,
# Enter y flecha abajo en la busqueda, y cierre de la app con la boveda abierta.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
function Size-Note([string]$tag) {
    $i = Get-WinInfo
    $off = @(Get-All | Where-Object { $_.Current.IsOffscreen -and (Type-Of $_) -in @('Button', 'Edit', 'ComboBox') -and $_.Current.Name } | ForEach-Object { $_.Current.Name } | Select-Object -Unique)
    Log "   [$tag] interior logico $($i.LogicalW)x$($i.LogicalH); controles que piden desplazarse ($($off.Count)): $($off -join ' | ')"
}
# g-ventana.ps1 pidio 300x200 con SetWindowPos y la ventana lo acepto: esas capturas no son de 480x360.
foreach ($n in @('G04-boveda-detalle', 'G04b-boveda-copiado', 'G05-formulario-generador', 'G07-dialogo-borrar', 'G06-ajustes')) {
    $old = Join-Path $Shots "$n-480x360.png"
    if (Test-Path $old) { Move-Item -Force $old (Join-Path $Shots "$n-300x200.png") }
}
Step 'G6 Minimo de la ventana arrastrando la esquina con el raton' -Continue {
    [void](Focus-App)
    [void](Set-WinClient 800 600)
    $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    $x0 = $r.Right - 2; $y0 = $r.Bottom - 2
    [void][QA.Native]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 200
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150
    for ($k = 1; $k -le 25; $k++) {
        [void][QA.Native]::SetCursorPos([int]($x0 - ($x0 - $r.Left - 60) * $k / 25), [int]($y0 - ($y0 - $r.Top - 60) * $k / 25))
        [QA.Native]::mouse_event(0x0001, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
    }
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 500
    $i = Get-WinInfo
    Log "   tras arrastrar la esquina hasta casi cerrar la ventana: interior logico $($i.LogicalW)x$($i.LogicalH)"
    Check 'Arrastrando con el raton, la ventana no baja de 480x360 (tauri.conf.json)' ($i.LogicalW -ge 478 -and $i.LogicalH -ge 358 -and $i.LogicalW -le 500) "$($i.LogicalW)x$($i.LogicalH)"
}
Step 'G7 Capturas a 480x360' -Continue {
    $i = Set-WinClient 480 360
    Log "   interior logico $($i.LogicalW)x$($i.LogicalH)"
    Click-Name -Name 'Abrir qa-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Start-Sleep -Milliseconds 500
    Shot 'G03-boveda-480x360'
    Size-Note '480x360 boveda'
    Open-Entry 'Personal QA' $Canary.Title
    Shot 'G04-boveda-detalle-480x360'
    Size-Note '480x360 boveda con detalle'
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    Check 'A 480x360: copiar la contrasena con el raton funciona' ((Get-Clip) -ceq $Canary.Pass2)
    Shot 'G04b-boveda-copiado-480x360'
    Click-Name -Name 'Editar'
    [void](Wait-Text 'Editar *' 4)
    Click-Name -Name 'Generar contraseña'
    Start-Sleep -Milliseconds 400
    Shot 'G05-formulario-generador-480x360'
    Size-Note '480x360 formulario con generador'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 400
    Select-Group 'Archivo'
    Click-Name -Name 'Borrar grupo…'
    Start-Sleep -Milliseconds 500
    Shot 'G07-dialogo-borrar-480x360'
    Size-Note '480x360 dialogo de borrado'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    Shot 'G06-ajustes-480x360'
    Size-Note '480x360 ajustes'
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
}
Step 'G8 Busqueda con teclado: Enter y flecha abajo' -Continue {
    [void](Set-WinClient 800 600)
    Select-Group 'qa-uno'
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Type-Text 'correo'; Start-Sleep -Milliseconds 800
    Press 'ENTER'; Start-Sleep -Milliseconds 600
    $afterEnter = Has-El -Name 'Copiar contraseña' -Type Button -Timeout 0.5
    Log "   Enter en la busqueda: abre el primer resultado = $afterEnter; foco: $(Focused-Desc)"
    Press 'DOWN'; Start-Sleep -Milliseconds 400
    Log "   flecha abajo en la busqueda: foco: $(Focused-Desc)"
    Check 'Tarea principal en 3 pasos con teclado (atajo, escribir, copiar): Enter abre el primer resultado' $afterEnter 'si falla, hay que tabular por la barra, los grupos y sus acciones para llegar al resultado (9 Tab + Enter + 3 Tab + Enter)'
    Press 'ESC'; Start-Sleep -Milliseconds 300
}
Step 'G9 Cerrar la app con la boveda abierta' {
    Open-Entry 'Personal QA' $Canary.Title
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    $closed = Close-App 20
    Start-Sleep -Milliseconds 500
    Check 'Cierre normal con la boveda abierta: el proceso termina y el portapapeles queda vacio (RF-75)' ($closed -and (Get-Clip) -eq '')
    Check 'No quedan .tmp ni recuperaciones tras cerrar' (@(Get-ChildItem $Work | Where-Object { $_.Name -match '\.tmp|recuperacion' }).Count -eq 0)
}
Summary
