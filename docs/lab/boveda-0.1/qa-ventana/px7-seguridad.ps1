# Pase de interaccion (7): pasada corta de seguridad sobre el build nuevo, cada cosa con su control positivo.
#  (i)  El valor de contrasena MOSTRADO en el detalle nuevo no sale por el navegador (doble clic, arrastre, Ctrl+A + Ctrl+C).
#  (ii) Clic derecho y Ctrl+P no hacen nada en la boveda, en el dialogo de entrada y en crear.
# Argumento opcional: 70, 71, 72, 73 o 74 para repetir un solo paso.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { '' }
$UserNow = 'FAROPX-usuario-editado@example.com'
function Detail-Text([string]$like) { @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -like $like -and (LRect $_) -and (LRect $_).X -ge 572 }) | Select-Object -First 1 }
function Open-Canary { Ensure-Cuentas; Pick-Group 'Personal'; Pick-Entry $Rf.Title }
# Clic derecho SIN Escape despues (en un dialogo, Escape lo cerraria). Senales: ventana nueva de algun proceso de la
# app (el menu del sistema es #32768; el de WebView2, otra ventana), elementos de menu bajo el puntero o en la pagina.
function RC-Plain([string]$stage, [string]$what, [int]$x, [int]$y) {
    [void](Focus-App); Start-Sleep -Milliseconds 120
    $w0 = App-Windows
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 120
    [QA.Native]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [QA.Native]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 1000
    $w1 = App-Windows
    $new = @($w1.Keys | Where-Object { -not $w0.ContainsKey($_) } | ForEach-Object { $w1[$_] })
    $miCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuItem)
    $items = New-Object System.Collections.Generic.List[string]
    try {
        $cur = $AE::FromPoint((New-Object System.Windows.Point(($x + 14), ($y + 14))))
        for ($k = 0; $k -lt 6 -and $null -ne $cur; $k++) {
            if ((Type-Of $cur) -in @('Menu', 'MenuBar', 'Window', 'Pane') -and @($cur.FindAll($TreeScope::Descendants, $miCond)).Count -gt 0) { foreach ($m in $cur.FindAll($TreeScope::Descendants, $miCond)) { $items.Add($m.Current.Name) }; break }
            $cur = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($cur)
        }
    } catch { }
    try { foreach ($m in (Get-Root).FindAll($TreeScope::Descendants, $miCond)) { if (-not $items.Contains($m.Current.Name)) { $items.Add($m.Current.Name) } } } catch { }
    $sig = @(Surface-Signal)
    $script:RcCount++
    $shot = 'PX-rc-{0:00}-{1}' -f $RcCount, ($stage -replace '[^A-Za-z0-9]+', '-')
    Shot-Area $shot
    $res = if ($items.Count) { "MENU: $($items -join ' | ')" } elseif ($new.Count) { "MENU o ventana nueva de clase: $($new -join ', ')" } elseif ($sig.Count) { 'SENAL: ' + ($sig -join '; ') } else { 'nada' }
    Row $stage "clic derecho sobre $what" "$res [captura $shot]"
    if ($items.Count -or $new.Count -or $sig.Count) { Press 'ESC'; Start-Sleep -Milliseconds 500 }
    return [pscustomobject]@{ Items = @($items); New = $new; Nothing = ($items.Count -eq 0 -and $new.Count -eq 0 -and $sig.Count -eq 0) }
}
function Point-Of($el, [double]$fx = 0.5) { $r = $el.Current.BoundingRectangle; return @([int]($r.X + $r.Width * $fx), [int]($r.Y + $r.Height / 2)) }

if (-not $only -or $only -eq '70') { Step 'PX70 (i) El valor mostrado en el detalle no sale por el navegador' -Continue {
    $st = '(i) detalle'
    Open-Canary
    [void](Purge-Mine)
    $shown = Show-Pass
    $valEl = Find-El -Name $Rf.Pass -Type Text -Timeout 1 -Quiet
    Check 'Al pulsar "Mostrar contrasena" el valor queda a la vista y en el arbol (lo lee un lector de pantalla)' ($shown -and $null -ne $valEl) "rectangulo: $(if ($valEl) { LRect-Text $valEl })"
    Shot-Client 'i-valor-mostrado'
    # CONTROL POSITIVO: el mismo gesto sobre un texto que no es secreto (el usuario) SI copia.
    $u = Detail-Text $UserNow
    Set-ClipExternal 'centinela-px-control'; Start-Sleep -Milliseconds 300
    $p = Point-Of $u 0.75
    [QA.Native]::ClickAt($p[0], $p[1]); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($p[0], $p[1]); Start-Sleep -Milliseconds 350
    Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $c = "$(Get-Clip)".Trim()
    $ctl = ($c -cne 'centinela-px-control' -and $c.Length -ge 3 -and $UserNow.Contains($c))
    Check 'CONTROL POSITIVO: doble clic + Ctrl+C sobre el usuario (no secreto) si copia por el navegador' $ctl "copio $($c.Length) caracteres, parte del usuario = $($UserNow.Contains($c)); formatos de exclusion: $((@($ExclFormats | Where-Object { @(Get-ClipFormats) -contains $_ })) -join ', ')"
    $selAcross = { $r = Pass-Rect; $a = (Detail-Text $UserNow).Current.BoundingRectangle; $b = (Detail-Text 'Creada el').Current.BoundingRectangle
        Mouse-Drag ([int]($a.X + 2)) ([int]($a.Y + $a.Height / 2)) ([int]($b.X + $b.Width + 150)) ([int]($b.Y + $b.Height / 2)) 24 }
    $selAll = { [void](Pass-Rect); $b = (Detail-Text 'Creada el').Current.BoundingRectangle; [QA.Native]::ClickAt([int]($b.X + 330), [int]($b.Y + 70)); Start-Sleep -Milliseconds 150; Press-Ctrl 'a' }
    $hits = 0
    foreach ($g in @(@('doble clic sobre el valor', $SelDouble), @('triple clic sobre el valor', $SelTriple), @('arrastre de punta a punta del valor', $SelDrag), @('arrastre que empieza en el usuario y cruza el valor', $selAcross), @('Ctrl+A en la pagina', $selAll))) {
        try {
            $r = Try-Copy $st $g[0] $g[1] $Rf.Pass ''
            if ($r.Has) { $hits++ }
            if (-not ((Get-Names) -contains $Rf.Pass)) { Log "   AVISO: tras el gesto el valor ya no estaba a la vista ($($g[0]))" }
        } catch { Row $st "$($g[0]) + Ctrl+C" "NO PROBADO: $($_.Exception.Message)" }
        $b = (Detail-Text 'Creada el').Current.BoundingRectangle; [QA.Native]::ClickAt([int]($b.X + 330), [int]($b.Y + 70)); Start-Sleep -Milliseconds 200
    }
    Check '(i) Ninguno de los cinco gestos + Ctrl+C deja el valor (ni un trozo de 8 caracteres) en el portapapeles' ($hits -eq 0 -and $ctl) "$hits de 5 lo dejan; control positivo = $ctl"
    # Segundo control: con el valor OCULTO, Ctrl+A + Ctrl+C copia el texto de la pagina (sin la contrasena).
    $hide = Find-El -Name 'Ocultar contraseña' -Type Button -Timeout 0.5 -Quiet; if ($hide) { Click-El $hide; Start-Sleep -Milliseconds 300 }
    Set-ClipExternal 'centinela-px-oculto'; Start-Sleep -Milliseconds 300
    $b = (Detail-Text 'Creada el').Current.BoundingRectangle; [QA.Native]::ClickAt([int]($b.X + 330), [int]($b.Y + 70)); Start-Sleep -Milliseconds 150
    Press-Ctrl 'a'; Start-Sleep -Milliseconds 200; Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $c2 = "$(Get-Clip)"
    Row $st 'CONTROL: Ctrl+A + Ctrl+C con el valor oculto' "copia $($c2.Length) caracteres; lleva el titulo = $($c2.Contains($Rf.Title)); lleva la contrasena = $($c2.Contains($Rf.Pass))"
    [QA.Native]::ClickAt([int]($b.X + 330), [int]($b.Y + 70)); Start-Sleep -Milliseconds 200
    # Aparte: la contrasena mostrada DENTRO del campo del dialogo de edicion (es un campo de texto editable).
    Click-Name -Name 'Editar'; [void](Find-El -Name 'Editar entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
    $pw = Find-El -Id 'entry-dialog-password'
    Set-ClipExternal 'centinela-px-campo'; Start-Sleep -Milliseconds 300
    Click-El $pw; Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $hidden = "$(Get-Clip)"
    Click-Name -Name 'Mostrar contraseña'; Start-Sleep -Milliseconds 400
    $pw = Find-El -Id 'entry-dialog-password'
    $isText = -not $pw.Current.IsPassword
    Click-El $pw; Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 700
    $shownClip = "$(Get-Clip)"; $fmt = @(Get-ClipFormats)
    Start-Sleep -Milliseconds 1500
    $hist = History-Has $Rf.Pass
    Row '(i) dialogo de edicion' 'Ctrl+A + Ctrl+C en el campo Contrasena, oculto' $(if ($hidden.Contains($Rf.Pass)) { 'SENAL: copia la contrasena' } elseif ($hidden -ceq 'centinela-px-campo') { 'sigue el centinela: no se copio nada' } else { "otro texto de $($hidden.Length) caracteres" })
    Row '(i) dialogo de edicion' "Ctrl+A + Ctrl+C en el campo Contrasena, mostrado (campo de texto = $isText)" $(if ($shownClip.Contains($Rf.Pass)) { "OBSERVACION: el portapapeles lleva la contrasena (copia del navegador: la app no la borra a los 20 s); formatos de exclusion: $((@($ExclFormats | Where-Object { $fmt -contains $_ })) -join ', '); en el historial: $hist" } elseif ($shownClip -ceq 'centinela-px-campo') { 'sigue el centinela: no se copio nada' } else { "otro texto de $($shownClip.Length) caracteres, sin la contrasena" })
    if ($shownClip.Contains($Rf.Pass)) { Start-Sleep -Seconds 22; Row '(i) dialogo de edicion' 'esa copia, 24 s despues' $(if ("$(Get-Clip)".Contains($Rf.Pass)) { 'OBSERVACION: sigue en el portapapeles' } else { 'ya no esta' }) }
    [System.Windows.Forms.Clipboard]::Clear()
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $d = Find-El -Name 'Descartar' -Type Button -Timeout 0.5 -Quiet; if ($d) { Click-El $d; Start-Sleep -Milliseconds 400 }
    [void](Purge-Mine)
} }

if (-not $only -or $only -eq '71') { Step 'PX71 (ii) Controles positivos del detector' -Continue {
    Open-Canary
    $i = Get-WinInfo
    $r = RC-Plain 'control' 'la barra de titulo de la ventana (menu del sistema)' ($i.X + 220) ($i.Y + 16)
    Check 'CONTROL POSITIVO: el clic derecho en la barra de titulo abre un menu y el detector lo ve' (-not $r.Nothing) "$($r.New -join ', ') $($r.Items -join ' | ')"
    Press 'ESC'; Start-Sleep -Milliseconds 400
    [void](Focus-App); Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Check 'CONTROL POSITIVO: las teclas llegan a la pagina (Ctrl+K lleva el foco a la busqueda)' ((Focused-Desc) -like '*vault-search*') (Focused-Desc)
} }

if (-not $only -or $only -eq '72') { Step 'PX72 (ii) Boveda abierta: clic derecho y Ctrl+P' -Continue {
    $st = '(ii) boveda'
    Open-Canary
    $shown = Show-Pass
    $v = Find-El -Name $Rf.Pass -Type Text -Timeout 1 -Quiet
    if ($v) { $p = Point-Of $v; [void](RC-Plain $st "la contrasena mostrada" $p[0] $p[1]) } else { Row $st 'clic derecho sobre la contrasena mostrada' 'NO PROBADO: el valor no estaba en el arbol' }
    $p = Point-Of (Detail-Text "$($Rf.Notes)*") 0.2; [void](RC-Plain $st 'el texto de las notas' $p[0] $p[1])
    $p = Point-Of (Entry-Row 'Netflix' 2); [void](RC-Plain $st 'una fila de la lista' $p[0] $p[1])
    $p = Point-Of (Find-El -Id 'vault-search'); [void](RC-Plain $st 'el campo de busqueda' $p[0] $p[1])
    $i = Get-WinInfo; [void](RC-Plain $st 'el fondo del riel' ($i.X + [int](120 * $i.Scale)) ($i.Y + $i.OuterH - [int](120 * $i.Scale)))
    $g = RC-Plain $st 'un grupo del riel (Bancos)' (Point-Of (Group-BtnPx 'Bancos'))[0] (Point-Of (Group-BtnPx 'Bancos'))[1]
    Log "   clic derecho sobre un grupo: $(if ($g.Items.Count) { "menu PROPIO de la app = $(($g.Items -join ' | ') -like '*Renombrar*')" } else { 'no abre el menu del grupo' })"
    Press 'ESC'; Start-Sleep -Milliseconds 300
    Open-Canary
    $shown = Show-Pass
    [void](Probe-Surface $st 'Ctrl+P' '' "contrasena mostrada=$shown")
    $shown = Show-Pass
    [void](Probe-Surface $st 'Ctrl+Shift+P' '' "contrasena mostrada=$shown")
    [void](Probe-Surface $st 'Ctrl+S' '' "contrasena mostrada=$shown")
    Probe-Many $st @('Shift+F10', 'Tecla Menu')
    Fill (Find-El -Id 'vault-search') 'zz-marca'; Start-Sleep -Milliseconds 600
    [void](Probe-ReloadKey $st 'F5' $SearchMarker 'foco en la busqueda')
    Click-El (Find-El -Id 'vault-search'); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 400
} }

if (-not $only -or $only -eq '73') { Step 'PX73 (ii) Dialogo de entrada: clic derecho y Ctrl+P' -Continue {
    $st = '(ii) dialogo de entrada'
    Open-Canary
    Click-Name -Name 'Editar'; [void](Find-El -Name 'Editar entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
    Click-Name -Name 'Mostrar contraseña'; Start-Sleep -Milliseconds 300
    $f0 = Focused-Desc; Press 'TAB'; Start-Sleep -Milliseconds 200
    Row $st 'CONTROL: Tab mueve el foco dentro del dialogo' "$f0 -> $(Focused-Desc)"
    $p = Point-Of (Find-El -Id 'entry-dialog-title'); [void](RC-Plain $st 'el campo Titulo' $p[0] $p[1])
    $p = Point-Of (Find-El -Id 'entry-dialog-password') 0.3; [void](RC-Plain $st 'el campo Contrasena (mostrada)' $p[0] $p[1])
    $p = Point-Of (Find-El -Name 'Editar entrada' -Type Text); [void](RC-Plain $st 'el titulo del dialogo' $p[0] $p[1])
    $i = Get-WinInfo; [void](RC-Plain $st 'el fondo oscurecido, fuera del dialogo' ($i.X + [int](120 * $i.Scale)) ($i.Y + [int](400 * $i.Scale)))
    $open = $null -ne (Find-El -Name 'Editar entrada' -Type Window -Timeout 0.5 -Quiet)
    Row $st 'tras los clics derechos (y un clic derecho en el fondo)' "el dialogo sigue abierto = $open"
    [void](Probe-Surface $st 'Ctrl+P' '' 'foco en el dialogo, contrasena mostrada')
    [void](Probe-Surface $st 'Ctrl+S' '' 'foco en el dialogo')
    Probe-Many $st @('Shift+F10', 'Tecla Menu')
    $open = $null -ne (Find-El -Name 'Editar entrada' -Type Window -Timeout 0.5 -Quiet)
    Row $st 'tras las teclas' "el dialogo sigue abierto = $open; titulo intacto = $((Get-Value (Find-El -Id 'entry-dialog-title' -Timeout 1 -Quiet)) -ceq $Rf.Title)"
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $d = Find-El -Name 'Descartar' -Type Button -Timeout 0.5 -Quiet; if ($d) { Click-El $d; Start-Sleep -Milliseconds 400 }
} }

if (-not $only -or $only -eq '74') { Step 'PX74 (ii) Crear boveda: clic derecho, Ctrl+P y control del dialogo nativo' -Continue {
    $st = '(ii) crear boveda'
    Ensure-Cuentas
    Start-Sleep -Milliseconds 2500
    Click-Name -Name 'Bloquear'; [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10); Start-Sleep -Milliseconds 1200
    Click-Name -Name 'Cerrar bóveda'; [void](Find-El -Id 'welcome-title' -Timeout 6); Start-Sleep -Milliseconds 400
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 400
    Fill-Id 'create-name' 'FAROPX-Boveda'
    Fill-Id 'create-password' $RfOther -Secret
    Click-Name -Name 'Mostrar contraseña'; Start-Sleep -Milliseconds 300
    $p = Point-Of (Find-El -Id 'create-name'); [void](RC-Plain $st 'el campo Nombre' $p[0] $p[1])
    $p = Point-Of (Find-El -Id 'create-password') 0.3; [void](RC-Plain $st 'la contrasena maestra (mostrada)' $p[0] $p[1])
    $p = Point-Of (Find-El -Id 'create-title'); [void](RC-Plain $st 'el titulo' $p[0] $p[1])
    $i = Get-WinInfo; [void](RC-Plain $st 'el panel de marca' ($i.X + [int](120 * $i.Scale)) ($i.Y + [int](400 * $i.Scale)))
    [void](Probe-Surface $st 'Ctrl+P' '' 'contrasena maestra mostrada')
    [void](Probe-Surface $st 'Ctrl+S')
    Probe-Many $st @('Shift+F10', 'Tecla Menu')
    Row $st 'tras los gestos' "sigue en crear = $((Screen-Ext) -eq 'crear'); nombre intacto = $((Get-Value (Find-El -Id 'create-name')) -ceq 'FAROPX-Boveda')"
    # CONTROL POSITIVO del detector de dialogos nativos: "Cambiar ubicacion..." abre uno a proposito.
    Click-Name -Name 'Cambiar ubicación…'
    $h = Find-FileDialog 6; Start-Sleep -Milliseconds 600
    $sig = @(Surface-Signal)
    Check 'CONTROL POSITIVO: un dialogo nativo (#32770) abierto a proposito si lo ve el detector' ($h -ne [IntPtr]::Zero -and (($sig -join ';') -like '*#32770*')) ($sig -join '; ')
    if ($h -ne [IntPtr]::Zero) { [void][QA.Native]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300; Press 'ESC'; Start-Sleep -Milliseconds 800 }
    Check 'El dialogo nativo se cerro con Escape' ((Find-FileDialog 0.5) -eq [IntPtr]::Zero)
    [void](Focus-App)
    Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 700
    Log "   tras Cancelar: pantalla = $(Screen-Ext)"
} }
Rows-Summary
Summary
