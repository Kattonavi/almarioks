# Reverificacion final (5): D-3, D-6, D-7, D-8 y "Volver a la lista", uno por uno (D-4 y D-5 van en rf2 y rf4).
# Parte de la boveda abierta que deja rf4-humo.ps1 y termina en la bienvenida.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
# Los botones de "Acciones del grupo": si cada uno esta a la vista, se alcanza con el raton en su centro y en su borde
# inferior, cabe en la ventana y no lo pisa el texto "Guardado.".
function Actions-Report([string]$tag) {
    $i = Get-WinInfo
    $bar = Find-El -Like 'Acciones del grupo*' -Type Group -Timeout 3 -Quiet
    $btnCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::Button)
    $btns = if ($bar) { @($bar.FindAll($TreeScope::Descendants, $btnCond)) } else { @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match 'grupo|papelera' }) }
    $saved = Find-El -Name 'Guardado.' -Type Text -Timeout 0.5 -Quiet
    $sr = if ($saved) { $saved.Current.BoundingRectangle } else { $null }
    $bad = 0
    foreach ($b in $btns) {
        $c = $b.Current; $r = $c.BoundingRectangle
        if ($r.IsEmpty) { $bad++; Log "   [$tag] `"$($c.Name)`": sin rectangulo (fuera=$($c.IsOffscreen))"; continue }
        $cx = [int]($r.X + $r.Width / 2); $cy = [int]($r.Y + $r.Height / 2)
        $hit = Test-Hit $b $cx $cy
        $hitLow = Test-Hit $b $cx ([int]($r.Y + $r.Height - 3))
        $over = $false; if ($null -ne $sr -and -not $sr.IsEmpty) { $over = $r.IntersectsWith($sr) }
        $inside = ($r.Bottom -le ($i.Y + $i.OuterH)) -and ($r.Right -le ($i.X + $i.OuterW))
        if ($c.IsOffscreen -or -not $hit -or -not $hitLow -or $over -or -not $inside) { $bad++ }
        Log ("   [{0}] `"{1}`": fuera={2} centro-alcanzable={3} borde-inferior-alcanzable={4} pisado-por-Guardado={5} dentro-de-la-ventana={6} alto={7:n0}" -f $tag, $c.Name, $c.IsOffscreen, $hit, $hitLow, $over, $inside, $r.Height)
    }
    Log "   [$tag] interior logico $($i.LogicalW)x$($i.LogicalH); barra de acciones encontrada=$([bool]$bar); botones=$($btns.Count); con problema=$bad; 'Guardado.' a la vista=$([bool]$saved)"
    return [pscustomobject]@{ Count = $btns.Count; Bad = $bad }
}
# Con el argumento "desde18" se saltan los pasos RF16 y RF17 (ya hechos) y se cierra un dialogo que haya quedado abierto.
$from18 = ($StepArgs.Count -gt 0 -and $StepArgs[0] -eq 'desde18')
if ($from18 -and (Has-El -Name 'Cancelar' -Type Button -Timeout 1)) { Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 400 }
if (-not $from18) {
Step 'RF16 Grupos para D-6 y D-7' {
    Check 'Grupo "Trabajo"' (New-Group $RootName 'Trabajo'); Start-Sleep -Milliseconds 2600
    Check 'Subgrupo "Trabajo > Clientes"' (New-Group 'Trabajo' 'Clientes'); Start-Sleep -Milliseconds 2600
    Check 'Grupo "Personal"' (New-Group $RootName 'Personal'); Start-Sleep -Milliseconds 2600
    Check 'Grupo "Archivo"' (New-Group $RootName 'Archivo'); Start-Sleep -Milliseconds 2600
    Show-Tree 'con 6 grupos'
}
Step 'RF17 D-6: acciones del grupo a 800 x 600 con mas de 4 grupos' -Continue {
    [void](Set-WinClient 800 600)
    Select-Group 'Archivo'
    Start-Sleep -Milliseconds 500
    $rep = Actions-Report '800x600, grupo Archivo'
    Shot 'RF11-acciones-grupo-800x600'
    Check 'D-6: a 800 x 600 con 6 grupos, todas las acciones del grupo estan a la vista, enteras y sin nada encima' ($rep.Count -ge 3 -and $rep.Bad -eq 0) "botones $($rep.Count); con problema $($rep.Bad)"
}
}
Step 'RF18 D-7: el dialogo de mover grupo explica los destinos que faltan' -Continue {
    [void](Set-WinClient 800 600)
    Select-Group 'Trabajo'
    Start-Sleep -Milliseconds 400
    Click-Name -Name 'Mover grupo…'
    [void](Find-El -Id 'move-group-target' -Timeout 5)
    Start-Sleep -Milliseconds 400
    # El parrafo de ayuda no expone su id por UIA: se lee por su texto dentro del dialogo.
    $help = (Get-Names | Where-Object { $_ -like 'No aparecen*' -or $_ -like 'No hay otro grupo*' } | Select-Object -Unique) -join ' '
    $opts = List-Options 'move-group-target'
    Log "   destinos para 'Trabajo': $($opts -join ' | ')"
    Log "   ayuda del dialogo: $help"
    Shot 'RF12-dialogo-mover-grupo-800x600'
    Check 'D-7: el dialogo dice por que faltan el propio grupo y sus subgrupos' ($help -like '*no puede ir dentro de sí mismo*') $help
    Check 'D-7: ni "Trabajo" ni "Clientes" salen como destino' (@($opts | Where-Object { $_ -like '*Trabajo*' -or $_ -like '*Clientes*' }).Count -eq 0)
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 400
}
Step 'RF19 "Volver a la lista" en ventana estrecha' -Continue {
    $wide = Has-El -Name 'Volver a la lista' -Type Button -Timeout 0.5
    Open-Entry $RootName $Rf.Title
    Log "   a 800 x 600 con una entrada abierta: 'Volver a la lista' existe = $(Has-El -Name 'Volver a la lista' -Type Button -Timeout 0.5)"
    Shot 'RF13-boveda-detalle-800x600'
    $i = Set-WinClient 480 360
    Start-Sleep -Milliseconds 500
    # La entrada sigue abierta desde el paso anterior: al estrechar, el detalle ocupa el sitio de la lista.
    if (-not (Has-El -Name 'Copiar contraseña' -Type Button -Timeout 1)) { Open-Entry $RootName $Rf.Title }
    Shot 'RF14-boveda-detalle-480x360'
    Dump-Tree 'RF14-boveda-detalle-480x360.uia.txt' | Out-Null
    $back = Find-El -Name 'Volver a la lista' -Type Button -Timeout 3 -Quiet
    Check '"Volver a la lista" existe a 480 x 360 con una entrada abierta' ($null -ne $back -and -not $back.Current.IsOffscreen) "interior $($i.LogicalW)x$($i.LogicalH)"
    $count = Read-Count
    $label = Find-El -Name 'Se bloquea en' -Timeout 0.5 -Quiet
    Log "   a 480 x 360: cuenta de bloqueo = $count; rotulo 'Se bloquea en' en el arbol = $([bool]$label)$(if ($label) { ", fuera de la vista = $($label.Current.IsOffscreen)" })"
    if ($null -ne $back) {
        Click-El $back
        Start-Sleep -Milliseconds 500
        $row = Find-El -Name $Rf.Title -Type Button -Timeout 3 -Quiet
        Check 'Tras "Volver a la lista" se ve la lista y el foco queda en la fila de la entrada' ($null -ne $row -and -not $row.Current.IsOffscreen -and (Focused-Desc) -like "*$($Rf.Title)*") (Focused-Desc)
        Shot 'RF15-lista-tras-volver-480x360'
    }
    Select-Group 'Archivo'
    Start-Sleep -Milliseconds 400
    $rep = Actions-Report '480x360, grupo Archivo'
    Shot 'RF16-acciones-grupo-480x360'
    Check 'D-6 (480 x 360): las acciones del grupo se alcanzan enteras' ($rep.Count -ge 3 -and $rep.Bad -eq 0) "botones $($rep.Count); con problema $($rep.Bad)"
    [void](Set-WinClient 1120 720)
    Start-Sleep -Milliseconds 400
}
Step 'RF20 D-3: la cuenta "Se bloquea en" descuenta, sigue el ajuste y el bloqueo llega cerca de 0:00' -Continue {
    Select-Group $RootName
    $before = Read-Count
    Check 'Ajustes: bloqueo automatico a 1 minuto' (Set-AutoLock '1 minuto')
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Pointer-Away
    Start-Sleep -Milliseconds 700
    $c0 = Read-Count; $t0 = $sw.Elapsed.TotalSeconds
    Check 'D-3: recien guardado el ajuste de 1 minuto, la cuenta ya lo sigue' ((Count-Secs $c0) -gt 45 -and (Count-Secs $c0) -le 60) "antes del cambio decia $before; ahora $c0"
    Start-Sleep -Seconds 6
    $c1 = Read-Count; $t1 = $sw.Elapsed.TotalSeconds
    Start-Sleep -Seconds 6
    $c2 = Read-Count; $t2 = $sw.Elapsed.TotalSeconds
    Log ("   lecturas sin tocar la ventana: {0} a los {1:n1} s; {2} a los {3:n1} s; {4} a los {5:n1} s" -f $c0, $t0, $c1, $t1, $c2, $t2)
    $d1 = (Count-Secs $c0) - (Count-Secs $c1); $d2 = (Count-Secs $c1) - (Count-Secs $c2)
    Check 'D-3: tres lecturas separadas 6 s: la cuenta baja al ritmo del reloj' ($d1 -ge 5 -and $d1 -le 8 -and $d2 -ge 5 -and $d2 -le 8) "$c0 -> $c1 -> $c2"
    Shot 'RF17-cuenta-regresiva'
    $locked = -1; $last = ''; $lastAt = 0
    while ($sw.Elapsed.TotalSeconds -lt 110) {
        if (Has-El -Name 'Desbloquear' -Type Button -Timeout 0.1) { $locked = $sw.Elapsed.TotalSeconds; break }
        $c = Read-Count
        if ($c) { $last = $c; $lastAt = $sw.Elapsed.TotalSeconds }
        Start-Sleep -Milliseconds 250
    }
    Log ("TIEMPO bloqueo automatico: {0:n1} s desde la ultima entrada real (ajuste: 1 minuto); ultima cuenta leida: {1}, {2:n1} s antes del bloqueo" -f $locked, $last, ($locked - $lastAt))
    Check 'RF-29: se bloquea sola al minuto' ($locked -ge 50 -and $locked -le 68) ("{0:n1} s" -f $locked)
    Check 'D-3: el bloqueo llega con la cuenta cerca de 0:00' ((Count-Secs $last) -ge 0 -and (Count-Secs $last) -le 3) "ultima cuenta $last"
    Start-Sleep -Milliseconds 1200
    Shot 'RF18-bloqueo-automatico'
    $tree = (Dump-Tree 'RF18-bloqueo-automatico.uia.txt') -join ' ## '
    Check 'RF-32: tras el bloqueo automatico el arbol de la ventana no expone canarios' (-not $tree.Contains('TESTIGO-RF'))
    [void](Unlock-Timed 'tras el bloqueo automatico')
    Check 'Ajustes devueltos a 5 minutos y 20 s' (Set-AutoLock '5 minutos' '20')
    Pointer-Away; Start-Sleep -Milliseconds 700
    $c5 = Read-Count
    Check 'D-3: al volver a 5 minutos la cuenta lo sigue' ((Count-Secs $c5) -gt 285 -and (Count-Secs $c5) -le 300) "ahora $c5"
}
Step 'RF21 D-8: crear sobre un archivo que ya existe' -Continue {
    Start-Sleep -Milliseconds 2500
    [void](Lock-Timed)
    Click-Name -Name 'Cerrar bóveda'
    Check 'Bienvenida tras cerrar la boveda' (Has-El -Id 'welcome-title' -Timeout 5)
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    $hint = (Get-Names | Where-Object { $_ -like 'AlmarioKS no reemplaza archivos*' }) -join ' '
    Check 'D-8: antes de elegir, el formulario avisa que AlmarioKS no reemplaza archivos' ([bool]$hint) $hint
    $hash = Sha $RfVault
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $RfVault -Replace
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 1500
    $msg = @(Get-Names (Find-El -Id 'create-error' -Timeout 4)) -join ' '
    Shot 'RF19-archivo-ya-existe'
    Check 'D-8: el error explica que no se reemplaza aunque Windows haya preguntado' ($msg -like '*nunca reemplaza un archivo*Windows*') $msg
    Check 'RF-4: el archivo existente queda identico' ((Sha $RfVault) -eq $hash)
    Click-Name -Name 'Volver a la bienvenida'
    Check 'De vuelta en la bienvenida' (Has-El -Id 'welcome-title' -Timeout 5)
}
Summary
