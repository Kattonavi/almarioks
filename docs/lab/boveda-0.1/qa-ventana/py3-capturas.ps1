# Ronda 2 (3): capturas y medidas de la boveda (lista en tabla) y de la ventana de la entrada en la app instalada.
# Van a la carpeta de estado; a .impeccable\review\ronda2\app se copian despues de mirarlas. Mide por UIA (px logicos)
# y, lo que UIA no da (circulo, tinta del rotulo MODIFICADA, fondo de los botones), leyendo pixeles de la pantalla.
# Incluye (e): sin barra horizontal a 1000 x 640. RF_VARIANT=py-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
function Grab-L([double]$x, [double]$y, [double]$w, [double]$h) {
    $i = Get-WinInfo; $o = Client-Origin; $s = $i.Scale
    $bmp = New-Object System.Drawing.Bitmap([int]($w * $s), [int]($h * $s)); $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($o.X + [int]($x * $s), $o.Y + [int]($y * $s), 0, 0, $bmp.Size); $g.Dispose(); return $bmp
}
# Extension (en px logicos, relativa al origen del recorte) de los pixeles que cumplen $test, en X y en Y.
function Span-Of($bmp, [scriptblock]$test) {
    $x0 = -1; $x1 = -1; $y0 = -1; $y1 = -1; $s = (Get-WinInfo).Scale
    for ($x = 0; $x -lt $bmp.Width; $x++) { for ($y = 0; $y -lt $bmp.Height; $y++) {
            if (& $test ($bmp.GetPixel($x, $y))) { if ($x0 -lt 0) { $x0 = $x }; $x1 = $x; if ($y0 -lt 0 -or $y -lt $y0) { $y0 = $y }; if ($y -gt $y1) { $y1 = $y } } } }
    [pscustomobject]@{ X0 = [math]::Round($x0 / $s, 1); X1 = [math]::Round(($x1 + 1) / $s, 1); W = [math]::Round(($x1 + 1 - $x0) / $s, 1); H = [math]::Round(($y1 + 1 - $y0) / $s, 1); Found = ($x0 -ge 0) }
}
function Measure-List([string]$tag) {
    Pointer-Away; Start-Sleep -Milliseconds 400
    $panel = Find-El -Like 'Entradas de *' -Type Group -Timeout 3
    $table = Find-El -Type Table -Timeout 3
    $rows = @(Kids $table)
    Log "   [$tag] RIEL: la lista empieza en x=$((LRect $panel).X); tabla $(LRect-Text $table); filas (alto): $(($rows | ForEach-Object { (LRect $_).H }) -join ', ')"
    $cols = foreach ($c in @('TÍTULO', 'USUARIO', 'CONTRASEÑA', 'SITIO', 'MODIFICADA')) { $r = LRect (Header-Of $c); "$c x=$($r.X) w=$($r.W) fin=$($r.R)" }
    Log "   [$tag] ROTULOS: $($cols -join ' | ')"
    $cells = foreach ($c in @('TÍTULO', 'USUARIO', 'CONTRASEÑA', 'SITIO', 'MODIFICADA')) { $k = Cell-Of 'GitHub' $c; $r = LRect $k; "$c `"$($k.Current.Name)`" x=$($r.X) w=$($r.W) h=$($r.H)" }
    Log "   [$tag] FILA GitHub: $($cells -join ' | ')"
    $copy = Find-El -Name 'Copiar contraseña de GitHub' -Type Button -Timeout 1 -Quiet; $open = Find-El -Name 'Abrir GitHub' -Type Button -Timeout 1 -Quiet
    if ($null -ne $copy) {
        $act = LRect (Raw-Parent $copy)
        Log "   [$tag] FILA ELEGIDA: celda de acciones x=$($act.X) w=$($act.W) fin=$($act.R); Copiar $(LRect-Text $copy) brillo=$(Luma-Of $copy); Abrir $(LRect-Text $open) brillo=$(Luma-Of $open)"
    }
    $h = LRect (Header-Of 'MODIFICADA')
    $bmp = Grab-L 900 ($h.Y - 2) (([double](Get-WinInfo).LogicalW) - 900) ($h.H + 4)
    $bg = $bmp.GetPixel(1, 1)
    $ink = Span-Of $bmp { param($c) ([math]::Abs($c.R - $bg.R) + [math]::Abs($c.G - $bg.G) + [math]::Abs($c.B - $bg.B)) -gt 70 }
    $bmp.Dispose()
    Log "   [$tag] TINTA del rotulo MODIFICADA (pixeles): de x=$(900 + $ink.X0) a x=$(900 + $ink.X1) (ancho $($ink.W)); su columna va de $($h.X) a $($h.R); la columna de acciones empieza en $($h.R + 8)"
    $t = LRect (Entry-Row 'GitHub')
    $bmp = Grab-L $t.X $t.Y 70 $t.H
    $circle = Span-Of $bmp { param($c) $c.R -gt 180 -and $c.B -lt 110 -and ($c.R - $c.B) -gt 110 }
    $bmp.Dispose()
    Log "   [$tag] CIRCULO de GitHub (pixeles naranja): ancho $($circle.W) alto $($circle.H), empieza en x=$($t.X + $circle.X0)"
    $foot = Find-El -Name 'Doble clic' -Type Text -Timeout 1 -Quiet
    Log "   [$tag] PIE: `"Doble clic`" $(if ($null -ne $foot) { LRect-Text $foot } else { '(no esta)' })"
    foreach ($s in (Scroll-Report)) { Log "   [$tag] DESPLAZABLE: $($s.What) horizontal=$($s.H) vertical=$($s.V) (vista h $($s.HView) % v $($s.VView) %)" }
    $over = @(Get-All | Where-Object { $r = LRect $_; $null -ne $r -and $r.W -gt 4 -and $r.R -gt ((Get-WinInfo).LogicalW + 0.5) -and (Type-Of $_) -ne 'Pane' -and $_.Current.Name -notlike 'Atajo:*' } | ForEach-Object { "[$(Type-Of $_)] `"$($_.Current.Name)`" fin=$((LRect $_).R)" })
    Log "   [$tag] ELEMENTOS QUE SE SALEN POR LA DERECHA: $(if ($over.Count) { $over -join ' | ' } else { 'ninguno' })"
}
Step 'PY30 Bloquear y desbloquear (riel limpio)' {
    Ensure-Cuentas; Clear-Search; Start-Sleep -Milliseconds 2500
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10); Start-Sleep -Milliseconds 1200
    Ensure-Cuentas
}
Step 'PY31 Boveda a 1120 x 720' -Continue {
    Check 'Ventana a 1120 x 720 logicos y dentro del area util' (Size-Window 1120 720)
    Pick-Group 'Trabajo'; Pick-Entry 'GitHub'; Start-Sleep -Milliseconds 800
    Shot-Client 'app-boveda-1120x720'
    [void](Dump-Rects -All -file 'rects-boveda-1120x720.txt')
    Measure-List '1120'
}
Step 'PY32 Ventana de la entrada con doble clic real en el titulo' -Continue {
    Pick-Entry 'Figma'; Start-Sleep -Milliseconds 500      # GitHub deja de ser la fila elegida: el primer clic del doble clic la repinta
    $b = Entry-Row 'GitHub'; $r = $b.Current.BoundingRectangle; $s = (Get-WinInfo).Scale
    $x = [int]($r.X + 70 * $s); $y = [int]($r.Y + $r.Height / 2)
    Dbl-At $x $y
    $win = Entry-Window 'GitHub' 2.5
    Check 'Doble clic real en el TITULO de una fila NO elegida abre la ventana de la entrada' ($null -ne $win) "punto: x=$([math]::Round(($x - (Client-Origin).X) / $s)) logico dentro de la celda del titulo; separacion entre clics 110 ms"
    if ($null -eq $win) { Dbl-At $x $y; $win = Entry-Window 'GitHub' 2.5; Log "   segundo intento (fila ya elegida): abre = $($null -ne $win)" }
    Pointer-Away; Start-Sleep -Milliseconds 600
    Shot-Client 'app-entrada-1120x720'
    [void](Dump-Rects -All -file 'rects-entrada-1120x720.txt')
    $win = Entry-Window 'GitHub' 2
    Log "   VENTANA DE LA ENTRADA: $(LRect-Text $win); foco: $(Focused-Desc)"
    foreach ($el in @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition))) {
        $t = Type-Of $el; $n = $el.Current.Name
        if ($t -eq 'Button') { Log "   BOTON `"$n`" $(LRect-Text $el) brillo=$(Luma-Of $el)" }
        elseif ($t -eq 'ListItem') { Log "   FILA `"$n`" (etiqueta) $(LRect-Text $el)" }
        elseif ($t -eq 'Text' -and $n) { Log "   TEXTO `"$n`" $(LRect-Text $el)" }
        elseif ($t -eq 'List') { Log "   TARJETA $(LRect-Text $el)" }
    }
    $w = LRect $win
    $bmp = Grab-L $w.X $w.Y 110 90
    $circle = Span-Of $bmp { param($c) $c.R -gt 180 -and $c.B -lt 110 -and ($c.R - $c.B) -gt 110 }
    $bmp.Dispose()
    Log "   CIRCULO de la cabecera (pixeles naranja): ancho $($circle.W) alto $($circle.H)"
}
Step 'PY33 Menu de la entrada y Esc' -Continue {
    $btn = Find-El -Name 'Más acciones de la entrada' -Type Button
    Click-El $btn; Start-Sleep -Milliseconds 600
    $items = @(Get-All | Where-Object { (Type-Of $_) -eq 'MenuItem' } | ForEach-Object { "$($_.Current.Name) [$(LRect-Text $_)]" })
    Log "   MENU de la entrada: $($items -join ' | ')"
    Shot-Client 'app-entrada-menu' -KeepPointer
    Press 'ESC'; Start-Sleep -Milliseconds 500
    Check 'Esc cierra el menu y la ventana de la entrada sigue abierta' ($null -ne (Entry-Window 'GitHub' 1)) "foco: $(Focused-Desc)"
    Press 'ESC'; Start-Sleep -Milliseconds 600
    $chosen = Has-El -Name 'Abrir GitHub' -Type Button -Timeout 1
    Check 'Esc en la ventana: vuelve a la lista con la fila elegida y el foco en ella' ($null -eq (Entry-Window 'GitHub' 0.4) -and $chosen -and (Focused-Desc) -like '*"GitHub"*') "fila GitHub elegida = $chosen; foco: $(Focused-Desc)"
}
Step 'PY34 Nueva entrada con las opciones del generador abiertas' -Continue {
    Click-Name -Name 'Nueva entrada'
    [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 500
    Click-Name -Name 'Opciones del generador'; Start-Sleep -Milliseconds 800
    Pointer-Away
    Shot-Client 'app-nueva-entrada-opciones'
    $dlg = Find-El -Name 'Nueva entrada' -Type Window
    Log "   DIALOGO: $(LRect-Text $dlg); controles: $((@($dlg.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) | Where-Object { $_.Current.Name -and (Type-Of $_) -ne 'Text' } | ForEach-Object { "[$(Type-Of $_)] $($_.Current.Name)" }) -join ' | ')"
    Ensure-Cuentas
}
Step 'PY35 (e) Ventana minima 1000 x 640: sin barra horizontal' -Continue {
    Check 'Ventana a 1000 x 640 logicos' (Size-Window 1000 640)
    Pick-Group 'Trabajo'; Pick-Entry 'GitHub'; Start-Sleep -Milliseconds 700
    Shot-Client 'app-boveda-1000x640'
    [void](Dump-Rects -All -file 'rects-boveda-1000x640.txt')
    Measure-List '1000 Trabajo'
    Pick-All; Start-Sleep -Milliseconds 600
    foreach ($s in (Scroll-Report)) { Log "   [1000 Todas, control] DESPLAZABLE: $($s.What) horizontal=$($s.H) vertical=$($s.V) (vista h $($s.HView) % v $($s.VView) %)" }
    Fill-Id 'vault-search' 'a'; Start-Sleep -Milliseconds 900
    $cols = foreach ($c in @('TÍTULO', 'USUARIO', 'CONTRASEÑA', 'SITIO', 'GRUPO', 'MODIFICADA')) { $r = LRect (Header-Of $c); "$c x=$($r.X) w=$($r.W) fin=$($r.R)" }
    Log "   [1000 busqueda] ROTULOS: $($cols -join ' | ')"
    foreach ($s in (Scroll-Report)) { Log "   [1000 busqueda] DESPLAZABLE: $($s.What) horizontal=$($s.H) vertical=$($s.V) (vista h $($s.HView) % v $($s.VView) %)" }
    Pointer-Away; Shot-Client 'extra-busqueda-1000x640'
    Clear-Search
    $i = Set-WinClient 900 560
    Log "   Pedir 900 x 560 deja la ventana en $($i.LogicalW) x $($i.LogicalH) logicos"
    Check 'La ventana no baja de 1000 x 640' ($i.LogicalW -ge 1000 -and $i.LogicalH -ge 640) "$($i.LogicalW) x $($i.LogicalH)"
}
Step 'PY36 Volver a 1120 x 720' -Continue { Ensure-Cuentas; Clear-Search; Check 'Ventana a 1120 x 720' (Size-Window 1120 720); Pick-Group 'Trabajo'; Pick-Entry 'GitHub' }
Summary
