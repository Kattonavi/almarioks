# Reverificacion final (2): primer arranque con perfil nuevo y el recorrido minimo que en la primera prueba dejo
# rastro en disco, con canarios nuevos. Termina con cierre normal; despues corre rf3-inspeccion.ps1.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'RF3 Primer arranque (perfil de WebView2 nuevo) y tamano inicial' {
    if ((Get-AppProc).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    Check 'El perfil de WebView2 no existe antes del primer arranque' (-not (Test-Path $EbDir))
    $ms = Start-AppTimed 'primero, perfil nuevo'
    Set-State 'arranque1' "$ms"
    Check 'La app abre en la bienvenida' ($ms -ge 0 -and (Has-El -Id 'welcome-title' -Timeout 3)) "$ms ms"
    $i = Get-WinInfo
    Log "VENTANA inicial: interior logico $($i.LogicalW) x $($i.LogicalH) (fisico $($i.ClientW) x $($i.ClientH), escala $($i.Scale)); maximizada = $($i.Maximized)"
    Check 'Tamano inicial 1120 x 720 logicos' ([math]::Abs($i.LogicalW - 1120) -le 1 -and [math]::Abs($i.LogicalH - 720) -le 1) "$($i.LogicalW) x $($i.LogicalH)"
    Shot 'RF01-bienvenida-primer-arranque'
    Dump-Tree 'RF01-bienvenida.uia.txt' | Out-Null
    Show-Net 'bienvenida, primer arranque'
}
Step 'RF4 Crear la boveda tecleando la maestra y su repeticion' {
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $RfVault
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfOther -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 800
    $msg = @(Get-Names (Find-El -Id 'create-error' -Timeout 2)) -join ' '
    Check 'Repeticion distinta tecleada y rechazada' ($msg -like '*no coinciden*') $msg
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    $btn = Find-El -Name 'Crear bóveda' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 90 -Quiet)
    $ms = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO crear boveda (clic -> boveda abierta): $ms ms"
    Check 'Boveda creada y abierta' ($ok -and (Test-Path $RfVault)) "$ms ms; $((Get-Item $RfVault).Length) bytes"
    Shot 'RF02-boveda-recien-creada'
}
Step 'RF5 Entrada, grupo, busquedas, ajustes' {
    Select-Group $RootName
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text 'Nueva entrada en *' 5)
    Fill-Id 'entry-title' $Rf.Title
    Fill-Id 'entry-username' $Rf.User
    Fill-Id 'entry-password' $Rf.Pass -Secret
    Fill-Id 'entry-url' $Rf.Url
    Fill-Id 'entry-notes' $Rf.Notes
    Fill-Id 'entry-tags' $Rf.Tag
    $s = Save-EntryTimed
    Check 'Entrada con titulo, usuario, URL, notas, etiqueta y contrasena canario' ($s.Msg -ge 0 -and $s.Disk -ge 0)
    Check 'D-4: "Entrada guardada." no aparece antes de que el archivo cambie en disco' ($s.DiskAtMsg -eq $true) "aviso $($s.Msg) ms; disco $($s.Disk) ms"
    Start-Sleep -Milliseconds 800
    Check 'Grupo con nombre canario' (New-Group $RootName $Rf.Group)
    Start-Sleep -Milliseconds 2500
    Fill (Find-El -Id 'vault-search') $Rf.Partial
    Start-Sleep -Milliseconds 800
    Check 'Busqueda con resultado' (Has-El -Name $Rf.Title -Type Button -Timeout 3)
    Press 'ENTER'; Start-Sleep -Milliseconds 600
    Fill (Find-El -Id 'vault-search') $Rf.Search
    Start-Sleep -Milliseconds 800
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    Click-El (Find-El -Id 'vault-search')
    Press 'ESC'; Start-Sleep -Milliseconds 500
    Check 'Ajustes: 15 minutos y 30 s, guardados' (Set-AutoLock '15 minutos' '30')
    Check 'Ajustes: de vuelta a 5 minutos y 20 s' (Set-AutoLock '5 minutos' '20')
}
Step 'RF6 Detalle al tamano por defecto, bloquear, desbloquear y cierre normal' {
    Open-Entry $RootName $Rf.Title
    Start-Sleep -Milliseconds 500
    $i = Get-WinInfo
    Shot "RF03-boveda-detalle-$($i.LogicalW)x$($i.LogicalH)"
    Dump-Tree 'RF03-boveda-detalle.uia.txt' | Out-Null
    $names = @(Get-Names)
    Check 'Detalle: usuario y URL completos en el arbol de la ventana' (($names -contains $Rf.User) -and ($names -contains $Rf.Url))
    Check 'Detalle: acciones con nombre accesible completo' ((Has-El -Name 'Copiar contraseña' -Type Button) -and (Has-El -Name 'Copiar usuario' -Type Button) -and (Has-El -Name 'Abrir URL en el navegador' -Type Button) -and (Has-El -Name 'Enviar a la papelera' -Type Button))
    [void](Lock-Timed)
    Shot 'RF04-bloqueada'
    $locked = (Dump-Tree 'RF04-bloqueada.uia.txt') -join ' ## '
    Check 'Bloqueada: el arbol de la ventana no expone canarios' (-not $locked.Contains('TESTIGO-RF'))
    $ms = Unlock-Timed 'tras bloquear'
    Check 'Desbloquear con la maestra nueva' (Has-El -Name 'Bloquear' -Type Button -Timeout 2) "$ms ms"
    Check 'La entrada y el grupo siguen ahi' ((Has-El -Name $Rf.Title -Type Button -Timeout 4) -and (Has-El -Like "$($Rf.Group) · *" -Type Button -Timeout 2))
    Start-Sleep -Seconds 3
    $closed = Close-App 20
    Check 'App cerrada con cierre normal' $closed
    Start-Sleep -Seconds 3
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length) bytes" }
    Check 'En disco: .kdbx y .kdbx.bak, sin .tmp ni recuperacion' ((Test-Path $RfVault) -and (Test-Path "$RfVault.bak") -and @(Get-ChildItem $Work | Where-Object { $_.Name -match '\.tmp|recuper' }).Count -eq 0)
}
Summary
