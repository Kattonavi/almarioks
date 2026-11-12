# R-1, paso 2: el EFECTO de la capa nativa sola (ejecutable de prueba sin la capa de la pagina), por gesto y por
# momento: (a) arranque en frio, antes de cualquier bloqueo o recarga, en dos arranques; (b) boveda abierta sin
# bloqueo previo; (c) tras bloquear y desbloquear. F5 va el ultimo de cada momento: una recarga cambia el regimen.
# Ejecutar con RF_VARIANT=vn-lib y VN_EXE=aislada, con la app instalada cerrada. No instala nada.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
if ($AppExe -ne $IsolatedExe) { throw 'Falta VN_EXE=aislada' }
function Same-Doc([string]$stage, [string]$d0, [string]$when) {
    $d = Doc-Id
    Row $stage "identidad del documento $when" $(if ($d -ne '' -and $d -eq $d0) { 'la misma que al arrancar (ninguna navegacion desde el arranque)' } else { "SENAL: cambio ($d0 -> $d)" })
}
function Fresh { if ((Get-AppProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 3 } }
function A-Run([int]$n) {
    Fresh
    Start-Checked "aislada, arranque $n"
    $d0 = Doc-Id
    $st = "(a) arranque $n, bienvenida"
    $c = Center-Of (Find-El -Id 'welcome-title')
    # El primer gesto es el que se mide: clic derecho en el arranque 1, Ctrl+P en el 2.
    if ($n -eq 1) {
        [void](RC-Row $st 'el titulo (primer gesto tras arrancar)' $c[0] $c[1] "VN20-a$n-clic-derecho-titulo")
        [void](Probe-Surface $st 'Ctrl+P' "VN20-a$n-ctrl-p")
    } else {
        [void](Probe-Surface $st 'Ctrl+P' "VN20-a$n-ctrl-p" 'primer gesto tras arrancar')
        [void](RC-Row $st 'el titulo' $c[0] $c[1] "VN20-a$n-clic-derecho-titulo")
    }
    $i = Get-WinInfo
    [void](RC-Row $st 'el fondo' ($i.X + 80) ($i.Y + $i.OuterH - 140))
    Probe-Many $st @('F3', 'Ctrl+G', 'Ctrl+J')
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    $st2 = "(a) arranque $n, Crear"
    $c = Center-Of (Find-El -Id 'create-password')
    [void](RC-Row $st2 'el campo de la contrasena maestra' $c[0] $c[1] "VN20-a$n-clic-derecho-campo")
    [void](Probe-Surface $st2 'Ctrl+P' '' 'foco en un campo')
    Same-Doc $st2 $d0 'antes de F5'
    $kept = Probe-ReloadKey $st2 'F5' { (Screen-Ext) -eq 'crear' } 'el ultimo; marcador = seguir en Crear'
    if (-not $kept) {
        # Hubo recarga: lo que sigue ya no es (a). Se repiten los gestos para ver si la navegacion cambio el regimen.
        [void](Find-El -Id 'welcome-title' -Timeout 8)
        $st3 = "(a') arranque $n, tras la recarga que provoco F5"
        $c = Center-Of (Find-El -Id 'welcome-title')
        [void](RC-Row $st3 'el titulo' $c[0] $c[1] "VN21-a$n-tras-recarga-clic-derecho")
        [void](Probe-Surface $st3 'Ctrl+P' "VN21-a$n-tras-recarga-ctrl-p")
        Probe-Many $st3 @('F3', 'Ctrl+J')
        [void](Probe-ReloadKey $st3 'F5' { $true } 'solo identidad del documento')
    }
    if ($n -eq 1) {
        # Controles del detector, al final para no tocar nada antes del primer gesto.
        if ((Screen-Ext) -eq 'crear') { Click-Name -Name 'Volver a la bienvenida'; [void](Find-El -Id 'welcome-title' -Timeout 5) }
        $f0 = Focused-Desc; Press 'TAB'; Start-Sleep -Milliseconds 250; $f1 = Focused-Desc
        Check 'CONTROL: las teclas llegan a la pagina (Tab mueve el foco)' ($f1 -ne $f0) "$f0 -> $f1"
        Click-Name -Like 'Abrir una bóveda*'
        Start-Sleep -Milliseconds 1800
        $sig = @(Surface-Signal)
        Check 'CONTROL POSITIVO: el detector ve un dialogo nativo' (@($sig | Where-Object { $_ -like 'dialogo nativo #32770*' }).Count -eq 1) ($sig -join '; ')
        Press 'ESC'; Start-Sleep -Milliseconds 900
        $r = [QA.Native]::VisibleRect((Get-AppHwnd))
        $new = @(RC-Win32 ($r.Left + 260) ($r.Top + 16) 'VN22-control-menu-del-sistema')
        Check 'CONTROL POSITIVO: el detector por ventanas ve el menu del sistema (#32768)' ($new -contains '#32768') ($new -join ', ')
    }
    Check "Cierre del ejecutable de prueba (arranque $n)" (Close-App 20)
    Start-Sleep -Seconds 3
}
function Vault-Native([string]$stage, [string]$prefix, [string]$d0) {
    Open-Entry $RootName $Rf.Title
    $shown = Show-Pass
    $pt = Notes-Point
    [void](RC-Row $stage "el texto de las notas (contrasena mostrada=$shown)" $pt[0] $pt[1] "$prefix-clic-derecho-notas")
    $c = Center-Of (Find-El -Id 'vault-search')
    [void](RC-Row $stage 'el campo de busqueda' $c[0] $c[1] "$prefix-clic-derecho-campo")
    $i = Get-WinInfo
    [void](RC-Row $stage 'el fondo de la barra de grupos' ($i.X + 120) ($i.Y + $i.OuterH - 160))
    $shown = Show-Pass
    [void](Probe-Surface $stage 'Ctrl+P' "$prefix-ctrl-p" "contrasena mostrada=$shown")
    Probe-Many $stage @('F3', 'Ctrl+G', 'Ctrl+J')
    if ($d0) { Same-Doc $stage $d0 'antes de F5' }
    Fill (Find-El -Id 'vault-search') 'zz-marca'; Start-Sleep -Milliseconds 600
    [void](Probe-ReloadKey $stage 'F5' $SearchMarker 'el ultimo; foco en la busqueda')
    $s = Find-El -Id 'vault-search' -Timeout 3 -Quiet
    if ($null -ne $s) { Click-El $s; Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 400 }
}
Step 'VN10 (a) Arranque en frio 1' -Continue { A-Run 1 }
Step 'VN11 (a) Arranque en frio 2' -Continue { A-Run 2 }
Step 'VN12 (b) Boveda abierta sin bloqueo previo y (c) tras bloquear y desbloquear' -Continue {
    Fresh
    Start-Checked 'aislada, arranque 3'
    $d0 = Doc-Id
    Click-Name -Name 'Abrir rf-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    Fill-Id 'unlock-password' $RfMaster -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    $stB = '(b) boveda abierta, sin bloqueo ni recarga en esta ejecucion'
    Same-Doc $stB $d0 'tras desbloquear'
    Vault-Native $stB 'VN30' $d0
    Ensure-VaultOpen
    Start-Sleep -Milliseconds 1500
    $d1 = Doc-Id
    [void](Lock-Timed)
    $d2 = Doc-Id
    Check 'CONTROL POSITIVO: bloquear recarga la webview (la identidad del documento cambia)' ($d2 -ne '' -and $d2 -ne $d1) "$d1 -> $d2"
    Fill-Id 'unlock-password' $RfMaster -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Vault-Native '(c) boveda abierta, tras bloquear y desbloquear' 'VN40' ''
}
Step 'VN13 R-4, control complementario sobre este ejecutable (tampoco trae el arreglo del valor mostrado)' -Continue {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $st = 'R-4, ejecutable de prueba sin el arreglo'
    [void](Try-Copy $st 'doble clic sobre el valor' $SelDouble $Rf.Pass)
    [void](Try-Copy $st 'arrastre de punta a punta del valor' $SelDrag $Rf.Pass)
    Generator-Copy $st
}
Step 'VN14 Cerrar el ejecutable de prueba y limpiar el portapapeles' {
    [System.Windows.Forms.Clipboard]::Clear()
    $n = Purge-Mine
    Start-Sleep -Milliseconds 2000
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20) }
    Start-Sleep -Seconds 2
    Check 'El ejecutable de prueba no queda corriendo' ((Any-AlmarioProc).Count -eq 0) "borrados del historial: $n; canario en el historial: $(History-Has 'CENTELLA')"
}
Rows-Summary
Summary
