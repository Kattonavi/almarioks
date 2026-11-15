# Reverificacion final (4): humo del flujo principal en release, con tiempos. Dos arranques mas, abrir la reciente,
# entrada con contrasena GENERADA, buscar con TECLADO (flecha abajo, Enter), copiar, portapapeles real, bloquear.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
function Classes([string]$s) {
    $c = @()
    if ($s -cmatch '[a-z]') { $c += 'min' }; if ($s -cmatch '[A-Z]') { $c += 'may' }
    if ($s -match '\d') { $c += 'dig' }; if ($s -match '[^A-Za-z0-9]') { $c += 'sim' }
    return ($c -join '+')
}
Step 'RF11 Dos arranques mas' {
    if ((Get-AppProc).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    $a2 = Start-AppTimed 'segundo'
    Check 'RF-19: la bienvenida ofrece la boveda reciente' (Has-El -Name 'Abrir rf-uno.kdbx' -Type Button -Timeout 5)
    Shot 'RF05-bienvenida-con-reciente'
    [void](Close-App 20); Start-Sleep -Seconds 2
    $a3 = Start-AppTimed 'tercero'
    Log "TIEMPO arranques hasta leer 'Bienvenida': primero (perfil nuevo) $(Get-State 'arranque1') ms; segundo $a2 ms; tercero $a3 ms"
    $i = Get-WinInfo
    Check 'El tamano inicial se repite en cada arranque: 1120 x 720' ([math]::Abs($i.LogicalW - 1120) -le 1 -and [math]::Abs($i.LogicalH - 720) -le 1) "$($i.LogicalW) x $($i.LogicalH)"
}
Step 'RF12 Abrir la reciente y desbloquear' {
    Click-Name -Name 'Abrir rf-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $ms = Unlock-Timed 'tras reiniciar la app'
    Check 'La boveda abre con su entrada y su grupo' ((Has-El -Name $Rf.Title -Type Button -Timeout 4) -and (Has-El -Like "$($Rf.Group) · *" -Type Button -Timeout 2)) "$ms ms"
}
Step 'RF13 Entrada con contrasena generada' {
    Select-Group $RootName
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text 'Nueva entrada en *' 5)
    Fill-Id 'entry-title' $Rf.Title2
    Fill-Id 'entry-username' 'humo@example.net'
    Click-Name -Name 'Generar contraseña'
    Check 'Aparece el generador' ((Wait-Text 'Generador de contraseñas' 5) -ge 0)
    Click-Name -Name 'Generar'
    $use = Find-El -Name 'Usar esta contraseña' -Type Button -Timeout 5
    $len = Find-El -Id 'gen-length' -Timeout 2 -Quiet
    $lenText = if ($len) { Get-Value $len } else { '?' }
    $strength = (Get-Names | Where-Object { $_ -like '* de 4)*' }) -join ' '
    Log "   generador: longitud $lenText; $strength"
    Shot 'RF06-formulario-con-generador'
    Click-El $use
    Start-Sleep -Milliseconds 400
    $s = Save-EntryTimed
    Check 'Entrada con contrasena generada: guardada' ($s.Msg -ge 0 -and $s.Disk -ge 0)
    Check 'D-4: "Entrada guardada." no aparece antes de que el archivo cambie en disco' ($s.DiskAtMsg -eq $true) "aviso $($s.Msg) ms; disco $($s.Disk) ms"
    Check 'D-4: antes del aviso final se vio el aviso de que se esta guardando' (@($s.Seen | Where-Object { $_ -like 'Entrada creada. Guard*' }).Count -ge 1)
    Shot 'RF07-entrada-guardada'
    Start-Sleep -Milliseconds 800
}
Step 'RF14 Buscar con teclado y copiar (D-5)' {
    [void](Focus-App)
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    $f0 = Focused-Desc
    Check 'Ctrl+K lleva el foco a la busqueda' ($f0 -like '*vault-search*') $f0
    Type-Text 'rf-gener'; Start-Sleep -Milliseconds 900
    Check 'La entrada sale en los resultados' (Has-El -Name $Rf.Title2 -Type Button -Timeout 3)
    Press 'DOWN'; Start-Sleep -Milliseconds 350
    $f1 = Focused-Desc
    Check 'D-5: flecha abajo en la busqueda lleva al primer resultado' ($f1 -like "*$($Rf.Title2)*") $f1
    Shot 'RF08-foco-primer-resultado'
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $f2 = Focused-Desc
    Check 'D-5: Enter abre la entrada con el foco en "Copiar contraseña"' ($f2 -like '*Copiar contraseña*') $f2
    Shot 'RF09-foco-copiar'
    $seq0 = Get-ClipSeq
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    $pw = Get-Clip
    Set-State 'generada' $pw
    $status = (Get-Names | Where-Object { $_ -like 'Copiado*' }) -join ' | '
    $n = if ($status -match 'se borra en (\d+) s') { [int]$Matches[1] } else { 20 }
    Log "   estado en la ventana: $status; portapapeles: $($pw.Length) caracteres, tipos $(Classes $pw); secuencia $seq0 -> $(Get-ClipSeq)"
    Log "   formatos en el portapapeles: $((Get-ClipFormats) -join ', ')"
    Check 'Tarea principal con teclado en 5 teclas: Ctrl+K, escribir, flecha abajo, Enter, Enter' ($pw.Length -eq 20 -and (Classes $pw) -eq 'min+may+dig+sim') "$($pw.Length) caracteres, $(Classes $pw)"
    Check 'RF-74: la ventana dice en cuantos segundos se limpia' ([bool]$status) $status
    Check 'RF-72: formatos de exclusion del historial y de la nube' (((Get-ClipFormats) -contains 'ExcludeClipboardContentFromMonitorProcessing') -and ((Get-ClipFormats) -contains 'CanIncludeInClipboardHistory') -and ((Get-ClipFormats) -contains 'CanUploadToCloudClipboard'))
    Shot 'RF10-copiado'
    Click-Name -Name 'Mostrar contraseña'
    Start-Sleep -Milliseconds 500
    Check 'El valor mostrado es el copiado' ((Get-Names) -contains $pw)
    Click-Name -Name 'Ocultar contraseña'
    Start-Sleep -Milliseconds 300
    Check 'Al ocultar, el valor sale del arbol de la ventana' (-not ((Get-Names) -contains $pw))
    Pointer-Away
    $cleared = -1
    while ($sw.Elapsed.TotalSeconds -lt ($n + 15)) {
        if ((Get-Clip) -eq '') { $cleared = $sw.Elapsed.TotalSeconds; break }
        Start-Sleep -Milliseconds 200
    }
    Log ("TIEMPO portapapeles vacio a los {0:n1} s de copiar (N = {1})" -f $cleared, $n)
    Check 'RF-73: el portapapeles queda vacio pasados N s' ($cleared -ge ($n - 1) -and $cleared -le ($n + 3)) ("{0:n1} s" -f $cleared)
    # Camino mas corto: Enter directo en la busqueda abre el primer resultado.
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Press-Ctrl 'a'; Type-Text 'rf-tit'; Start-Sleep -Milliseconds 900
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $f3 = Focused-Desc
    Check 'D-5: Enter directo en la busqueda abre el primer resultado con el foco en "Copiar contraseña"' ($f3 -like '*Copiar contraseña*' -and (Get-Names) -contains $Rf.User) $f3
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    Check 'Y Enter copia la contrasena de esa entrada' ((Get-Clip) -ceq $Rf.Pass)
}
Step 'RF15 Bloquear (limpia el portapapeles) y desbloquear' {
    $ms = Lock-Timed
    Check 'RF-31: al bloquear, el portapapeles queda vacio' ((Get-Clip) -eq '')
    $u = Unlock-Timed 'tras bloquear, segunda medida'
    Check 'Tras desbloquear estan las dos entradas' ((Has-El -Name $Rf.Title -Type Button -Timeout 4) -and (Has-El -Name $Rf.Title2 -Type Button -Timeout 2))
    Show-Net 'boveda abierta'
}
Summary
