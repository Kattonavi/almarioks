# Decisiones del dueno (4), RF-116: la cabecera de la ventana de la entrada (Editar, mas acciones, Cerrar) no se mueve al
# mostrar u ocultar la contrasena ni al desplazar el contenido con la rueda; la contrasena de 500 mostrada ocupa como
# mucho 4 lineas y se desplaza dentro de su fila (rueda y teclado); Ctrl+A + Ctrl+C con el foco en ella no cambia el
# centinela del portapapeles (control: el mismo gesto en la busqueda si copia). RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$Long = Get-State 'pd-500'
if ($Long.Length -ne 500) { throw 'No hay contrasena de 500 en el estado' }
function Head-Y([string]$title) {
    $w = Entry-Window $title 2
    $e = Find-El -Name 'Editar' -Type Button -Root $w -Timeout 1 -Quiet
    $m = Find-El -Name 'Más acciones de la entrada' -Type Button -Root $w -Timeout 1 -Quiet
    $c = Find-El -Name 'Cerrar' -Type Button -Root $w -Timeout 1 -Quiet
    $wr = LRect $w
    [pscustomobject]@{ E = (Y-Of $e); M = (Y-Of $m); C = (Y-Of $c); Win = "$($wr.Y)..$($wr.B)"; Txt = "Editar $(Y-Of $e) · mas acciones $(Y-Of $m) · Cerrar $(Y-Of $c) (ventana y=$($wr.Y) a $($wr.B))" }
}
function Same-Head($a, $b) { foreach ($k in @('E', 'M', 'C')) { if ($a.$k -eq '-' -or $b.$k -eq '-' -or [math]::Abs([double]$a.$k - [double]$b.$k) -gt 1) { return $false } }; return $true }
# Nodo de texto con el valor mostrado y su contenedor enfocable (el .secret con tabindex).
function Secret-Of([string]$title, [string]$value) {
    $w = Entry-Window $title 2
    $t = @($w.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name.Length -ge 8 -and $value.Contains($_.Current.Name) }) | Select-Object -First 1
    if ($null -eq $t) { return $null }
    $chain = @(); $box = $null; $cur = $t
    for ($k = 0; $k -lt 4 -and $null -ne $cur; $k++) {
        $chain += "[$(Type-Of $cur)] $(LRect-Text $cur)$(if ($cur.Current.IsKeyboardFocusable) { ' (enfocable)' })"
        if ($null -eq $box -and $k -gt 0 -and $cur.Current.IsKeyboardFocusable) { $box = $cur }
        $cur = Raw-Parent $cur
    }
    if ($null -eq $box) { $box = Raw-Parent $t }
    [pscustomobject]@{ Text = $t; Box = $box; Chain = ($chain -join '  <  '); Len = $t.Current.Name.Length }
}
# Y de todo lo que no es la cabecera ni el valor, dentro de la ventana (para ver si el cuerpo se desplazo).
function Body-Ys([string]$title) {
    $w = Entry-Window $title 2; $h = @{}
    foreach ($el in @($w.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition))) {
        $n = $el.Current.Name
        if ($n -and $n.Length -lt 40 -and $n -notin @('Editar', 'Más acciones de la entrada', 'Cerrar') -and -not $h.ContainsKey($n)) { $r = LRect $el; if ($null -ne $r) { $h[$n] = $r.Y } }
    }
    return $h
}
function Moved($a, $b) { $m = 0; foreach ($k in $a.Keys) { if ($b.ContainsKey($k) -and [math]::Abs($a[$k] - $b[$k]) -gt 1) { $m++ } }; return $m }

Step 'PD40 Entrada corta: cabecera al mostrar y ocultar; alto de una linea' -Continue {
    Ensure-Pd; [void](Size-Window 1120 720); Clear-Search
    Pick-Entry $PdShort; Click-Name -Name "Abrir $PdShort"; [void](Entry-Window $PdShort 4); Start-Sleep -Milliseconds 600
    $h0 = Head-Y $PdShort
    Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Entry-Window $PdShort 2)); Start-Sleep -Milliseconds 600
    $h1 = Head-Y $PdShort
    $s = Secret-Of $PdShort $PdShortPass
    $one = if ($null -ne $s) { (LRect $s.Box).H } else { -1 }
    Set-State 'pd-una-linea' "$one"
    Log "   VALOR CORTO mostrado: $($s.Chain)"
    Click-El (Find-El -Name 'Ocultar contraseña' -Type Button -Root (Entry-Window $PdShort 2)); Start-Sleep -Milliseconds 600
    $h2 = Head-Y $PdShort
    Check '(3) Entrada corta: la cabecera no se mueve al mostrar ni al ocultar (+-1 px)' ((Same-Head $h0 $h1) -and (Same-Head $h0 $h2)) "antes: $($h0.Txt); mostrada: $($h1.Txt); oculta otra vez: $($h2.Txt); fila del valor de una linea: alto $one"
    Press 'ESC'; Start-Sleep -Milliseconds 500
}
Step 'PD41 Entrada de 500: cabecera, 4 lineas, desplazamiento dentro de la fila' -Continue {
    Ensure-Pd; Clear-Search
    Pick-Entry $PdLong; Click-Name -Name "Abrir $PdLong"; [void](Entry-Window $PdLong 4); Start-Sleep -Milliseconds 600
    $h0 = Head-Y $PdLong
    Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Entry-Window $PdLong 2)); Start-Sleep -Milliseconds 700
    $h1 = Head-Y $PdLong
    $s = Secret-Of $PdLong $Long
    if ($null -eq $s) { throw 'El valor mostrado no esta en el arbol' }
    Log "   VALOR LARGO mostrado ($($s.Len) caracteres en el nodo): $($s.Chain)"
    $box = LRect $s.Box; $one = [double](Get-State 'pd-una-linea')
    $lh = $one - 22; $max4 = 4 * $lh + 22
    Check '(3) Con la de 500 a la vista, la cabecera no se mueve (+-1 px)' (Same-Head $h0 $h1) "antes: $($h0.Txt); mostrada: $($h1.Txt)"
    Check '(3) La contrasena mostrada ocupa como mucho 4 lineas' ($box.H -le $max4 + 1 -and $box.H -ge 3 * $lh + 22 - 1) "fila del valor: $(LRect-Text $s.Box); una linea = $one (texto $lh + 22 de relleno) -> 4 lineas = $max4; nodo de texto completo: $(LRect-Text $s.Text)"
    Pointer-Away; Shot-Client 'app-entrada-500-mostrada-1120x720'
    # Rueda sobre el valor: se desplaza el valor, no la cabecera.
    $c = Mid $s.Box; $ty0 = (LRect $s.Text).Y
    Wheel-At $c[0] $c[1] -2
    $ty1 = (LRect (Secret-Of $PdLong $Long).Text).Y; $h2 = Head-Y $PdLong; $bx = LRect (Secret-Of $PdLong $Long).Box
    Check '(3) La rueda sobre el valor lo desplaza dentro de su fila; la cabecera quieta' ($ty1 -lt $ty0 - 5 -and (Same-Head $h0 $h2) -and [math]::Abs($bx.Y - $box.Y) -le 1) "texto y=$ty0 -> $ty1 (fila y=$($box.Y) -> $($bx.Y)); cabecera: $($h2.Txt)"
    Wheel-At $c[0] $c[1] 6; $ty2 = (LRect (Secret-Of $PdLong $Long).Text).Y
    # Teclado: un clic en el valor (un clic no copia) y flecha abajo / AvPag.
    [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 400
    $foc = Focused-Desc
    $tk0 = (LRect (Secret-Of $PdLong $Long).Text).Y
    Press 'DOWN' 3; Start-Sleep -Milliseconds 400; $tk1 = (LRect (Secret-Of $PdLong $Long).Text).Y
    [QA.Native]::Key(0x22); Start-Sleep -Milliseconds 500; $tk2 = (LRect (Secret-Of $PdLong $Long).Text).Y
    $h3 = Head-Y $PdLong
    Check '(3) Con el foco en el valor, el teclado lo desplaza dentro de su fila; la cabecera quieta' (($tk1 -lt $tk0 - 5 -or $tk2 -lt $tk0 - 5) -and (Same-Head $h0 $h3)) "foco: $foc; texto y=$tk0 (tras rueda arriba: $ty2) -> flecha abajo x3: $tk1 -> AvPag: $tk2; cabecera: $($h3.Txt)"
    # Ctrl+A + Ctrl+C con el foco en el valor: el centinela no cambia.
    [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 400
    $foc2 = Focused-Desc
    Set-ClipExternal 'centinela-pd-sel'; Start-Sleep -Milliseconds 300; $seq0 = Get-ClipSeq
    Press-Ctrl 'a'; Start-Sleep -Milliseconds 200; Press-Ctrl 'c'; Start-Sleep -Milliseconds 700
    $clip = "$(Get-Clip)"; $seq1 = Get-ClipSeq
    Check '(3) Ctrl+A + Ctrl+C con el foco en el valor: el centinela sigue (no se copia por seleccion)' ($clip -ceq 'centinela-pd-sel' -and $seq1 -eq $seq0) "foco: $foc2; portapapeles = centinela: $($clip -ceq 'centinela-pd-sel'); trae el valor o un trozo: $($clip.Contains($Long.Substring(200, 12))); numero de secuencia igual: $($seq1 -eq $seq0)"
    # Rueda sobre otra fila del cuerpo (el usuario): la cabecera no se mueve; anota si el cuerpo se desplazo.
    $u = Find-El -Name 'FAROPD-usuario' -Type Text -Root (Entry-Window $PdLong 2) -Timeout 1 -Quiet
    $b0 = Body-Ys $PdLong
    if ($null -ne $u) { $p = Mid $u } else { $wr = (Entry-Window $PdLong 2).Current.BoundingRectangle; $p = @([int]($wr.X + 40), [int]($wr.Y + $wr.Height - 40)) }
    Wheel-At $p[0] $p[1] -5
    $b1 = Body-Ys $PdLong; $h4 = Head-Y $PdLong
    Wheel-At $p[0] $p[1] 5
    $mv = Moved $b0 $b1
    Check '(3) Rueda sobre el cuerpo: la cabecera no se mueve (+-1 px)' (Same-Head $h0 $h4) "elementos del cuerpo que cambiaron de Y: $mv de $($b0.Count) (0 = el cuerpo no desborda a este tamano); cabecera: $($h4.Txt)"
    Set-State 'pd-cuerpo-movido' "$mv"
    Click-El (Find-El -Name 'Ocultar contraseña' -Type Button -Root (Entry-Window $PdLong 2)); Start-Sleep -Milliseconds 600
    $h5 = Head-Y $PdLong
    Check '(3) Al ocultar la de 500, la cabecera sigue en su sitio' (Same-Head $h0 $h5) "$($h5.Txt)"
    Press 'ESC'; Start-Sleep -Milliseconds 500
    [System.Windows.Forms.Clipboard]::Clear()
}
Step 'PD42 Control: Ctrl+A + Ctrl+C en la busqueda si copia' -Continue {
    Ensure-Pd
    $sr = Find-El -Id 'vault-search' -Timeout 3; Click-El $sr; Type-Text 'centinela-pd-busca'; Start-Sleep -Milliseconds 400
    Set-ClipExternal 'centinela-pd-otro'; Start-Sleep -Milliseconds 300; $seq0 = Get-ClipSeq
    Press-Ctrl 'a'; Start-Sleep -Milliseconds 200; Press-Ctrl 'c'; Start-Sleep -Milliseconds 700
    $clip = "$(Get-Clip)"
    Check '(control) El mismo gesto en la busqueda si cambia el portapapeles' ($clip -ceq 'centinela-pd-busca' -and (Get-ClipSeq) -ne $seq0) "portapapeles = lo tecleado en la busqueda: $($clip -ceq 'centinela-pd-busca')"
    [System.Windows.Forms.Clipboard]::Clear(); Clear-Search
}
Summary
