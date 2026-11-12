# Verificacion del cierre de la superficie de navegador (D-9 a D-12): variante de rf-lib.ps1 (se carga con
# RF_VARIANT=vc-lib). Canarios, carpetas y un detector propio. Casi todo debe dar "nada": por eso cada senal del
# detector tiene su control positivo en la misma corrida (vc1) o en los registros de la corrida anterior.
$script:Shots = Join-Path $Repo '.impeccable\review\qa3'
New-Item -ItemType Directory -Force $Shots | Out-Null
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa3-20261112'
$script:RfMaster = "VC-Maestra-Cierre-39-$([char]0x00F1)$([char]0x20AC)@fin"
$script:RfOther = 'VC-Repeticion-Otra-17'
$script:Rf = @{
    Title = 'VIGIA-VC-Titulo'; User = 'VIGIA-VC-usuario@example.org'; Url = 'https://example.org/VIGIA-VC-url'
    Notes = 'VIGIA-VC-notas reservadas'; Tag = 'VIGIA-VC-etiqueta'; Group = 'VIGIA-VC-Grupo'; Search = 'VIGIA-VC-busqueda'
    Partial = 'vigia-vc-tit'; Pass = 'VIGIA-VC-Clave-73'; Title2 = 'VIGIA-VC-Pegada'; Pass2 = 'VIGIA-VC-Clave-pegada-11'
}
$script:RfVault = Join-Path $Work 'rf-uno.kdbx'
$script:Vault = $RfVault
$script:Master = $RfMaster
function Rf-Secrets {
    $plain = @{ 'titulo' = $Rf.Title; 'usuario' = $Rf.User; 'URL' = $Rf.Url; 'notas' = $Rf.Notes; 'etiqueta' = $Rf.Tag; 'grupo' = $Rf.Group
        'busqueda parcial' = $Rf.Partial; 'contrasena de la entrada' = $Rf.Pass; 'titulo pegado' = $Rf.Title2; 'contrasena de la entrada pegada' = $Rf.Pass2
        'prefijo VIGIA-VC' = 'VIGIA-VC'; 'prefijo de la corrida anterior' = 'TESTIGO-RF' }
    New-Secrets $plain @{ 'maestra' = $RfMaster; 'repeticion distinta' = $RfOther }
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
namespace QA3 {
  public static class Kb {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern short VkKeyScanEx(char ch, IntPtr hkl);
    [DllImport("user32.dll")] public static extern IntPtr GetKeyboardLayout(uint thread);
  }
}
"@

# ---------- Tabla de resultados ----------
$script:Rows = New-Object System.Collections.Generic.List[string]
function Row([string]$stage, [string]$gesture, [string]$result) { $Rows.Add("$stage | $gesture | $result"); Log "FILA | $stage | $gesture | $result" }
function Rows-Summary {
    $bad = @($Rows | Where-Object { $_ -match '\| (SENAL|MENU|RECARGO|NO PROBADO)' })
    Log "--- filas: $($Rows.Count); con senal, menu, recarga o sin probar: $($bad.Count) ---"
    foreach ($b in $bad) { Log "  ! $b" }
}

# ---------- Detector ----------
$script:K = @{
    'Ctrl+P' = @(0x11, 0x50); 'Ctrl+Shift+P' = @(0x11, 0x10, 0x50); 'Ctrl+S' = @(0x11, 0x53); 'F5' = @(0x74); 'Ctrl+R' = @(0x11, 0x52)
    'Ctrl+Shift+R' = @(0x11, 0x10, 0x52); 'Ctrl+F5' = @(0x11, 0x74); 'F3' = @(0x72); 'Ctrl+G' = @(0x11, 0x47); 'Ctrl+F' = @(0x11, 0x46)
    'Ctrl+J' = @(0x11, 0x4A); 'Ctrl+U' = @(0x11, 0x55); 'F12' = @(0x7B); 'Ctrl+Shift+I' = @(0x11, 0x10, 0x49); 'Ctrl+Shift+J' = @(0x11, 0x10, 0x4A)
    'Ctrl+Shift+C' = @(0x11, 0x10, 0x43); 'Ctrl+Shift+S' = @(0x11, 0x10, 0x53); 'Alt+Izquierda' = @(0x12, 0x25); 'Shift+F10' = @(0x10, 0x79); 'Tecla Menu' = @(0x5D)
}
function Screen-Ext {
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.25) { return 'boveda abierta' }
    if (Has-El -Id 'unlock-password' -Timeout 0.25) { return 'desbloqueo' }
    if (Has-El -Id 'create-title' -Timeout 0.25) { return 'crear' }
    if (Has-El -Id 'welcome-title' -Timeout 0.25) { return 'bienvenida' }
    return 'otra'
}
# Identidad del documento web: cambia cuando la pagina se recarga (control positivo: el bloqueo recarga la webview).
function Doc-Id {
    $d = Find-El -Id 'RootWebArea' -Type Document -Timeout 2 -Quiet
    if ($null -eq $d) { return '' }
    try { return (($d.GetRuntimeId()) -join '.') } catch { return '' }
}
function Browser-Names {
    @(Get-Names | Where-Object { $_ -in @('Imprimir', 'Impresora', 'Buscar en la página', 'Descargas', 'Guardar como') -or $_ -like 'Vista previa*' -or $_ -like 'Total: * hoja*' } | Select-Object -Unique)
}
# Senales de que el navegador hizo algo: primer plano fuera de la ventana de la app (de cualquier dueno), un dialogo
# nativo #32770 de los procesos de la app, el foco en un menu o en una pieza del navegador, o nombres del navegador
# dentro de la ventana (vista de impresion, buscar en la pagina, descargas).
function Surface-Signal {
    $sig = @()
    $app = Get-AppHwnd
    if ((Get-AppProc).Count -ne 1 -or $app -eq [IntPtr]::Zero) { return @('la app no esta viva') }
    $fg = [QA.Native]::GetForegroundWindow()
    if ($fg -ne $app) {
        [uint32]$owner = 0; [void][QA.Native]::GetWindowThreadProcessId($fg, [ref]$owner)
        $sig += "primer plano fuera de la app: clase $([QA.Native]::ClassOf($fg)), proceso $((Get-Process -Id $owner -ErrorAction SilentlyContinue).Name)"
    }
    $dialogs = 0
    foreach ($procId in Get-AppPids) { foreach ($w in [QA.Native]::WindowsOfPid([uint32]$procId)) { if ([QA.Native]::ClassOf($w) -eq '#32770') { $dialogs++ } } }
    if ($dialogs) { $sig += "dialogo nativo #32770 x$dialogs" }
    $f = Focused-Desc
    if ($f -match '^\[(MenuItem|Menu|MenuBar)\]' -or $f -match 'Buscar en la página|"Imprimir"|"Descargas"|id=view_|printerSelector') { $sig += "foco en $f" }
    $bn = @(Browser-Names)
    if ($bn.Count) { $sig += "nombres de navegador: $($bn -join ', ')" }
    return $sig
}
function Clear-Signal {
    for ($k = 0; $k -lt 3; $k++) {
        if (@(Surface-Signal).Count -eq 0) { return }
        Press 'ESC'; Start-Sleep -Milliseconds 700
    }
    Log "   AVISO: la senal sigue tras tres Escape: $(@(Surface-Signal) -join '; ')"
}
function Probe-Surface([string]$stage, [string]$label, [string]$shot = '', [string]$note = '') {
    [void](Focus-App)
    $pre = @(Surface-Signal)
    if ($pre.Count) { Log "   (antes de $label ya habia senal: $($pre -join '; '))"; Clear-Signal }
    $f0 = Focused-Desc; $s0 = Screen-Ext
    Keys ([byte[]]$K[$label]); Start-Sleep -Milliseconds 1300
    $sig = @(Surface-Signal); $f1 = Focused-Desc; $s1 = Screen-Ext
    if ($s1 -ne $s0) { $sig += "la pantalla cambio: $s0 -> $s1" }
    $res = if ($sig.Count) { 'SENAL: ' + ($sig -join '; ') } else { 'nada' }
    $gesture = if ($note) { "$label ($note)" } else { $label }
    Log "TECLA [$stage] $gesture -> $res | foco: $f0 -> $f1"
    Row $stage $gesture $res
    if ($sig.Count) { Shot-Area ("VC-senal-" + (("$stage-$label") -replace '[^A-Za-z0-9]+', '-')); Clear-Signal }
    elseif ($shot) { Shot-Area $shot }
    return ($sig.Count -eq 0)
}
function Probe-Many([string]$stage, [string[]]$labels) { foreach ($l in $labels) { [void](Probe-Surface $stage $l) } }
# Recarga: el marcador (un estado que solo vive en la pagina) debe seguir, el documento debe ser el mismo y la
# pantalla tambien.
function Probe-ReloadKey([string]$stage, [string]$label, [scriptblock]$marker, [string]$note = '') {
    [void](Focus-App)
    $gesture = if ($note) { "$label ($note)" } else { $label }
    if (-not (& $marker)) { Row $stage $gesture 'NO PROBADO: el marcador no estaba puesto'; return $false }
    $d0 = Doc-Id; $s0 = Screen-Ext
    Keys ([byte[]]$K[$label]); Start-Sleep -Milliseconds 1800
    $still = [bool](& $marker); $d1 = Doc-Id; $s1 = Screen-Ext
    $sig = @(Surface-Signal)
    $reloaded = (-not $still) -or ($d1 -ne $d0) -or ($s1 -ne $s0)
    $res = if ($reloaded) { "RECARGO: marcador presente=$still; mismo documento=$($d1 -eq $d0); pantalla $s0 -> $s1" }
    elseif ($sig.Count) { 'SENAL: ' + ($sig -join '; ') }
    else { 'nada (marcador intacto, mismo documento, misma pantalla)' }
    Log "RECARGA [$stage] $gesture -> $res"
    Row $stage $gesture $res
    if ($sig.Count) { Clear-Signal }
    return (-not $reloaded)
}
# Clic derecho con la misma funcion que en la corrida anterior encontro los menus de WebView2 (Probe-RightClick).
# Segundo detector, sin UIA: un menu emergente es una ventana nueva de alguno de los procesos de la app (el menu del
# sistema es de clase #32768). Se anadio porque en vc1 Probe-RightClick no leyo por UIA el menu del sistema que la
# captura si muestra. Cada clic derecho deja ademas su captura, tomada con el menu (si lo hay) abierto.
function App-Windows {
    $map = @{}
    foreach ($procId in Get-AppPids) { foreach ($w in [QA.Native]::WindowsOfPid([uint32]$procId)) { $map[[string]$w] = [QA.Native]::ClassOf($w) } }
    return $map
}
$script:RcCount = 0
function RC-Win32([int]$x, [int]$y, [string]$shot = '') {
    [void](Focus-App); Start-Sleep -Milliseconds 150
    $w0 = App-Windows
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 120
    [QA.Native]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [QA.Native]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 900
    $w1 = App-Windows
    $new = @($w1.Keys | Where-Object { -not $w0.ContainsKey($_) } | ForEach-Object { $w1[$_] })
    if ($shot) { Shot-Area $shot }
    Press 'ESC'; Start-Sleep -Milliseconds 400
    return $new
}
function RC-Row([string]$stage, [string]$what, [int]$x, [int]$y, [string]$shot = '') {
    $m = Probe-RightClick "$stage / $what" $x $y ''
    $sig = @(Surface-Signal)
    $script:RcCount++
    if (-not $shot) { $shot = 'VC-rc-{0:00}-{1}' -f $RcCount, (($stage -replace '[^A-Za-z0-9]+', '-')) }
    $new = @(RC-Win32 $x $y $shot)
    $res = if ($m.Items.Count -gt 0) { "MENU: $($m.Items -join ' | ')" } elseif ($new.Count) { "MENU o ventana nueva de clase: $($new -join ', ')" } elseif ($sig.Count) { 'SENAL: ' + ($sig -join '; ') } else { 'nada' }
    Row $stage "clic derecho sobre $what" "$res [captura $shot]"
    if ($sig.Count) { Clear-Signal }
    return ($m.Items.Count -eq 0 -and $sig.Count -eq 0 -and $new.Count -eq 0)
}
function Center-Of($el) { $r = $el.Current.BoundingRectangle; return @([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) }
function Notes-Point { $r = (Find-El -Name $Rf.Notes -Type Text -Timeout 3).Current.BoundingRectangle; return @([int]($r.X + 40), [int]($r.Y + $r.Height / 2)) }
# Deja la contrasena de la entrada abierta recien mostrada (20 s por delante) y dice si esta en el arbol.
function Show-Pass {
    $hide = Find-El -Name 'Ocultar contraseña' -Type Button -Timeout 0.3 -Quiet
    if ($null -ne $hide) { Click-El $hide; Start-Sleep -Milliseconds 300 }
    $b = Find-El -Name 'Mostrar contraseña' -Type Button -Timeout 2 -Quiet
    if ($null -eq $b) { return $false }
    Click-El $b; Start-Sleep -Milliseconds 450
    return ((Get-Names) -contains $Rf.Pass)
}
$script:SearchMarker = { $s = Find-El -Id 'vault-search' -Timeout 2 -Quiet; ($null -ne $s) -and ((Get-Value $s) -eq 'zz-marca') }
# El trio minimo con la boveda abierta y la contrasena mostrada: menu contextual, Ctrl+P y F5.
function Trio([string]$stage, [string]$shotPrefix = '') {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $shown = Show-Pass
    $pt = Notes-Point
    [void](RC-Row $stage "el texto de las notas (contrasena mostrada=$shown)" $pt[0] $pt[1] $(if ($shotPrefix) { "$shotPrefix-clic-derecho" } else { '' }))
    $shown = Show-Pass
    [void](Probe-Surface $stage 'Ctrl+P' $(if ($shotPrefix) { "$shotPrefix-ctrl-p" } else { '' }) "contrasena mostrada=$shown")
    Fill (Find-El -Id 'vault-search') 'zz-marca'; Start-Sleep -Milliseconds 600
    [void](Probe-ReloadKey $stage 'F5' $SearchMarker 'foco en la busqueda')
    Click-El (Find-El -Id 'vault-search'); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 400
}
# La tanda completa con la boveda abierta.
function Vault-Battery([string]$stage, [string]$shotPrefix) {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    $fk = Focused-Desc
    Row $stage 'Ctrl+K (las teclas llegan a la pagina)' $(if ($fk -like '*vault-search*') { 'foco en la busqueda propia' } else { "NO PROBADO: foco en $fk" })
    $shown = Show-Pass
    $pt = Notes-Point
    [void](RC-Row $stage "el texto de las notas (contrasena mostrada=$shown)" $pt[0] $pt[1] "$shotPrefix-clic-derecho-notas")
    $shown = Show-Pass
    $pw = Find-El -Name $Rf.Pass -Type Text -Timeout 1 -Quiet
    if ($null -ne $pw) { $c = Center-Of $pw; [void](RC-Row $stage 'la contrasena mostrada' $c[0] $c[1]) } else { Row $stage 'clic derecho sobre la contrasena mostrada' 'NO PROBADO: el valor no estaba en el arbol' }
    [QA.Native]::ClickAt($pt[0], $pt[1]); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($pt[0], $pt[1]); Start-Sleep -Milliseconds 400
    [void](RC-Row $stage 'una palabra seleccionada de las notas' $pt[0] $pt[1] "$shotPrefix-clic-derecho-seleccion")
    $c = Center-Of (Find-El -Id 'vault-search')
    [void](RC-Row $stage 'el campo de busqueda' $c[0] $c[1])
    $i = Get-WinInfo
    [void](RC-Row $stage 'el fondo de la barra de grupos' ($i.X + 120) ($i.Y + $i.OuterH - 160))
    $shown = Show-Pass
    [void](Probe-Surface $stage 'Ctrl+P' "$shotPrefix-ctrl-p" "foco en un boton, contrasena mostrada=$shown")
    $shown = Show-Pass
    [void](Probe-Surface $stage 'Ctrl+Shift+P' '' "contrasena mostrada=$shown")
    $shown = Show-Pass
    [void](Probe-Surface $stage 'Ctrl+S' '' "contrasena mostrada=$shown")
    Probe-Many $stage @('Shift+F10', 'Tecla Menu')
    Fill (Find-El -Id 'vault-search') 'zz-marca'; Start-Sleep -Milliseconds 600
    foreach ($l in @('F5', 'Ctrl+R', 'Ctrl+Shift+R', 'Ctrl+F5')) { [void](Probe-ReloadKey $stage $l $SearchMarker 'foco en la busqueda') }
    [void](Probe-Surface $stage 'Ctrl+P' '' 'foco en un campo')
    Probe-Many $stage @('Shift+F10', 'Tecla Menu')
    Press 'TAB'; Start-Sleep -Milliseconds 200
    [void](Probe-ReloadKey $stage 'F5' $SearchMarker "foco en $(Focused-Desc)")
    Probe-Many $stage @('F3', 'Ctrl+G', 'Ctrl+J', 'Ctrl+U', 'F12', 'Ctrl+Shift+I', 'Ctrl+Shift+J', 'Ctrl+Shift+C', 'Ctrl+Shift+S', 'Alt+Izquierda')
    Log "   marcador de la busqueda al final de la tanda: presente = $([bool](& $SearchMarker)); pantalla = $(Screen-Ext)"
    [void](Probe-Surface $stage 'Ctrl+F')
    $ff = Focused-Desc
    Row $stage 'Ctrl+F: adonde va el foco' $(if ($ff -like '*vault-search*') { 'a la busqueda propia' } else { "a $ff" })
    Click-El (Find-El -Id 'vault-search'); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 400
}
# Combinacion real de teclas que escribe un caracter en la distribucion de teclado de la ventana (VkKeyScanEx).
function Combo-For([char]$ch) {
    [uint32]$owner = 0
    $tid = [QA.Native]::GetWindowThreadProcessId((Get-AppHwnd), [ref]$owner)
    $hkl = [QA3.Kb]::GetKeyboardLayout($tid)
    $r = [QA3.Kb]::VkKeyScanEx($ch, $hkl)
    if ($r -eq -1) { return $null }
    $state = ($r -shr 8) -band 0xFF
    $names = @(); if ($state -band 2) { $names += 'Ctrl' }; if ($state -band 4) { $names += 'Alt' }; if ($state -band 1) { $names += 'Shift' }
    [pscustomobject]@{ Vk = [byte]($r -band 0xFF); State = $state; AltGr = (($state -band 6) -eq 6); Layout = ('{0:X4}' -f ($hkl.ToInt64() -band 0xFFFF))
        Text = ((($names + ('VK 0x{0:X2}' -f ($r -band 0xFF))) -join '+')) }
}
function Type-Real([char]$ch) {
    $c = Combo-For $ch
    if ($null -eq $c) { return $false }
    $mods = @(); if ($c.State -band 2) { $mods += 0x11 }; if ($c.State -band 4) { $mods += 0x12 }; if ($c.State -band 1) { $mods += 0x10 }
    Keys ([byte[]]($mods + $c.Vk)); Start-Sleep -Milliseconds 80
    return $true
}
function Where-Window([string]$label) {
    $i = Get-WinInfo
    $r = [QA.Native]::VisibleRect($i.Hwnd)
    $wa = [System.Windows.Forms.Screen]::FromHandle($i.Hwnd).WorkingArea
    $dx = [int](($r.Left + $r.Right) / 2 - ($wa.Left + $wa.Right) / 2); $dy = [int](($r.Top + $r.Bottom) / 2 - ($wa.Top + $wa.Bottom) / 2)
    $inside = ($r.Bottom -le $wa.Bottom -and $r.Right -le $wa.Right -and $r.Top -ge $wa.Top -and $r.Left -ge $wa.Left)
    Log "VENTANA [$label]: interior logico $($i.LogicalW) x $($i.LogicalH); borde visible fisico ($($r.Left), $($r.Top)) a ($($r.Right), $($r.Bottom)); area util ($($wa.Left), $($wa.Top)) a ($($wa.Right), $($wa.Bottom)); desvio del centro del area util: $dx, $dy px; dentro = $inside"
    return [pscustomobject]@{ Inside = $inside; Dx = $dx; Dy = $dy; W = $i.LogicalW; H = $i.LogicalH }
}
