# Reverificacion final (6b): lo que ofrece el menu contextual de WebView2 en release. "Guardar como" con una
# entrada abierta y su contrasena mostrada (se guarda SOLO dentro de la carpeta de trabajo, se inspecciona y se borra),
# el submenu "Mas herramientas" y Ctrl+Shift+P. Nunca se confirma una impresion.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$script:MiCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuItem)
# Clic derecho en (x, y) y devuelve los elementos del menu que se abre (sin cerrarlo).
function Open-ContextMenu([int]$x, [int]$y) {
    [void](Focus-App)
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 120
    [QA.Native]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [QA.Native]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 1200
    $el = $AE::FromPoint((New-Object System.Windows.Point(($x + 14), ($y + 14))))
    # El menu emergente se alcanza subiendo desde lo que hay bajo el puntero (vista cruda); desde la ventana de la app
    # hacia abajo no aparece. Se toma el primer ancestro que tenga elementos de menu.
    $cur = $el; $top = $null
    $walker = [System.Windows.Automation.TreeWalker]::RawViewWalker
    for ($k = 0; $k -lt 8 -and $null -ne $cur; $k++) {
        if ((Type-Of $cur) -in @('Menu', 'MenuBar', 'Window', 'Pane') -and @($cur.FindAll($TreeScope::Descendants, $MiCond)).Count -gt 0) { $top = $cur; break }
        $cur = $walker.GetParent($cur)
    }
    $script:MenuTop = $top
    if ($null -eq $top) { Log "   menu: no se encontro (bajo el puntero: [$(Type-Of $el)] `"$($el.Current.Name)`" clase=$($el.Current.ClassName))"; return @() }
    Log "   menu: raiz [$(Type-Of $top)] clase=$($top.Current.ClassName) proceso=$((Get-Process -Id $top.Current.ProcessId -ErrorAction SilentlyContinue).Name)"
    return @($top.FindAll($TreeScope::Descendants, $MiCond))
}
function Click-MenuItem($items, [string]$name) {
    $it = $items | Where-Object { $_.Current.Name -eq $name } | Select-Object -First 1
    if ($null -eq $it) { return $false }
    $r = $it.Current.BoundingRectangle
    [QA.Native]::ClickAt([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2))
    return $true
}
function Notes-Point {
    $note = Find-El -Name $Rf.Notes -Type Text -Timeout 3
    $r = $note.Current.BoundingRectangle
    return @([int]($r.X + 40), [int]($r.Y + $r.Height / 2))
}
Step 'RF37 "Guardar como" del menu contextual con una entrada abierta' -Continue {
    Ensure-VaultOpen
    [void](Set-WinClient 1120 720)
    Open-Entry $RootName $Rf.Title
    Click-Name -Name 'Mostrar contraseña'
    Start-Sleep -Milliseconds 500
    Check 'Preparacion: la contrasena de la entrada esta mostrada en pantalla' ((Get-Names) -contains $Rf.Pass)
    $pt = Notes-Point
    $w0 = Top-Wins
    $items = Open-ContextMenu $pt[0] $pt[1]
    Log "   elementos: $(($items | ForEach-Object { $_.Current.Name }) -join ' | ')"
    Shot-Area 'RF31-menu-contextual'
    $clicked = Click-MenuItem $items 'Guardar como'
    Start-Sleep -Seconds 3
    $w1 = Top-Wins
    $newKeys = @($w1.Keys | Where-Object { -not $w0.ContainsKey($_) })
    Log "   'Guardar como' pulsado = $clicked; ventanas nuevas=$($newKeys.Count) $(($newKeys | ForEach-Object { $w1[$_] }) -join ' ; ')"
    $dir = Join-Path $Work 'guardado-menu'
    New-Item -ItemType Directory -Force $dir | Out-Null
    $saved = $false
    # El dialogo "Guardar como" no sale entre las ventanas de primer nivel de UIA: se busca por clase entre las
    # ventanas de los procesos de la app (el propio y los de WebView2).
    $h = [IntPtr]::Zero; $ownerName = ''
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($h -eq [IntPtr]::Zero -and $sw.Elapsed.TotalSeconds -lt 6) {
        foreach ($procId in Get-AppPids) {
            foreach ($w in [QA.Native]::WindowsOfPid([uint32]$procId)) {
                if ([QA.Native]::ClassOf($w) -eq '#32770') { $h = $w; $ownerName = (Get-Process -Id $procId -ErrorAction SilentlyContinue).Name }
            }
        }
        if ($h -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 200 }
    }
    if ($h -ne [IntPtr]::Zero) {
        $dlg = $AE::FromHandle($h)
        Log "   dialogo nativo: `"$($dlg.Current.Name)`" del proceso $ownerName"
        $edit = $null
        foreach ($id in @('1001', '1148')) { $edit = Find-El -Id $id -Type 'Edit' -Root $dlg -Timeout 2 -Quiet; if ($null -ne $edit) { break } }
        [void][QA.Native]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300
        $target = Join-Path $dir 'pagina-guardada.html'
        if ($null -ne $edit) {
            $vp = $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
            Log "   nombre propuesto por el dialogo: $(Redact $vp.Current.Value)"
            $vp.SetValue($target)
        } else {
            [QA.Native]::Chord(0x12, [byte][char]'N'); Start-Sleep -Milliseconds 150
            Press-Ctrl 'a'; [QA.Native]::TypeUnicode($target, 4)
        }
        Start-Sleep -Milliseconds 300
        Shot 'RF32-dialogo-guardar-como' $h
        Press 'ENTER'; Start-Sleep -Seconds 6
        $saved = $true
    }
    $files = @(Get-ChildItem $dir -Recurse -File -Force)
    Log "   en la carpeta de trabajo quedaron $($files.Count) archivos, $(($files | Measure-Object Length -Sum).Sum) bytes: $(($files | Select-Object -First 10 | ForEach-Object { $_.Name }) -join ', ')"
    if ($files.Count) {
        $lines = Run-Inspector (Rf-Secrets) @("guardado=$dir")
        $kinds = @($lines | Where-Object { $_ -like 'APARICION|*' } | ForEach-Object { ($_ -split '\|')[3] } | Select-Object -Unique)
        Log "GUARDAR COMO: apariciones = $(Hits-Of $lines 'guardado'); que aparece: $($kinds -join ' ; ')"
        Check '"Guardar como" NO deja en disco, en claro, el contenido de la entrada abierta' ((Hits-Of $lines 'guardado') -eq 0) "apariciones: $(Hits-Of $lines 'guardado')"
    } else {
        Check '"Guardar como" NO deja en disco, en claro, el contenido de la entrada abierta' $true "no se guardo ningun archivo (dialogo abierto = $saved)"
    }
    Shot-Area 'RF33-tras-guardar-como'
    Press 'ESC'; Start-Sleep -Milliseconds 500
    foreach ($k in $newKeys) { if ((Top-Wins).ContainsKey($k)) { [void][QA.Native]::PostMessage([IntPtr][long]$k, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) } }
    if (Test-Path $dir) { Remove-Item -LiteralPath $dir -Recurse -Force }
    Log "   pantalla tras guardar: $(Screen-Now)"
}
Step 'RF38 Submenu "Mas herramientas"' -Continue {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $pt = Notes-Point
    $items = Open-ContextMenu $pt[0] $pt[1]
    $n0 = @($items | ForEach-Object { $_.Current.Name })
    $clicked = Click-MenuItem $items 'Más herramientas'
    Start-Sleep -Milliseconds 1200
    $after = @(); if ($null -ne $MenuTop) { $after = @($MenuTop.FindAll($TreeScope::Descendants, $MiCond) | ForEach-Object { $_.Current.Name }) }
    $focus = Focused-Desc
    $sub = @($after | Where-Object { $n0 -notcontains $_ })
    try {
        $f = $AE::FocusedElement
        $parent = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($f)
        if ($null -ne $parent) { foreach ($m in $parent.FindAll($TreeScope::Children, $MiCond)) { if ($sub -notcontains $m.Current.Name -and $n0 -notcontains $m.Current.Name) { $sub += $m.Current.Name } } }
    } catch { }
    try {
        $r = ($items | Where-Object { $_.Current.Name -eq 'Más herramientas' } | Select-Object -First 1).Current.BoundingRectangle
        $el = $AE::FromPoint((New-Object System.Windows.Point(($r.X + $r.Width + 40), ($r.Y + $r.Height / 2))))
        Log "   a la derecha de 'Más herramientas': [$(Type-Of $el)] `"$($el.Current.Name)`""
        $p2 = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($el)
        if ($null -ne $p2) { foreach ($m in $p2.FindAll($TreeScope::Children, $MiCond)) { if ($sub -notcontains $m.Current.Name -and $n0 -notcontains $m.Current.Name) { $sub += $m.Current.Name } } }
    } catch { }
    Log "MAS HERRAMIENTAS: pulsado = $clicked; foco: $focus; elementos del submenu ($($sub.Count)): $($sub -join ' | ')"
    Shot-Area 'RF34-mas-herramientas'
    Press 'ESC'; Start-Sleep -Milliseconds 300; Press 'ESC'; Start-Sleep -Milliseconds 400
}
Step 'RF39 Ctrl+Shift+P y cierre de la vista de impresion con Escape' -Continue {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $p = Probe-Key 'Ctrl+Shift+P' { Keys @(0x11, 0x10, 0x50) } 'RF35-ctrl-shift-p'
    Press 'ESC'; Start-Sleep -Milliseconds 800
    foreach ($k in $p.NewHandles) { if ((Top-Wins).ContainsKey($k)) { Log "   sigue abierta: se cierra"; [void][QA.Native]::PostMessage([IntPtr][long]$k, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Start-Sleep -Milliseconds 600 } }
    Log "   pantalla tras Ctrl+Shift+P y Escape: $(Screen-Now)"
    [void](Focus-App)
    Open-Entry $RootName $Rf.Title
    $p = Probe-Key 'Ctrl+P' { Keys @(0x11, 0x50) }
    $preview = @(Get-Names | Where-Object { $_ -like '*TESTIGO-RF*' }).Count
    Log "   con la vista de impresion abierta: nombres con canarios en el arbol = $preview"
    Press 'ESC'; Start-Sleep -Milliseconds 900
    Check 'Escape cierra la vista de impresion y la boveda sigue abierta' ((Screen-Now) -eq 'boveda abierta')
}
Summary
