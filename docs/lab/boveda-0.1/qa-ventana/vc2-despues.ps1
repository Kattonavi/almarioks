# Cierre de la superficie de navegador (2/4): el bloqueo recarga la ventana. Control positivo del detector de
# recarga, tanda con la boveda bloqueada, desbloqueo tecleando @ y el euro con su combinacion real de teclas, y la
# tanda completa DESPUES de bloquear y desbloquear. Parte de la boveda abierta que deja vc1. RF_VARIANT=vc-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'VC7 Bloquear: la recarga se ve en la identidad del documento' {
    if ((Screen-Ext) -ne 'boveda abierta') { throw "Se esperaba la boveda abierta y la pantalla es $(Screen-Ext): no vale como 'antes de cualquier bloqueo'" }
    # Control positivo del detector de menus sin UIA: el menu del sistema de la barra de titulo.
    $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    $new = @(RC-Win32 ($r.Left + 260) ($r.Top + 16) 'VC12-control-menu-del-sistema-win32')
    Check 'CONTROL POSITIVO: el detector por ventanas ve el menu del sistema (#32768) al hacer clic derecho en la barra de titulo' ($new -contains '#32768') ($new -join ', ')
    # Antes de cualquier bloqueo, con el detector por ventanas: texto de las notas con la contrasena mostrada.
    Open-Entry $RootName $Rf.Title
    $shown = Show-Pass
    $pt = Notes-Point
    [void](RC-Row 'boveda abierta, antes de bloquear (segundo detector)' "el texto de las notas (contrasena mostrada=$shown)" $pt[0] $pt[1] 'VC13-antes-de-bloquear-clic-derecho-notas')
    $c = Center-Of (Find-El -Id 'vault-search')
    [void](RC-Row 'boveda abierta, antes de bloquear (segundo detector)' 'el campo de busqueda' $c[0] $c[1] 'VC14-antes-de-bloquear-clic-derecho-campo')
    $d0 = Doc-Id
    Log "   identidad del documento: al arrancar $(Get-State 'doc-inicial'); ahora $d0 (iguales = ninguna recarga desde el arranque: $((Get-State 'doc-inicial') -eq $d0))"
    Check 'Desde el arranque hasta aqui el documento es el mismo: ninguna tecla lo recargo y no hubo bloqueo' ((Get-State 'doc-inicial') -eq $d0)
    Start-Sleep -Milliseconds 1500
    [void](Lock-Timed)
    $d1 = Doc-Id
    Check 'CONTROL POSITIVO: tras bloquear (que recarga la webview) la identidad del documento cambia' ($d1 -ne '' -and $d1 -ne $d0) "$d0 -> $d1"
}
Step 'VC8 Boveda bloqueada' -Continue {
    $st = 'boveda bloqueada'
    $c = Center-Of (Find-El -Id 'unlock-title')
    [void](RC-Row $st 'el titulo' $c[0] $c[1] 'VC15-bloqueada-clic-derecho')
    $c = Center-Of (Find-El -Id 'unlock-password')
    [void](RC-Row $st 'el campo de la contrasena maestra' $c[0] $c[1])
    $i = Get-WinInfo
    [void](RC-Row $st 'el fondo' ($i.X + 80) ($i.Y + $i.OuterH - 140))
    Fill-Id 'unlock-password' 'clave-equivocada-vc' -Secret
    Click-Name -Name 'Desbloquear'
    $marker = { @(Get-Names | Where-Object { $_ -like '*no abren esta bóveda*' }).Count -gt 0 }
    [void](Wait-Text '*no abren esta bóveda*' 8)
    Check 'Bloqueada: el error de llave incorrecta esta en pantalla (marcador de recarga)' ([bool](& $marker))
    foreach ($l in @('F5', 'Ctrl+R', 'Ctrl+Shift+R', 'Ctrl+F5')) { [void](Probe-ReloadKey $st $l $marker) }
    [void](Probe-Surface $st 'Ctrl+P' 'VC16-bloqueada-ctrl-p')
    Probe-Many $st @('Ctrl+Shift+P', 'Ctrl+S', 'Ctrl+F', 'F3', 'Ctrl+G', 'Ctrl+J', 'Ctrl+U', 'F12', 'Shift+F10', 'Tecla Menu')
    Log "   al final: pantalla = $(Screen-Ext); marcador presente = $([bool](& $marker))"
}
Step 'VC9 Desbloquear tecleando @ y el euro con su combinacion real (AltGr)' -Continue {
    $at = Combo-For '@'; $eu = Combo-For ([char]0x20AC)
    $euText = if ($eu) { $eu.Text + ', AltGr = ' + $eu.AltGr } else { 'sin tecla en esta distribucion: se teclea por Unicode' }
    $atText = $at.Text + ', AltGr = ' + $at.AltGr
    Log "TECLADO: distribucion de la ventana = $($at.Layout); arroba = $atText; euro = $euText"
    Click-El (Find-El -Id 'unlock-password')
    Press-Ctrl 'a'; Press 'DEL'
    Type-Text "VC-Maestra-Cierre-39-$([char]0x00F1)" -Secret
    if ($eu) { [void](Type-Real ([char]0x20AC)) } else { Type-Text "$([char]0x20AC)" -Secret }
    [void](Type-Real '@')
    Type-Text 'fin' -Secret
    Click-Name -Name 'Desbloquear'
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 20 -Quiet)
    Check 'La maestra tecleada con @ y euro por su combinacion real de teclas abre la boveda' $ok "arroba = $atText; euro = $euText"
    Row 'boveda bloqueada' "teclear @ ($($at.Text)) y euro en la contrasena maestra" $(if ($ok) { 'la boveda abre' } else { 'SENAL: la boveda NO abre' })
    if (-not $ok) { Fill-Id 'unlock-password' $RfMaster -Secret; Click-Name -Name 'Desbloquear'; [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20) }
}
Step 'VC10 Boveda abierta DESPUES de bloquear y desbloquear (ventana recargada)' -Continue {
    Vault-Battery 'boveda abierta, tras bloquear y desbloquear' 'VC20'
    Log "   pantalla al terminar: $(Screen-Ext)"
}
Rows-Summary
Summary
