# Ronda 2, puntos 3 y 4e: el generador en la app instalada. Argumento: a1 (500 caracteres: generar, guardar, mostrar,
# copiar y leer con KeePassLib), a2 (7, 501, vacio y 65) o a3 (200 con las opciones por defecto). RF_VARIANT=pz-lib.
# Las contrasenas generadas se guardan en la carpeta de estado (clave "generadas") para el rastro en disco; no se imprimen.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { 'a1' }
$ExactId = 'entry-dialog-password-length-exact'
function Open-NewEntry([switch]$Options) {
    Click-Name -Name 'Nueva entrada'
    [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    if ($Options) { Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 500 }
}
# Lee el campo de contrasena del dialogo. Si sigue enmascarado pulsa "Mostrar contraseña" y lo anota.
function Read-Field {
    $pw = Find-El -Id 'entry-dialog-password' -Timeout 3
    $masked = $pw.Current.IsPassword
    if ($masked) { Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Find-El -Name 'Nueva entrada' -Type Window)); Start-Sleep -Milliseconds 400; $pw = Find-El -Id 'entry-dialog-password' -Timeout 3 }
    $v = "$(Raw-Value $pw)"
    [pscustomobject]@{ Value = $v; WasMasked = $masked; StillMasked = $pw.Current.IsPassword; Bullets = ($v.Length -gt 0 -and ($v.ToCharArray() | Select-Object -Unique).Count -eq 1) }
}
function Gen-Click { Click-El (Find-El -Name 'Generar' -Type Button); Start-Sleep -Milliseconds 600 }
function Close-Dialog-Discard {
    if (-not (Dialog-Open 'Nueva entrada')) { return 'ya cerrado' }
    Click-El (Find-El -Name 'Cancelar' -Type Button); Start-Sleep -Milliseconds 600
    $ask = Find-El -Name 'Descartar' -Type Button -Timeout 1.5 -Quiet
    $txt = (Names-Like 'escart|cambios|perder|Seguir') -join ' | '
    if ($null -ne $ask) { Click-El $ask; Start-Sleep -Milliseconds 600 }
    return "pregunta = $($null -ne $ask) ($txt); cerrado = $(-not (Dialog-Open 'Nueva entrada'))"
}

if ($only -eq 'a1') {
Step 'PZA1 500 caracteres: generar y guardar' {
    Ensure-Cuentas; Check 'Ventana a 1120 x 720' (Size-Window 1120 720); Clear-Search; Pick-Group 'Personal'
    if (Has-Entry $PzTitle) { throw 'La entrada de 500 ya existe: no se repite' }
    Open-NewEntry
    Type-Text $PzTitle
    Fill-Id 'entry-dialog-username' 'FAROPZ-usuario'
    Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 500
    Fill-Id $ExactId '500'; Start-Sleep -Milliseconds 300
    Log "   casilla = `"$(Get-Value (Find-El -Id $ExactId))`"; deslizante = `"$(Get-Value (Find-El -Id 'entry-dialog-password-length'))`"; avisos: $((Names-Like 'Entre 8|caracteres') -join ' | ')"
    Gen-Click
    $f = Read-Field
    $c = Classes $f.Value
    Check '(1) Con 500 en la casilla, "Generar" llena el campo con 500 caracteres' ($f.Value.Length -eq 500 -and -not $f.Bullets -and -not $f.StillMasked) "longitud leida $($f.Value.Length); a la vista sin pulsar Mostrar = $(-not $f.WasMasked); minusculas/mayusculas/numeros/simbolos = $($c -join '/'); distintos = $(($f.Value.ToCharArray() | Select-Object -Unique).Count); con espacios o saltos = $($f.Value -match '\s')"
    Set-State 'pz-500' $f.Value; Add-Generated $f.Value
    Log "   deslizante tras generar = `"$(Get-Value (Find-El -Id 'entry-dialog-password-length'))`"; casilla = `"$(Get-Value (Find-El -Id $ExactId))`""
    $w = Find-El -Name 'Nueva entrada' -Type Window; $wr = LRect $w
    $maxR = 0; $worst = ''
    foreach ($el in @($w.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition))) { $r = LRect $el; if ($null -ne $r -and $r.W -gt 0 -and $r.R -gt $maxR) { $maxR = $r.R; $worst = "[$(Type-Of $el)] id=$($el.Current.AutomationId)" } }
    Check '(1) El dialogo no se desborda a lo ancho con 500 caracteres en el campo' ($maxR -le $wr.R + 0.5 -and $wr.R -le 1120 -and $wr.X -ge 0) "dialogo x=$($wr.X) a $($wr.R) (ancho $($wr.W)); borde derecho maximo de su contenido = $maxR ($worst); campo: $(LRect-Text (Find-El -Id 'entry-dialog-password'))"
    Pointer-Away; Shot-Client 'pz-a1-dialogo-500'
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    Click-El (Find-El -Name 'Guardar entrada' -Type Button)
    $sw = [Diagnostics.Stopwatch]::StartNew(); $closed = $false; $disk = $false
    while ($sw.Elapsed.TotalSeconds -lt 20 -and -not ($closed -and $disk)) {
        if (-not $closed -and -not (Dialog-Open 'Nueva entrada')) { $closed = $true }
        if (-not $disk -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $disk = $true }
        Start-Sleep -Milliseconds 60
    }
    Start-Sleep -Milliseconds 700
    Check '(1) "Guardar entrada" cierra el dialogo y escribe la boveda' ($closed -and $disk) "cerrado=$closed disco=$disk; avisos: $((Names-Like 'Entre |No se pudo|demasiado|caracteres') -join ' | ')"
}
Step 'PZA2 500 caracteres: mostrar en la ventana de la entrada, copiar y KeePassLib' -Continue {
    $value = Get-State 'pz-500'
    if ($value.Length -ne 500) { throw 'No hay contrasena de 500 guardada en el estado' }
    Ensure-Cuentas; Clear-Search; Pick-Group 'Personal'
    Pick-Entry $PzTitle; Click-Name -Name "Abrir $PzTitle"
    $win = Entry-Window $PzTitle 4; Start-Sleep -Milliseconds 500
    $wr0 = LRect $win
    Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root $win); Start-Sleep -Milliseconds 700
    $win = Entry-Window $PzTitle 2; $wr = LRect $win
    $texts = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Text' })
    $exact = @($texts | Where-Object { $_.Current.Name -ceq $value })
    $parts = @($texts | Where-Object { $_.Current.Name.Length -ge 8 -and $value.Contains($_.Current.Name) })
    $concat = -join @($parts | ForEach-Object { $_.Current.Name })
    $doc = ''; foreach ($s in (Tree-Strings)) { if ($s.Where -eq 'TextPattern del documento') { $doc = $s.Text } }
    $docFlat = $doc -replace '\s', ''
    Check '(1) "Mostrar contraseña" en la ventana de la entrada: los 500 caracteres estan en el arbol' (($exact.Count -ge 1) -or ($concat -ceq $value)) "nodos de texto con el valor exacto: $($exact.Count); trozos: $($parts.Count) que suman $($concat.Length) caracteres (iguales en orden = $($concat -ceq $value)); el texto del documento la trae entera = $($doc.Contains($value)) (sin espacios ni saltos = $($docFlat.Contains($value)))"
    $maxR = 0; $maxB = 0; $worst = ''
    foreach ($el in @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition))) { $r = LRect $el; if ($null -ne $r -and $r.W -gt 0) { if ($r.R -gt $maxR) { $maxR = $r.R; $worst = "[$(Type-Of $el)] `"$(if ($el.Current.Name.Length -gt 30) { '<largo>' } else { $el.Current.Name })`"" }; if ($r.B -gt $maxB) { $maxB = $r.B } } }
    Check '(1) La ventana de la entrada no se desborda a lo ancho con la contrasena de 500 a la vista' ($maxR -le $wr.R + 0.5 -and $wr.R -le 1120 -and $wr.X -ge 0) "ventana: x=$($wr.X) y=$($wr.Y) w=$($wr.W) h=$($wr.H) (antes w=$($wr0.W) h=$($wr0.H)); borde derecho maximo del contenido = $maxR ($worst); borde inferior maximo = $maxB (ventana hasta $($wr.B))"
    foreach ($t in @($exact + $parts | Select-Object -First 3)) {
        $chain = @(); $cur = $t
        for ($k = 0; $k -lt 4 -and $null -ne $cur; $k++) { $sp = $null; $scr = if ($cur.TryGetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern, [ref]$sp)) { " desplazable(H=$($sp.Current.HorizontallyScrollable),V=$($sp.Current.VerticallyScrollable))" } else { '' }
            $chain += "[$(Type-Of $cur)] $(LRect-Text $cur)$(if ($cur.Current.IsOffscreen) { ' (fuera)' })$scr"; $cur = Raw-Parent $cur }
        Log "   VALOR MOSTRADO ($($t.Current.Name.Length) caracteres) y sus contenedores: $($chain -join '  <  ')"
    }
    Log "   botones de la ventana: $((@($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) | Where-Object { (Type-Of $_) -eq 'Button' } | ForEach-Object { "$($_.Current.Name) $(LRect-Text $_)" }) -join ' | ')"
    Pointer-Away; Shot-Client 'pz-a1-entrada-500'
    Set-ClipExternal 'centinela-pz-a1'; Start-Sleep -Milliseconds 300
    Click-El (Find-El -Name 'Copiar contraseña' -Type Button -Root (Entry-Window $PzTitle 2)); Start-Sleep -Milliseconds 700
    $cs = Clip-State $value
    Start-Sleep -Milliseconds 1500
    Check '(1) "Copiar contraseña": el portapapeles tiene los mismos 500' ($cs.Exact -and $cs.Len -eq 500) "longitud $($cs.Len); exacta = $($cs.Exact); formatos de exclusion $($cs.Excl) de 3; en el historial de Windows: $(History-Has $value.Substring(100, 40)); aviso: $((Names-Like 'copiad') -join ' | ')"
    [System.Windows.Forms.Clipboard]::Clear()
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $exe = Join-Path $env:ProgramFiles 'KeePass Password Safe 2\KeePass.exe'
    if (-not (Test-Path $exe)) { Check '(1) KeePassLib lee los mismos 500' $false 'NO PROBADO: KeePass 2 no esta instalado' }
    else {
        [Reflection.Assembly]::LoadFrom($exe) | Out-Null
        $key = [KeePassLib.Keys.CompositeKey]::new(); $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($RfMaster))
        $db = [KeePassLib.PwDatabase]::new(); $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($RfVault), $key, $null)
        $read = $null; $n = 0; $ctl = ''
        foreach ($e in $db.RootGroup.GetEntries($true)) { $n++; if ($e.Strings.ReadSafe('Title') -ceq $PzTitle) { $read = $e.Strings.ReadSafe('Password') }; if ($e.Strings.ReadSafe('Title') -ceq 'Jira') { $ctl = $e.Strings.ReadSafe('Password') } }
        $db.Close()
        Check '(1) KeePassLib lee del .kdbx los mismos 500' ($read -ceq $value) "KeePassLib $([Diagnostics.FileVersionInfo]::GetVersionInfo($exe).ProductVersion); entradas leidas $n; longitud leida $("$read".Length); CONTROL: la de Jira leida es su canario = $($ctl -ceq $PyPass['Jira'])"
    }
}
}

if ($only -eq 'a2') {
Step 'PZA3 Fuera de 8 a 500 no genera; 65 deja el deslizante al tope' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Personal'
    Open-NewEntry -Options
    Log "   al abrir: casilla = `"$(Get-Value (Find-El -Id $ExactId))`"; deslizante = `"$(Get-Value (Find-El -Id 'entry-dialog-password-length'))`""
    Gen-Click; $base = Read-Field; Add-Generated $base.Value
    Log "   CONTROL: con la casilla por defecto genera $($base.Value.Length) caracteres (a la vista sin pulsar Mostrar = $(-not $base.WasMasked))"
    foreach ($n in @('7', '501', '')) {
        Fill-Id $ExactId $n; Start-Sleep -Milliseconds 350
        $shown = "$(Get-Value (Find-El -Id $ExactId))"
        $gen = Find-El -Name 'Generar' -Type Button
        $enabled = $gen.Current.IsEnabled
        $msgBefore = (Names-Like '^Entre 8 y 500') -join ' | '
        if ($enabled) { Click-El $gen; Start-Sleep -Milliseconds 700 }
        $after = Read-Field
        $msg = (Names-Like '^Entre 8 y 500') -join ' | '
        Check "(2) Con `"$n`" en la casilla no genera, dice `"Entre 8 y 500 caracteres.`" y el campo queda como estaba" ($after.Value -ceq $base.Value -and $msg -ceq 'Entre 8 y 500 caracteres.') "la casilla muestra `"$shown`" justo antes de Generar; boton Generar activo = $enabled; aviso antes de pulsar: `"$msgBefore`"; despues: `"$msg`"; campo igual = $($after.Value -ceq $base.Value) ($($after.Value.Length) caracteres); deslizante = `"$(Get-Value (Find-El -Id 'entry-dialog-password-length'))`""
        if ($after.Value -cne $base.Value) { Add-Generated $after.Value; $base = $after }
    }
    Pointer-Away; Shot-Client 'pz-a2-fuera-de-rango'
    Fill-Id $ExactId '65'; Start-Sleep -Milliseconds 350
    Gen-Click; $v65 = Read-Field; Add-Generated $v65.Value
    $sl = Find-El -Id 'entry-dialog-password-length'; $rp = $null; $max = '?'
    if ($sl.TryGetCurrentPattern([System.Windows.Automation.RangeValuePattern]::Pattern, [ref]$rp)) { $max = "$($rp.Current.Value) de $($rp.Current.Minimum)-$($rp.Current.Maximum)" }
    Check '(2) Con 65 genera 65 y el deslizante queda al tope (64)' ($v65.Value.Length -eq 65 -and "$(Get-Value $sl)" -eq '64') "longitud $($v65.Value.Length); deslizante valor=`"$(Get-Value $sl)`" rango: $max; aviso de rango a la vista: `"$((Names-Like '^Entre 8 y 500') -join ' | ')`""
    foreach ($n in @('8', '500')) { Fill-Id $ExactId $n; Start-Sleep -Milliseconds 300; Gen-Click; $x = Read-Field; Add-Generated $x.Value; Log "   limite $n -> genera $($x.Value.Length) caracteres" }
    Log "   cancelar: $(Close-Dialog-Discard)"
}
}

if ($only -eq 'a2b') {
Step 'PZA3b Que cambia el campo: teclear en la casilla, pegar 501 y "Generar"' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Personal'
    Open-NewEntry -Options
    Gen-Click; $v0 = (Read-Field).Value; Add-Generated $v0
    Fill-Id $ExactId '30'; Start-Sleep -Milliseconds 600; $v1 = (Read-Field).Value; Add-Generated $v1
    Log "   teclear 30 en la casilla SIN pulsar Generar: el campo cambio = $($v1 -cne $v0); longitud $($v0.Length) -> $($v1.Length)"
    Set-ClipExternal '501'; Click-El (Find-El -Id $ExactId); Press-Ctrl 'a'; Press-Ctrl 'v'; Start-Sleep -Milliseconds 600
    $shown = "$(Get-Value (Find-El -Id $ExactId))"; $v2 = (Read-Field).Value; Add-Generated $v2
    $gen = Find-El -Name 'Generar' -Type Button; $en = $gen.Current.IsEnabled
    if ($en) { Click-El $gen; Start-Sleep -Milliseconds 800 }
    $v3 = (Read-Field).Value; Add-Generated $v3
    Check '(2) 501 PEGADO en la casilla (sin pasar por 50): no genera, avisa y el campo queda como estaba' ($shown -eq '501' -and $v2 -ceq $v1 -and $v3 -ceq $v1 -and ((Names-Like '^Entre 8 y 500') -join '|') -ceq 'Entre 8 y 500 caracteres.') "casilla = `"$shown`"; el campo cambio al pegar = $($v2 -cne $v1); Generar activo = $en; el campo cambio al pulsar Generar = $($v3 -cne $v2) ($($v3.Length) caracteres); aviso: `"$((Names-Like '^Entre 8 y 500') -join '|')`""
    Fill-Id $ExactId '20'; Start-Sleep -Milliseconds 600; $v4 = (Read-Field).Value; Add-Generated $v4
    Fill-Id $ExactId '501'; Start-Sleep -Milliseconds 600; $v5 = (Read-Field).Value; Add-Generated $v5
    Click-El (Find-El -Name 'Generar' -Type Button); Start-Sleep -Milliseconds 800; $v6 = (Read-Field).Value; Add-Generated $v6
    Log "   501 TECLEADO (pasa por 5 y por 50): campo antes $($v4.Length) caracteres; tras teclear, sin pulsar Generar: cambio = $($v5 -cne $v4), $($v5.Length) caracteres; tras pulsar Generar: cambio = $($v6 -cne $v5), $($v6.Length) caracteres; aviso: `"$((Names-Like '^Entre 8 y 500') -join '|')`""
    Log "   cancelar: $(Close-Dialog-Discard)"
    [System.Windows.Forms.Clipboard]::Clear()
    Open-NewEntry
    Type-Text 'FAROPZ-tecleada'
    $typed = 'FAROPZtecleada77'
    Fill-Id 'entry-dialog-password' $typed -Secret; Start-Sleep -Milliseconds 700
    Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 500
    Fill-Id $ExactId '30'; Start-Sleep -Milliseconds 700
    $t1 = (Read-Field).Value
    $sl = Find-El -Id 'entry-dialog-password-length'; Click-El $sl; Press 'RIGHT' 2; Start-Sleep -Milliseconds 700
    $t2 = (Read-Field).Value
    if ($t1 -cne $typed) { Add-Generated $t1 }; if ($t2 -cne $typed) { Add-Generated $t2 }
    Check '(2) Una contrasena TECLEADA no se pisa al tocar la casilla o el deslizante de longitud (sin pulsar Generar)' ($t1 -ceq $typed -and $t2 -ceq $typed) "tras teclear 30 en la casilla: igual = $($t1 -ceq $typed) ($($t1.Length) caracteres); tras mover el deslizante: igual = $($t2 -ceq $typed) ($($t2.Length) caracteres); casilla ahora = `"$(Get-Value (Find-El -Id $ExactId))`""
    Log "   cancelar: $(Close-Dialog-Discard)"
}
}

if ($only -eq 'a3') {
Step 'PZA4 200 contrasenas con las opciones por defecto' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Personal'
    $count0 = (Names-Like '^\d+ entradas?$') -join '|'
    Open-NewEntry -Options
    $boxes = @(Get-All | Where-Object { (Type-Of $_) -eq 'CheckBox' } | ForEach-Object { $tp = $null; $st = '?'; if ($_.TryGetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern, [ref]$tp)) { $st = "$($tp.Current.ToggleState)" }; "$($_.Current.Name) = $st" })
    $exact0 = "$(Get-Value (Find-El -Id $ExactId))"
    $defaults = ($exact0 -eq '20' -and @($boxes | Where-Object { $_ -match '^(Min|May|N.meros|S.mbolos).* = On$' }).Count -eq 4 -and @($boxes | Where-Object { $_ -like 'Evitar* = Off' }).Count -eq 1)
    Check '(3) Dialogo recien abierto: opciones por defecto (20, cuatro tipos, sin "evitar parecidos")' $defaults "casilla = $exact0; $($boxes -join ' | ')"
    Click-El (Find-El -Like 'Ocultar opciones del generador' -Type Button); Start-Sleep -Milliseconds 400
    Gen-Click; $first = Read-Field
    Log "   primera: $($first.Value.Length) caracteres; a la vista sin pulsar Mostrar = $(-not $first.WasMasked)"
    $pw = Find-El -Id 'entry-dialog-password'; $g = Mid (Find-El -Name 'Generar' -Type Button)
    $list = New-Object System.Collections.Generic.List[string]; $list.Add($first.Value)
    $prev = $first.Value; $stuck = 0; $retries = 0
    [void](Focus-App)
    for ($k = 1; $k -lt 200; $k++) {
        $v = $prev
        for ($try = 0; $try -lt 2 -and $v -ceq $prev; $try++) {
            if ($try -gt 0) { $retries++ }
            [QA.Native]::ClickAt($g[0], $g[1])
            $sw = [Diagnostics.Stopwatch]::StartNew()
            while ($sw.ElapsedMilliseconds -lt 1500) {
                Start-Sleep -Milliseconds 25
                try { $v = "$(Raw-Value $pw)" } catch { $pw = Find-El -Id 'entry-dialog-password'; $v = "$(Raw-Value $pw)" }
                if ($v -cne $prev) { break }
            }
        }
        if ($v -ceq $prev) { $stuck++ }
        $list.Add($v); $prev = $v
    }
    $uniq = @($list | Sort-Object -Unique -CaseSensitive).Count
    $hs = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal); foreach ($v in $list) { [void]$hs.Add($v) }
    $len20 = @($list | Where-Object { $_.Length -eq 20 }).Count
    $four = @($list | Where-Object { $c = Classes $_; $c[0] -gt 0 -and $c[1] -gt 0 -and $c[2] -gt 0 -and $c[3] -gt 0 }).Count
    $alpha = New-Object 'System.Collections.Generic.HashSet[char]'; foreach ($v in $list) { foreach ($ch in $v.ToCharArray()) { [void]$alpha.Add($ch) } }
    $ws = @($list | Where-Object { $_ -match '\s' }).Count
    Check '(3) 200 pulsaciones de "Generar": las 200 distintas, todas de 20 y todas con los cuatro tipos' ($list.Count -eq 200 -and $hs.Count -eq 200 -and $len20 -eq 200 -and $four -eq 200) "leidas $($list.Count); distintas $($hs.Count); de 20 caracteres $len20; con los cuatro tipos $four; caracteres distintos vistos en total $($alpha.Count); con espacios $ws; pulsaciones sin cambio $stuck (reintentos $retries); el campo sigue a la vista = $(-not (Find-El -Id 'entry-dialog-password').Current.IsPassword)"
    $all = @((Get-State 'generadas') -split "`n" | Where-Object { $_ }) + @($list | Where-Object { $_ })
    Set-State 'generadas' ((@($all | Select-Object -Unique)) -join "`n")
    Log "   guardadas en el estado para el rastro: $(@((Get-State 'generadas') -split "`n" | Where-Object { $_ }).Count) contrasenas generadas en total"
    $res = Close-Dialog-Discard
    $count1 = (Names-Like '^\d+ entradas?$') -join '|'
    Check '(3) "Cancelar" pregunta por descartar y no guarda nada' ($res -like 'pregunta = True*cerrado = True' -and $count0 -eq $count1) "$res; entradas antes/despues: $count0 / $count1"
}
}
Summary
