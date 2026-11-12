# Ronda 2: comprobaciones de ventana real que pidio la revision de seguridad, sobre la entrada "Slack" de la boveda
# de muestra (canario de 40 caracteres sin separadores). Argumento: d10 (seleccion y Ctrl+C sobre el valor mostrado),
# d11 y d11b (U-2: Mostrar y enseguida Editar; d11b con dos clics nativos, PZ_ENTRY y PZ_GAPS eligen entrada y
# separaciones), d12 (bloquear con el valor mostrado) o c (punto 2: sin aviso de version de
# desarrollo). RF_VARIANT=pz-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { 'd10' }
$Entry = if ($env:PZ_ENTRY) { $env:PZ_ENTRY } else { 'Slack' }
$Canary = $PyPass[$Entry]; $Mid8 = if ($Canary.Length -ge 24) { $Canary.Substring(10, 12) } else { $Canary }
$Gaps = if ($env:PZ_GAPS) { @($env:PZ_GAPS -split ' ' | ForEach-Object { [int]$_ }) } else { @(0, 40, 120, 400, 1000) }
function Open-Window {
    if ($null -ne (Entry-Window $Entry 0.3)) { return (Entry-Window $Entry 1) }
    Pick-Entry $Entry; Click-Name -Name "Abrir $Entry"
    $w = Entry-Window $Entry 4; Start-Sleep -Milliseconds 500
    return $w
}
# El texto del valor mostrado; si no esta a la vista pulsa "Mostrar contraseña".
function Shown-El {
    $w = Open-Window
    $t = Find-El -Name $Canary -Type Text -Root $w -Timeout 0.4 -Quiet
    if ($null -eq $t) { Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root $w); Start-Sleep -Milliseconds 600; $t = Find-El -Name $Canary -Type Text -Root (Entry-Window $Entry 1) -Timeout 2 -Quiet }
    if ($null -eq $t) { throw 'El valor mostrado no esta en el arbol' }
    return $t
}
function Hits { @((Tree-Find @{ 'contrasena de Slack' = $Canary; 'trozo' = $Mid8 }).Hits).Count }
function Clip-Desc([string]$sent) {
    $c = Clip-State $Canary
    $what = if ($c.Exact) { 'el VALOR exacto' } elseif ($c.Text -ceq $sent) { 'el centinela' } elseif ($c.Text.Contains($Canary) -or $c.Text.Contains($Mid8)) { "el valor o un trozo dentro de $($c.Len) caracteres" } elseif ($c.Len -eq 0) { 'vacio' } else { "otro texto de $($c.Len) caracteres, sin el valor" }
    [pscustomobject]@{ What = "$what, $($c.Excl)/3 formatos de exclusion"; HasValue = ($c.Text.Contains($Canary) -or $c.Text.Contains($Mid8)); Excl = $c.Excl; Text = $c.Text }
}
$script:SentN = 0
# Centinela -> gesto -> que quedo -> Ctrl+C -> que quedo. Fuga por el navegador = el valor sin los tres formatos del
# nucleo, o un Ctrl+C que cambia el portapapeles y deja el valor.
function Gesture([string]$label, [scriptblock]$g, [string]$note = '') {
    $script:SentN++; $sent = "centinela-pz-$SentN"
    Set-ClipExternal $sent; Start-Sleep -Milliseconds 350; $s0 = Get-ClipSeq
    & $g; Start-Sleep -Milliseconds 700
    $a = Clip-Desc $sent; $s1 = Get-ClipSeq
    [void](Focus-App); Press-Ctrl 'c'; Start-Sleep -Milliseconds 800
    $b = Clip-Desc $sent; $s2 = Get-ClipSeq
    $leak = ($a.HasValue -and $a.Excl -lt 3) -or ($b.HasValue -and ($s2 -ne $s1 -or $b.Excl -lt 3))
    Check "(10) $label + Ctrl+C: el valor no llega al portapapeles por la via del navegador" (-not $leak) "tras el gesto: $($a.What) (el portapapeles cambio = $($s1 -ne $s0)); tras Ctrl+C: $($b.What) (Ctrl+C cambio el portapapeles = $($s2 -ne $s1)); el valor sigue a la vista = $($null -ne (Find-El -Name $Canary -Type Text -Timeout 0.3 -Quiet))$note"
}

if ($only -eq 'd10') {
Step 'PZD10 Seleccion y Ctrl+C sobre la contrasena mostrada' -Continue {
    Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search; Pick-Group 'Trabajo'
    $t = Shown-El
    Check '(10) CONTROL: "Mostrar contraseña" deja el canario en el arbol (el detector lo ve)' ((Hits) -gt 0) "rectangulo del valor: $(LRect-Text $t)"
    Gesture 'Doble clic sobre el valor mostrado' { $m = Mid (Shown-El); Dbl-At $m[0] $m[1] } '; aviso: '
    Log "   aviso tras el doble clic: $((Names-Like 'copiad') -join ' | ')"
    Gesture 'Triple clic sobre el valor mostrado' { $m = Mid (Shown-El); [void](Focus-App); [QA.Native]::ClickAt($m[0], $m[1]); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($m[0], $m[1]); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($m[0], $m[1]) }
    Gesture 'Arrastre de punta a punta del valor' { $r = (Shown-El).Current.BoundingRectangle; $y = [int]($r.Y + $r.Height / 2); [void](Focus-App); Mouse-Drag ([int]($r.X + 1)) $y ([int]($r.X + $r.Width - 1)) $y }
    Pointer-Away; Shot-Client 'pz-d10-tras-arrastre'
    Gesture 'Arrastre desde el rotulo de arriba hasta debajo del valor' { $r = (Shown-El).Current.BoundingRectangle; [void](Focus-App); Mouse-Drag ([int]($r.X - 60)) ([int]($r.Y - 40)) ([int]($r.X + $r.Width + 4)) ([int]($r.Y + $r.Height + 30)) 20 }
    Gesture 'Ctrl+A con el valor a la vista' { [void](Shown-El); [void](Focus-App); Press-Ctrl 'a' }
    # Controles: (i) otro texto de la ventana seleccionado con arrastre; (ii) un campo de texto con Ctrl+A y Ctrl+C.
    $w = Open-Window; $wr = LRect $w
    $other = @($w.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name.Length -ge 5 -and $_.Current.Name -cne $Canary -and (LRect $_).X -gt ($wr.X + 120) -and (LRect $_).Y -gt ($wr.Y + 80) } | Select-Object -First 1)
    if ($other.Count) {
        $r = $other[0].Current.BoundingRectangle; $y = [int]($r.Y + $r.Height / 2)
        Set-ClipExternal 'centinela-pz-control-a'; Start-Sleep -Milliseconds 300; $s0 = Get-ClipSeq
        [void](Focus-App); Mouse-Drag ([int]($r.X + 1)) $y ([int]($r.X + $r.Width - 1)) $y; Press-Ctrl 'c'; Start-Sleep -Milliseconds 700
        Log "   CONTROL (i): arrastre + Ctrl+C sobre otro texto de la ventana (`"$($other[0].Current.Name)`"): portapapeles = `"$(Get-Clip)`"; cambio = $((Get-ClipSeq) -ne $s0); formatos: $((Get-ClipFormats) -join ', ')"
    }
    Press 'ESC'; Start-Sleep -Milliseconds 600
    Fill-Id 'vault-search' 'centinela-pz-control-b'; Start-Sleep -Milliseconds 300
    Set-ClipExternal 'centinela-pz-antes'; Start-Sleep -Milliseconds 300
    Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 700
    $cs = Clip-State 'centinela-pz-control-b'
    Check '(10) CONTROL (ii): Ctrl+A y Ctrl+C en un campo de texto de la app SI copian por la via del navegador (sin formatos de exclusion)' ($cs.Exact -and $cs.Excl -eq 0) "portapapeles exacto = $($cs.Exact); formatos de exclusion $($cs.Excl)/3; formatos: $($cs.Formats)"
    Clear-Search; Start-Sleep -Milliseconds 400
    Start-Sleep -Milliseconds 1500
    Check '(10) El historial de Windows no tiene el canario tras los gestos' ((History-Has $Canary) -eq 'no' -and (History-Has $Mid8) -eq 'no') "canario: $(History-Has $Canary); trozo: $(History-Has $Mid8); CONTROL: el centinela 1 si esta en el historial = $(History-Has 'centinela-pz-1')"
}
Step 'PZD10b Doble clic en el usuario de la lista y Ctrl+C' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Trabajo'; Pick-Entry 'Correo de la oficina'; Start-Sleep -Milliseconds 300
    $c = Cell-Of $Entry 'USUARIO'; $m = Mid $c; $user = "$($c.Current.Name)"
    Set-ClipExternal 'centinela-pz-lista'; Start-Sleep -Milliseconds 300; $s0 = Get-ClipSeq
    Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 600
    $c1 = "$(Get-Clip)"; $f1 = (Clip-State $c1).Excl; $s1 = Get-ClipSeq
    [void](Focus-App); Press-Ctrl 'c'; Start-Sleep -Milliseconds 800
    $c2 = "$(Get-Clip)"; $f2 = (Clip-State $c2).Excl; $s2 = Get-ClipSeq
    Check '(10) Lista: doble clic en el usuario y luego Ctrl+C no deja nada nuevo en el portapapeles' ($s2 -eq $s1 -and $c2 -ceq $c1) "celda `"$user`"; tras el doble clic: `"$c1`" con $f1/3 formatos de exclusion (cambio = $($s1 -ne $s0)); tras Ctrl+C: `"$c2`" con $f2/3 (Ctrl+C cambio el portapapeles = $($s2 -ne $s1)); contiene alguna contrasena = $($c2 -match 'FAROPY'); foco: $(Focused-Desc)"
    [System.Windows.Forms.Clipboard]::Clear()
}
}

if ($only -eq 'd11') {
Step 'PZD11 (U-2) Mostrar y enseguida Editar, luego Cancelar' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Trabajo'
    foreach ($gap in @(0, 30, 100, 250)) {
        if ($null -ne (Entry-Window $Entry 0.3)) { Press 'ESC'; Start-Sleep -Milliseconds 500 }
        $w = Open-Window
        if ($null -ne (Find-El -Name $Canary -Type Text -Root $w -Timeout 0.3 -Quiet)) { throw 'La ventana se abrio con el valor a la vista' }
        $m = Mid (Find-El -Name 'Mostrar contraseña' -Type Button -Root $w); $e = Mid (Find-El -Name 'Editar' -Type Button -Root $w)
        [void](Focus-App); Start-Sleep -Milliseconds 200
        $sw = [Diagnostics.Stopwatch]::StartNew()
        [QA.Native]::ClickAt($m[0], $m[1]); $t1 = $sw.ElapsedMilliseconds
        if ($gap) { Start-Sleep -Milliseconds $gap }
        [QA.Native]::ClickAt($e[0], $e[1]); $t2 = $sw.ElapsedMilliseconds
        $save = Find-El -Name 'Guardar entrada' -Type Button -Timeout 4 -Quiet
        Start-Sleep -Milliseconds 500
        if ($null -eq $save) { Log "   separacion $gap ms: el formulario de editar NO se abrio (segundo clic a los $t2 ms); valor a la vista = $((Hits) -gt 0)"; continue }
        Click-El (Find-El -Name 'Cancelar' -Type Button); Start-Sleep -Milliseconds 150
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet; if ($null -ne $ask) { Click-El $ask }
        $h1 = Hits; Start-Sleep -Milliseconds 1200; $h2 = Hits
        $w2 = Entry-Window $Entry 1
        $btn = if ($null -ne $w2) { (@($w2.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match 'contraseña' } | ForEach-Object { $_.Current.Name }) -join ' / ') } else { '(la ventana de la entrada no esta)' }
        Check "(11) Mostrar y Editar con $gap ms entre los dos clics, luego Cancelar: la contrasena NO queda a la vista" ($h1 -eq 0 -and $h2 -eq 0 -and $null -ne $w2) "clics a los $t1 y $t2 ms; hallazgos del canario al volver: $h1; un segundo despues: $h2; botones de la contrasena: $btn"
    }
    $t = Shown-El
    Check '(11) CONTROL: un "Mostrar contraseña" normal en la misma ventana si la deja a la vista' ((Hits) -gt 0)
    Press 'ESC'; Start-Sleep -Milliseconds 500
}
}

if ($only -eq 'd11b') {
Add-Type -TypeDefinition @"
using System; using System.Runtime.InteropServices; using System.Threading;
namespace PZ { public static class Fast {
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  // Dos clics seguidos en dos puntos distintos, sin nada en medio salvo la separacion pedida.
  public static void Two(int x1, int y1, int x2, int y2, int gapMs) {
    SetCursorPos(x1, y1); Thread.Sleep(40);
    mouse_event(2, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(12); mouse_event(4, 0, 0, 0, UIntPtr.Zero);
    if (gapMs > 0) Thread.Sleep(gapMs);
    SetCursorPos(x2, y2); Thread.Sleep(12);
    mouse_event(2, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(12); mouse_event(4, 0, 0, 0, UIntPtr.Zero);
  } } }
"@
Step 'PZD11b (U-2) Mostrar y enseguida Editar (dos clics nativos seguidos), luego Cancelar' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Trabajo'
    foreach ($gap in $Gaps) {
        if ($null -ne (Find-El -Name 'Editar entrada' -Type Window -Timeout 0.3 -Quiet)) { Click-El (Find-El -Name 'Cancelar' -Type Button); Start-Sleep -Milliseconds 500 }
        if ($null -ne (Entry-Window $Entry 0.3)) { Press 'ESC'; Start-Sleep -Milliseconds 500 }
        $w = Open-Window
        if ($null -ne (Find-El -Name $Canary -Type Text -Root $w -Timeout 0.3 -Quiet)) { throw 'La ventana se abrio con el valor a la vista' }
        $m = Mid (Find-El -Name 'Mostrar contraseña' -Type Button -Root $w); $e = Mid (Find-El -Name 'Editar' -Type Button -Root $w)
        $ed0 = LRect (Find-El -Name 'Editar' -Type Button -Root $w); $wr0 = LRect $w
        [void](Focus-App); Start-Sleep -Milliseconds 250
        [PZ.Fast]::Two($m[0], $m[1], $e[0], $e[1], $gap)
        $sw = [Diagnostics.Stopwatch]::StartNew(); $formAt = -1
        while ($sw.ElapsedMilliseconds -lt 2500 -and $formAt -lt 0) { if ($null -ne (Find-El -Name 'Editar entrada' -Type Window -Timeout 0.05 -Quiet)) { $formAt = [int]$sw.ElapsedMilliseconds } }
        $how = "el formulario abrio con los dos clics seguidos (visto a los $formAt ms)"
        if ($formAt -lt 0) {
            $shown = $null -ne (Find-El -Name $Canary -Type Text -Timeout 0.3 -Quiet)
            $under = ''; try { $u = $AE::FromPoint((New-Object System.Windows.Point($e[0], $e[1]))); $under = "[$(Type-Of $u)] `"$($u.Current.Name)`" activo=$($u.Current.IsEnabled)" } catch { }
            $w1 = Entry-Window $Entry 1; $ed1 = LRect (Find-El -Name 'Editar' -Type Button -Root $w1); $wr1 = LRect $w1
            Log "   SALTO al mostrar ($Entry, $($Canary.Length) caracteres): ventana y=$($wr0.Y) alto=$($wr0.H) -> y=$($wr1.Y) alto=$($wr1.H); boton Editar y=$($ed0.Y) -> y=$($ed1.Y) (alto $($ed1.H))"
            Click-El (Find-El -Name 'Editar' -Type Button -Root $w1)
            $ok = $null -ne (Find-El -Name 'Editar entrada' -Type Window -Timeout 4 -Quiet)
            $how = "el segundo clic NO abrio el formulario (valor a la vista = $shown; bajo el puntero: $under); un tercer clic normal en Editar, con el valor ya a la vista, lo abrio = $ok"
            if (-not $ok) { Log "   separacion $gap ms: $how"; continue }
        }
        Start-Sleep -Milliseconds 700
        Click-El (Find-El -Name 'Cancelar' -Type Button); Start-Sleep -Milliseconds 200
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet; if ($null -ne $ask) { Click-El $ask }
        $h1 = Hits; Start-Sleep -Milliseconds 1200; $h2 = Hits
        $w2 = Entry-Window $Entry 1
        $btn = if ($null -ne $w2) { (@($w2.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match 'contraseña' } | ForEach-Object { $_.Current.Name }) -join ' / ') } else { '(la ventana de la entrada no esta)' }
        Check "(11) Mostrar y Editar con $gap ms entre los dos clics, luego Cancelar: la contrasena NO queda a la vista" ($h1 -eq 0 -and $h2 -eq 0 -and $null -ne $w2) "$how; hallazgos del canario al volver: $h1; 1,2 s despues: $h2; botones de la contrasena: $btn"
    }
    if ($null -ne (Entry-Window $Entry 0.3)) { Press 'ESC'; Start-Sleep -Milliseconds 500 }
}
}

if ($only -eq 'd12') {
Step 'PZD12 Bloquear con un valor mostrado' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Trabajo'
    $lockBtn = Mid (Find-El -Name 'Bloquear' -Type Button)
    $t = Shown-El
    $h0 = Hits
    $method = ''
    [void](Focus-App); [QA.Native]::Chord(0x11, [byte][char]'L'); Start-Sleep -Milliseconds 1500
    if (Has-El -Id 'unlock-password' -Timeout 1) { $method = 'Ctrl+L con la ventana de la entrada abierta' }
    if (-not $method) {
        [QA.Native]::ClickAt($lockBtn[0], $lockBtn[1]); Start-Sleep -Milliseconds 1200
        if (Has-El -Id 'unlock-password' -Timeout 1) { $method = 'clic en "Bloquear" (detras de la ventana de la entrada)' }
        else { Log "   Ctrl+L no bloquea y el clic en Bloquear detras de la ventana tampoco: ventana de la entrada abierta = $($null -ne (Entry-Window $Entry 0.5)); valor a la vista = $((Hits) -gt 0)" }
    }
    if (-not $method) {
        if ($null -ne (Entry-Window $Entry 0.3)) { Press 'ESC'; Start-Sleep -Milliseconds 500 }
        Click-Name -Name 'Ajustes y Acerca de'; [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 400
        Log "   opciones de bloqueo automatico: $((List-Options 'settings-auto-lock') -join ' | ')"
        Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 400
        $set = Set-AutoLock '1 min*'
        Log "   bloqueo automatico puesto en 1 minuto = $set"
        Set-State 'pz-autolock-cambiado' '1'
        $t = Shown-El; $h0 = Hits; Pointer-Away
        $sw = [Diagnostics.Stopwatch]::StartNew(); $lastSeen = 0
        while ($sw.Elapsed.TotalSeconds -lt 100) {
            if (Has-El -Id 'unlock-password' -Timeout 0.2) { $method = "bloqueo automatico de 1 minuto (a los $([int]$sw.Elapsed.TotalSeconds) s; el valor siguio a la vista hasta el segundo $lastSeen)"; break }
            if ($null -ne (Find-El -Name $Canary -Type Text -Timeout 0.2 -Quiet)) { $lastSeen = [int]$sw.Elapsed.TotalSeconds }
            Start-Sleep -Milliseconds 1500
        }
    }
    if (-not $method) { Check '(12) Bloquear con un valor mostrado' $false 'NO PROBADO: no se consiguio bloquear con la ventana de la entrada abierta'; return }
    Start-Sleep -Milliseconds 1200
    $hL = Hits
    Check '(12) CONTROL: antes de bloquear el valor estaba a la vista; en la pantalla de desbloqueo ya no esta en el arbol' ($h0 -gt 0 -and $hL -eq 0) "metodo: $method; hallazgos antes: $h0; bloqueada: $hL"
    Fill-Id 'unlock-password' $RfMaster -Secret; Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 30); $h1 = Hits; Start-Sleep -Milliseconds 1500; $h2 = Hits
    Check '(12) Tras desbloquear, el valor no esta a la vista ni en el arbol' ($h1 -eq 0 -and $h2 -eq 0) "hallazgos al abrir: $h1; 1,5 s despues: $h2; ventana de la entrada abierta = $($null -ne (Entry-Window $Entry 0.5)); foco: $(Focused-Desc)"
    if ((Get-State 'pz-autolock-cambiado') -eq '1') { $back = Set-AutoLock '5 min*'; Log "   bloqueo automatico de vuelta en 5 minutos = $back"; if ($back) { Set-State 'pz-autolock-cambiado' '0' } }
}
}

if ($only -eq 'c') {
function Scan-Screen([string]$where) {
    $all = Tree-Strings
    $low = @($all | ForEach-Object { $_.Text.ToLowerInvariant() })
    $bad = @()
    foreach ($n in @('contraseñas reales', 'versión de desarrollo', 'version de desarrollo', 'desarrollo', 'no la uses')) { if (@($low | Where-Object { $_.Contains($n) }).Count) { $bad += $n } }
    $flat = ($low -join ' ') -replace '\s+', ' '
    $ver = $flat.Contains('versión 0.1.0')
    Check "(C) $where`: sin `"contraseñas reales`" ni `"Versión de desarrollo`"" ($bad.Count -eq 0) "$($all.Count) textos recorridos; hallazgos: $(if ($bad.Count) { $bad -join ', ' } else { 'ninguno' }); CONTROL: el recorrido encuentra `"Versión 0.1.0`" = $ver; licencia a la vista = $($flat.Contains('gpl'))"
    return $ver
}
Step 'PZC Punto 2: sin aviso de version de desarrollo' -Continue {
    Ensure-Cuentas; Clear-Search
    [void](Scan-Screen 'Boveda abierta')
    Click-Name -Name 'Ajustes y Acerca de'; [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 500
    $v1 = Scan-Screen '"Ajustes y Acerca de"'
    Log "   textos de Acerca de: $((Names-Like 'Versi|Licencia|GPL|AlmarioKS|0\.1\.0|firma|auditor') -join ' | ')"
    Pointer-Away; Shot-Client 'pz-c-ajustes'
    Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 400
    [void](To-Welcome)
    $v2 = Scan-Screen 'Bienvenida (pie de la app)'
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
    $v3 = Scan-Screen 'Crear bóveda (pie de la app)'
    Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 500
    Check '(C) CONTROL: "Versión 0.1.0" se lee en Acerca de, en la bienvenida y en crear' ($v1 -and $v2 -and $v3) "Acerca de = $v1; bienvenida = $v2; crear = $v3"
}
}
Summary
