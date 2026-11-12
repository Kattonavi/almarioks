# Cierre de la superficie de navegador (3/4): lo que debe seguir funcionando (busqueda propia, pegar, copiar,
# deshacer, Tab, Escape, flechas, copiar contrasena y su limpieza), el trio minimo tras un segundo ciclo de bloqueo y
# tras reiniciar la app, y la posicion de la ventana en el segundo arranque. Termina con cierre normal. RF_VARIANT=vc-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'VC11 Lo que debe seguir funcionando' -Continue {
    Ensure-VaultOpen
    Select-Group $RootName
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Check 'Ctrl+K lleva a la busqueda propia' ((Focused-Desc) -like '*vault-search*') (Focused-Desc)
    Press 'TAB'; Start-Sleep -Milliseconds 250
    $ft = Focused-Desc
    Check 'Tab saca el foco de la busqueda' ($ft -notlike '*vault-search*') $ft
    Keys @(0x11, 0x46); Start-Sleep -Milliseconds 300
    Check 'Ctrl+F lleva a la busqueda propia' ((Focused-Desc) -like '*vault-search*') (Focused-Desc)
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text 'Nueva entrada en *' 5)
    Set-ClipExternal $Rf.Title2
    Fill (Find-El -Id 'entry-title') ''
    Press-Ctrl 'v'; Start-Sleep -Milliseconds 400
    $v1 = Get-Value (Find-El -Id 'entry-title')
    Check 'Ctrl+V pega en el titulo un texto copiado por otro proceso' ($v1 -ceq $Rf.Title2) $v1
    Set-ClipExternal 'otra-cosa-vc'
    Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 400
    Check 'Ctrl+A y Ctrl+C copian el contenido del campo' ((Get-Clip) -ceq $Rf.Title2) "portapapeles de $((Get-Clip).Length) caracteres"
    Press 'END'; Type-Text '-x'; Start-Sleep -Milliseconds 300
    $v2 = Get-Value (Find-El -Id 'entry-title')
    Press-Ctrl 'z'; Start-Sleep -Milliseconds 400
    $v3 = Get-Value (Find-El -Id 'entry-title')
    Check 'Ctrl+Z deshace lo ultimo tecleado' ($v2 -ceq "$($Rf.Title2)-x" -and $v3 -cne $v2) "$v2 -> $v3"
    $c = Center-Of (Find-El -Id 'entry-title')
    [void](RC-Row 'formulario de entrada' 'el campo del titulo (consecuencia aceptada: sin "Pegar" con el raton)' $c[0] $c[1] 'VC30-formulario-clic-derecho')
    Fill-Id 'entry-title' $Rf.Title2
    Fill-Id 'entry-password' $Rf.Pass2 -Secret
    $s = Save-EntryTimed
    Check 'La entrada nueva se guarda' ($s.Msg -ge 0 -and $s.Disk -ge 0)
    Start-Sleep -Milliseconds 800
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Type-Text $Rf.Partial; Start-Sleep -Milliseconds 900
    Press 'DOWN'; Start-Sleep -Milliseconds 350
    $f1 = Focused-Desc
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $f2 = Focused-Desc
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    $clip = Get-Clip
    Check 'Flecha abajo, Enter y Enter: el portapapeles tiene la contrasena de la entrada' ($f1 -like "*$($Rf.Title)*" -and $f2 -like '*Copiar contraseña*' -and $clip -ceq $Rf.Pass) "$f1 -> $f2"
    Check 'Lo copiado lleva los formatos de exclusion' ((Get-ClipFormats) -contains 'ExcludeClipboardContentFromMonitorProcessing')
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $sv = Get-Value (Find-El -Id 'vault-search')
    Check 'Escape limpia la busqueda' ([string]::IsNullOrEmpty($sv)) "valor: '$sv'"
    Pointer-Away
    $cleared = -1
    while ($sw.Elapsed.TotalSeconds -lt 35) { if ((Get-Clip) -eq '') { $cleared = $sw.Elapsed.TotalSeconds; break }; Start-Sleep -Milliseconds 200 }
    Check 'El portapapeles queda vacio a los ~20 s' ($cleared -ge 19 -and $cleared -le 23) ("{0:n1} s" -f $cleared)
}
Step 'VC12 Segundo ciclo de bloqueo: menu, Ctrl+P y F5' -Continue {
    Ensure-VaultOpen
    Start-Sleep -Milliseconds 2500
    $d0 = Doc-Id
    $lockMs = Lock-Timed
    Fill-Id 'unlock-password' $RfMaster -Secret
    Click-Name -Name 'Desbloquear'
    Check 'Bloquear y desbloquear por segunda vez' ($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 20 -Quiet)) "bloqueo en $lockMs ms; documento nuevo = $((Doc-Id) -ne $d0)"
    Trio 'boveda abierta, tras el segundo bloqueo' 'VC31'
}
Step 'VC13 Reiniciar la app: posicion, y el trio otra vez' -Continue {
    Start-Sleep -Milliseconds 2500
    $closed = Close-App 20
    Start-Sleep -Seconds 3
    Check 'Cierre normal; portapapeles vacio' ($closed -and (Get-Clip) -eq '')
    $ms = Start-AppTimed 'segundo arranque de esta compilacion'
    $w = Where-Window 'segundo arranque, sin tocarla'
    Check 'D-12 (segundo arranque): 1120 x 720, dentro del area util y centrada' ($w.Inside -and [math]::Abs($w.Dx) -le 12 -and [math]::Abs($w.Dy) -le 12 -and [math]::Abs($w.W - 1120) -le 1) "desvio $($w.Dx), $($w.Dy) px"
    $st = 'bienvenida, tras reiniciar'
    $c = Center-Of (Find-El -Id 'welcome-title')
    [void](RC-Row $st 'el titulo' $c[0] $c[1])
    [void](Probe-Surface $st 'Ctrl+P')
    [void](Probe-ReloadKey $st 'F5' { $true } 'solo identidad del documento')
    Trio 'boveda abierta, tras reiniciar' 'VC32'
    Open-Entry $RootName $Rf.Title
    Shot 'VC33-boveda-al-final'
    Start-Sleep -Milliseconds 2500
    $closed = Close-App 20
    Start-Sleep -Seconds 3
    Check 'Cierre normal al final' $closed
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length) bytes" }
}
Rows-Summary
Summary
