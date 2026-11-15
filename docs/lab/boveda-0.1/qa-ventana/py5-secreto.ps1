# Ronda 2 (5): (d) la contrasena no aparece en la lista por ningun camino del arbol de accesibilidad ni delata su
# longitud; en la ventana de la entrada, oculta tampoco y mostrada si (control). (f) entrada sin contrasena.
# (g) ventana por debajo del minimo. Argumento opcional: d, f o g. RF_VARIANT=py-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { '' }
# Ancho (px logicos) de la "tinta" dentro del rectangulo de un elemento: pixeles que se apartan del color de su esquina.
function Ink-Of($el, [int]$thr = 90) {
    Pointer-Away; Start-Sleep -Milliseconds 250
    $r = $el.Current.BoundingRectangle; $s = (Get-WinInfo).Scale
    $bw = [int]$r.Width - 4; $bh = [int]$r.Height - 8
    $bmp = New-Object System.Drawing.Bitmap($bw, $bh); $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen([int]$r.X + 2, [int]$r.Y + 4, 0, 0, $bmp.Size); $g.Dispose()
    $bg = $bmp.GetPixel(1, 1); $x0 = -1; $x1 = -1; $n = 0
    for ($x = 0; $x -lt $bmp.Width; $x++) { for ($y = 0; $y -lt $bmp.Height; $y++) { $c = $bmp.GetPixel($x, $y)
            if (([math]::Abs($c.R - $bg.R) + [math]::Abs($c.G - $bg.G) + [math]::Abs($c.B - $bg.B)) -gt $thr) { if ($x0 -lt 0) { $x0 = $x }; $x1 = $x; $n++ } } }
    $bmp.Dispose()
    if ($x0 -lt 0) { return 'sin tinta' }
    return "tinta de $([math]::Round(($x1 + 1 - $x0) / $s, 1)) px ($n pixeles)"
}
function Scan([string]$what, [bool]$expectHits, [hashtable]$needles) {
    $r = Tree-Find $needles
    $ok = if ($expectHits) { $r.Hits.Count -gt 0 } else { $r.Hits.Count -eq 0 }
    Check $what $ok "$($r.Count) textos recorridos (nombres, valores, HelpText, ItemStatus, ids y texto del documento); hallazgos: $(if ($r.Hits.Count) { ($r.Hits | Select-Object -First 6) -join ' | ' } else { 'ninguno' })"
}
$control = @{ 'usuario de muestra' = 'ana.casa@example.com'; 'sitio de muestra' = 'slack.example'; 'titulo de muestra' = 'Banco de la esquina' }
Step 'PY50 Preparacion' { Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search }

if (-not $only -or $only -eq 'd') {
Step 'PY51 (d) La lista: arbol completo y mascara' -Continue {
    Pick-All; Start-Sleep -Milliseconds 600
    Scan '(d) CONTROL: el recorrido SI encuentra usuario, sitio y titulo en "Todas las entradas"' $true $control
    Scan '(d) "Todas las entradas" (13 filas): ninguna contrasena en el arbol' $false (Pass-Needles)
    Pick-Entry 'Slack'; Start-Sleep -Milliseconds 400
    $c = Cell-Of 'Slack' 'CONTRASEÑA'; $m = Mid $c
    [void](Focus-App); [void][QA.Native]::SetCursorPos($m[0], $m[1]); Start-Sleep -Milliseconds 80; [void][QA.Native]::SetCursorPos($m[0] + 3, $m[1] + 1); Start-Sleep -Milliseconds 1600
    Scan '(d) Fila elegida y cursor quieto sobre la celda de la contrasena (ayuda a la vista): ninguna contrasena' $false (Pass-Needles)
    Pick-Group 'Trabajo'; Pick-Entry 'Correo de la oficina'; Start-Sleep -Milliseconds 400      # las tres filas medidas, a la vista y sin elegir
    $info = foreach ($t in @('Jira', 'GitHub', 'Slack')) { $c = Cell-Of $t 'CONTRASEÑA'; "$t ($($PyPass[$t].Length) caracteres): nombre `"$($c.Current.Name)`", ayuda `"$($c.Current.HelpText)`", estado `"$($c.Current.ItemStatus)`", ancho $((LRect $c).W), $(Ink-Of $c)" }
    foreach ($l in $info) { Log "   MASCARA en la lista: $l" }
    $norm = @($info | ForEach-Object { $_ -replace '^[^:]+: ', '' } | Select-Object -Unique)
    Check '(d) La mascara de la lista es identica (nombre, ayuda y tinta) para contrasenas de 8, 18 y 40 caracteres' ($norm.Count -eq 1) "variantes distintas: $($norm.Count)"
    $row = foreach ($col in @('TÍTULO', 'USUARIO', 'SITIO', 'MODIFICADA')) { $c = Cell-Of 'Slack' $col; "$col ayuda `"$($c.Current.HelpText)`"" }
    Log "   AYUDAS de la fila: $($row -join ' | '); boton del titulo: `"$((Entry-Row 'Slack').Current.HelpText)`""
    Fill-Id 'vault-search' 'example'; Start-Sleep -Milliseconds 900
    Scan '(d) Resultados de busqueda: ninguna contrasena en el arbol' $false (Pass-Needles)
    Fill-Id 'vault-search' 'FAROPY'; Start-Sleep -Milliseconds 900
    $n = @(Get-Names | Where-Object { $_ -match '^\d+ resultados?$|Sin resultados|Nada coincide|No hay' })
    Log "   buscar el prefijo de las contrasenas (`"FAROPY`") da: $($n -join ' | ')"
    Clear-Search
}
Step 'PY52 (d) La ventana de la entrada: oculta, mostrada (control) y cerrada' -Continue {
    Pick-Group 'Trabajo'; Start-Sleep -Milliseconds 300
    $mask = @()
    foreach ($t in @('Jira', 'Slack')) {
        Pick-Entry $t; Click-Name -Name "Abrir $t"
        $win = Entry-Window $t 4; Start-Sleep -Milliseconds 500
        Scan "(d) Ventana de $t con la contrasena OCULTA: ninguna contrasena en el arbol" $false (Pass-Needles)
        $txt = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -eq '(oculto)' })[0]
        $grp = Raw-Parent $txt
        $mask += "nombre `"$($txt.Current.Name)`", ayuda `"$($grp.Current.HelpText)`", $(Ink-Of $grp 200)"
        Log "   MASCARA en la ventana, $t ($($PyPass[$t].Length) caracteres): $($mask[-1])"
        if ($t -eq 'Slack') {
            Click-Name -Name 'Mostrar contraseña'; Start-Sleep -Milliseconds 600
            Scan '(d) CONTROL: con la contrasena MOSTRADA el mismo recorrido SI la encuentra' $true @{ 'contrasena de Slack' = $PyPass['Slack'] }
            Log "   botones con la contrasena a la vista: $((@((Entry-Window $t 1).FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) | Where-Object { (Type-Of $_) -eq 'Button' } | ForEach-Object { $_.Current.Name }) -join ' | ')"
            Pointer-Away; Shot-Client 'extra-entrada-mostrada'
        }
        Press 'ESC'; Start-Sleep -Milliseconds 600
        if ($t -eq 'Slack') {
            Scan '(d) Al cerrar con Esc lo mostrado ya no esta en el arbol' $false (Pass-Needles)
            Click-Name -Name "Abrir $t"; [void](Entry-Window $t 4); Start-Sleep -Milliseconds 500
            Scan '(d) Al volver a abrir la ventana la contrasena vuelve OCULTA' $false (Pass-Needles)
            Press 'ESC'; Start-Sleep -Milliseconds 500
        }
    }
    Check '(d) La mascara de la ventana es identica para contrasenas de 8 y 40 caracteres' ($mask[0] -eq $mask[1]) "8: $($mask[0]) / 40: $($mask[1])"
    Pick-Entry 'Correo de la oficina'; Start-Sleep -Milliseconds 300
    $c = Cell-Of 'Figma' 'USUARIO'; $m = Mid $c; Set-ClipExternal 'centinela-py-antes'
    Dbl-At $m[0] $m[1] 330; Start-Sleep -Milliseconds 400
    Check 'Doble clic LENTO (330 ms entre clics) en el usuario de una fila no elegida tambien copia' ((Clip-State 'ana@example.com').Exact) "portapapeles = `"$(Get-Clip)`""
}
}

if (-not $only -or $only -eq 'f') {
Step 'PY53 (f) Entrada sin contrasena' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Compras'; Start-Sleep -Milliseconds 300
    $title = 'Tarjeta de puntos'
    if (-not (Has-Entry $title)) {
        $r = New-EntryPy $title 'ana.casa@example.com' 'https://puntos.example' '' ''
        Log "   creada sin tocar el campo de contrasena: cerrado=$($r.Closed) disco=$($r.Disk); foco al guardar: $($r.FocusAfter)"
    } else { Pick-Entry $title }
    $c = Cell-Of $title 'CONTRASEÑA'
    Log "   celda de la contrasena: nombre `"$($c.Current.Name)`", ayuda `"$($c.Current.HelpText)`", $(Ink-Of $c); CONTROL (Tienda en línea): nombre `"$((Cell-Of 'Tienda en línea' 'CONTRASEÑA').Current.Name)`", $(Ink-Of (Cell-Of 'Tienda en línea' 'CONTRASEÑA'))"
    Check '(f) La celda de la contrasena va vacia (sin mascara)' ($c.Current.Name -ne 'Oculta' -and (Ink-Of $c) -eq 'sin tinta') "nombre `"$($c.Current.Name)`""
    $btns = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like "*$title*" } | ForEach-Object { $_.Current.Name })
    Check '(f) La fila elegida NO ofrece "Copiar contraseña"' (-not ($btns -like 'Copiar contrase*') -and ($btns -contains "Abrir $title")) "botones de la fila: $($btns -join ' | ')"
    Set-ClipExternal 'centinela-py-intacto'; Start-Sleep -Milliseconds 300; $seq = Get-ClipSeq
    $m = Mid $c; Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 900
    Check '(f) Doble clic en la celda vacia no copia nada: portapapeles intacto' ("$(Get-Clip)" -ceq 'centinela-py-intacto' -and (Get-ClipSeq) -eq $seq -and $null -eq (Find-El -Type Window -Timeout 0.3 -Quiet)) "numero de secuencia $seq -> $(Get-ClipSeq); nombres con 'copiad': $((Get-Names | Where-Object { $_ -match 'opiad' }) -join ' | ')"
    $c2 = Cell-Of 'Tienda en línea' 'CONTRASEÑA'; $m = Mid $c2; Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 500
    Check '(f) CONTROL: el mismo gesto en la fila vecina CON contrasena si copia' ((Clip-State $PyPass['Tienda en línea']).Exact) "fila elegida ahora ofrece: $((@(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like '*Tienda en l*' } | ForEach-Object { $_.Current.Name })) -join ' | ')"
    [System.Windows.Forms.Clipboard]::Clear()
    Pick-Entry $title; Click-Name -Name "Abrir $title"; $win = Entry-Window $title 4; Start-Sleep -Milliseconds 500
    $all = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -and (Type-Of $_) -in @('Text', 'Button') } | ForEach-Object { "[$(Type-Of $_)] $($_.Current.Name)" })
    Log "   VENTANA de la entrada sin contrasena: $($all -join ' | ')"
    Pointer-Away; Shot-Client 'extra-entrada-sin-contrasena'
    Press 'ESC'; Start-Sleep -Milliseconds 500
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 250; Type-Text 'tarjeta'; Start-Sleep -Milliseconds 800; Press 'ENTER'; Start-Sleep -Milliseconds 500
    Log "   teclado: Ctrl+K, 'tarjeta', Enter deja el foco en: $(Focused-Desc)"
    Pointer-Away; Shot-Client 'extra-lista-sin-contrasena'
    Clear-Search
}
}

if (-not $only -or $only -eq 'g') {
Step 'PY54 (g) Ventana por debajo de 1000 x 640' -Continue {
    Ensure-Cuentas; Pick-Group 'Trabajo'; Pick-Entry 'GitHub'
    [void](Size-Window 1000 640)
    $i = Set-WinClient 700 450
    Log "   SetWindowPos a 700 x 450 logicos deja el interior en $($i.LogicalW) x $($i.LogicalH)"
    [void](Size-Window 1000 640)
    $r = [QA.Native]::VisibleRect((Get-AppHwnd)); $s = (Get-WinInfo).Scale
    Mouse-Drag ($r.Right + 3) ($r.Bottom + 3) ($r.Right + [int](50 * $s)) ($r.Bottom + [int](40 * $s))
    Start-Sleep -Milliseconds 600; $g = Get-WinInfo
    Log "   CONTROL del arrastre: 50 x 40 px hacia FUERA desde 1000 x 640 deja el interior en $($g.LogicalW) x $($g.LogicalH) (el arrastre agarra el borde = $($g.LogicalW -gt 1000))"
    $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    Mouse-Drag ($r.Right + 3) ($r.Bottom + 3) ($r.Right - [int](200 * $s)) ($r.Bottom - [int](160 * $s))
    Start-Sleep -Milliseconds 600; $g2 = Get-WinInfo
    Log "   ARRASTRE hacia dentro 200 x 160 px desde ahi: interior $($g2.LogicalW) x $($g2.LogicalH)"
    [void](Size-Window 1000 640); $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    Mouse-Drag ($r.Right + 3) ($r.Bottom + 3) ($r.Right - [int](150 * $s)) ($r.Bottom - [int](120 * $s))     # el borde de redimensionar queda justo fuera del borde visible
    Start-Sleep -Milliseconds 600; $j = Get-WinInfo
    Log "   ARRASTRE real de la esquina inferior derecha 150 x 120 px hacia dentro desde 1000 x 640: interior $($j.LogicalW) x $($j.LogicalH)"
    Check '(g) Con el raton la ventana no baja de 1000 x 640' ($j.LogicalW -ge 1000 -and $j.LogicalH -ge 640) "$($j.LogicalW) x $($j.LogicalH); por SetWindowPos: $($i.LogicalW) x $($i.LogicalH)"
    if ($j.LogicalW -lt 1000) {
        $act = Find-El -Name 'Abrir GitHub' -Type Button -Timeout 1 -Quiet
        Log "   por debajo del minimo: boton `"Abrir GitHub`" $(if ($null -ne $act) { LRect-Text $act } else { '(no esta)' }) en una ventana de $($j.LogicalW) de ancho"
        Pointer-Away; Shot-Client 'extra-bajo-minimo'
    }
}
}
Step 'PY59 Dejar todo como estaba' -Continue {
    Ensure-Cuentas; Clear-Search; Check 'Ventana a 1120 x 720' (Size-Window 1120 720); Pick-Group 'Trabajo'; Pick-Entry 'GitHub'
    if ("$(Get-Clip)" -match 'FAROPY|centinela-py|example') { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'Portapapeles e historial sin canarios' (-not ("$(Get-Clip)" -match 'FAROPY|centinela-py') -and (History-Has 'FAROPY') -eq 'no' -and (History-Has 'centinela-py') -eq 'no') "borrados del historial: $n"
}
Summary
