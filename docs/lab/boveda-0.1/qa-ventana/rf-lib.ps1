# Reverificacion final con el instalador de RELEASE 0.1.0: datos y ayudas comunes de los pasos rf*.
# Se apoya en qa-lib.ps1, b-lib.ps1 y rv-lib.ps1 sin tocarlos: solo cambia las carpetas y los canarios.
# La salida de los pasos nombra los canarios en claro: redirigirla siempre a una carpeta que la inspeccion excluye
# (la variable RF_STATE apunta a ella; ahi se guarda tambien el estado entre pasos).
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rv-lib.ps1'), [Text.Encoding]::UTF8)))
$script:Shots = Join-Path $Repo '.impeccable\review\qa2'
New-Item -ItemType Directory -Force $Shots | Out-Null
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa2-20261111'
$script:StateDir = $env:RF_STATE
if (-not $StateDir -or -not (Test-Path $StateDir)) { throw 'Falta RF_STATE: carpeta de estado y registros, fuera de lo que se inspecciona' }
# Canarios NUEVOS, distintos de los de las dos corridas anteriores (CANARIO-QA, CENTINELA-RV).
$script:RfMaster = "RF-Maestra-Final-27-$([char]0x00F1)$([char]0x20AC)"
$script:RfOther = 'RF-Repeticion-Otra-94'
$script:Rf = @{
    Title = 'TESTIGO-RF-Titulo'; User = 'TESTIGO-RF-usuario.largo.de.prueba@example.net'
    Url = 'https://example.net/TESTIGO-RF-url/ruta/larga/de/prueba'; Notes = 'TESTIGO-RF-notas reservadas'
    Tag = 'TESTIGO-RF-etiqueta'; Group = 'TESTIGO-RF-Grupo'; Search = 'TESTIGO-RF-busqueda'; Partial = 'testigo-rf-tit'
    Pass = 'TESTIGO-RF-Clave-58'; Title2 = 'TESTIGO-RF-Generada'
}
$script:RfVault = Join-Path $Work 'rf-uno.kdbx'
$script:Vault = $RfVault
$script:Master = $RfMaster
$script:EbDir = Join-Path $DataDir 'EBWebView'
$script:RootName = 'rf-uno'

function Set-State([string]$key, [string]$value) { [IO.File]::WriteAllText((Join-Path $StateDir "$key.txt"), $value, [Text.Encoding]::UTF8) }
function Get-State([string]$key) {
    $p = Join-Path $StateDir "$key.txt"
    if (Test-Path $p) { return [IO.File]::ReadAllText($p, [Text.Encoding]::UTF8) }
    return ''
}
function Rf-Secrets {
    $plain = @{ 'titulo' = $Rf.Title; 'usuario' = $Rf.User; 'URL' = $Rf.Url; 'notas' = $Rf.Notes; 'etiqueta' = $Rf.Tag; 'grupo' = $Rf.Group
        'busqueda' = $Rf.Search; 'busqueda parcial' = $Rf.Partial; 'contrasena de la entrada' = $Rf.Pass; 'prefijo TESTIGO-RF' = 'TESTIGO-RF' }
    $gen = Get-State 'generada'
    if ($gen) { $plain['contrasena generada'] = $gen }
    New-Secrets $plain @{ 'maestra' = $RfMaster; 'repeticion distinta' = $RfOther }
}
function Start-Epoch { [string][DateTimeOffset]::new([datetime](Get-State 'inicio')).ToUnixTimeSeconds() }

# ---------- Medidas ----------
# Arranque: del lanzamiento a que "Bienvenida" se lee por UIA, con un solo cronometro. Antes se hace una consulta
# UIA cualquiera para no medir el primer enlace de UIA de este proceso.
function Start-AppTimed([string]$label) {
    [void]$AE::RootElement.FindAll($TreeScope::Children, [System.Windows.Automation.Condition]::TrueCondition).Count
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-App
    $ms = Wait-Text 'Bienvenida' 40
    $total = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO arranque [$label]: ventana a los $LastStartMs ms; 'Bienvenida' legible a los $total ms (sondeo $ms ms)"
    return $total
}
function Read-Count { (Get-Names | Where-Object { $_ -match '^\d+:\d\d$' } | Select-Object -First 1) }
function Count-Secs([string]$text) { if ($text -match '^(\d+):(\d\d)$') { return [int]$Matches[1] * 60 + [int]$Matches[2] }; return -1 }
function Pointer-Away {
    $i = Get-WinInfo
    $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    [void][QA.Native]::SetCursorPos([math]::Min($screen.Width - 5, $i.X + $i.OuterW + 200), [math]::Min($screen.Height - 60, $i.Y + $i.OuterH + 60))
}
# Pulsa "Guardar entrada" y mide. D-4: en el instante en que se lee "Entrada guardada." se RELEE la fecha del archivo;
# si todavia es la de antes, el aviso se adelanto al disco.
function Save-EntryTimed {
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    $btn = Find-El -Name 'Guardar entrada' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $tMsg = -1; $tDisk = -1; $tSaved = -1; $diskAtMsg = $null; $interim = New-Object System.Collections.Generic.List[string]
    while ($sw.Elapsed.TotalSeconds -lt 25 -and ($tMsg -lt 0 -or $tSaved -lt 0 -or $tDisk -lt 0)) {
        $names = @(Get-Names)
        if ($tDisk -lt 0 -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $tDisk = [int]$sw.ElapsedMilliseconds }
        foreach ($n in $names) {
            if (($n -like '*uard*' -or $n -like 'Entrada creada*' -or $n -like 'Cambios aplicados*') -and $n -notlike 'Guardar*' -and -not $interim.Contains("$n")) { $interim.Add("$n") }
        }
        if ($tMsg -lt 0 -and $names -contains 'Entrada guardada.') {
            $tMsg = [int]$sw.ElapsedMilliseconds
            $diskAtMsg = ((Get-Item $Vault).LastWriteTimeUtc -ne $stamp0)
            if ($diskAtMsg -and $tDisk -lt 0) { $tDisk = $tMsg }
        }
        if ($tSaved -lt 0 -and $tDisk -ge 0 -and $names -contains 'Guardado.') { $tSaved = [int]$sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 15
    }
    Log "TIEMPO guardar entrada: 'Entrada guardada.' a los $tMsg ms; archivo cambiado en disco a los $tDisk ms; 'Guardado.' a los $tSaved ms; al leer el aviso el archivo ya habia cambiado = $diskAtMsg"
    Log "   avisos vistos, en orden: $($interim -join ' -> ')"
    return [pscustomobject]@{ Msg = $tMsg; Disk = $tDisk; Saved = $tSaved; DiskAtMsg = $diskAtMsg; Seen = @($interim) }
}
function Unlock-Timed([string]$label) {
    Fill-Id 'unlock-password' $RfMaster -Secret
    $btn = Find-El -Name 'Desbloquear' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 30 -Quiet)
    $ms = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO desbloquear [$label]: $ms ms (clic -> boveda abierta = $ok)"
    return $ms
}
function Lock-Timed {
    $btn = Find-El -Name 'Bloquear' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $ok = $null -ne (Find-El -Name 'Desbloquear' -Type Button -Timeout 10 -Quiet)
    $ms = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO bloquear: $ms ms (clic -> pantalla de desbloqueo = $ok)"
    Start-Sleep -Milliseconds 1200
    return $ms
}
function Set-AutoLock([string]$optionLike, [string]$clipSeconds = '') {
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $ok = Select-Option 'settings-auto-lock' $optionLike
    if ($clipSeconds) { Fill-Id 'settings-clipboard' $clipSeconds }
    Click-Name -Name 'Guardar ajustes'
    $saved = (Wait-Text 'Ajustes guardados.' 5) -ge 0
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    return ($ok -and $saved)
}

# ---------- Superficie de navegador ----------
function Keys([byte[]]$vks) {
    foreach ($k in $vks) { [QA.Native]::keybd_event($k, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 25 }
    for ($k = $vks.Count - 1; $k -ge 0; $k--) { [QA.Native]::keybd_event($vks[$k], 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 25 }
}
# Ventanas de primer nivel del escritorio: manejador -> descripcion (clase y proceso; el titulo solo si es de la app).
function Top-Wins {
    $list = @{}
    $pids = @(Get-AppPids)
    foreach ($w in $AE::RootElement.FindAll($TreeScope::Children, [System.Windows.Automation.Condition]::TrueCondition)) {
        try {
            $c = $w.Current
            $own = $pids -contains [int]$c.ProcessId
            $proc = (Get-Process -Id $c.ProcessId -ErrorAction SilentlyContinue).Name
            $title = if ($own) { $c.Name } else { '<ajena>' }
            $list[[string]$c.NativeWindowHandle] = ("[{0}] `"{1}`" clase={2} proceso={3} de-la-app={4}" -f (Type-Of $w), $title, $c.ClassName, $proc, $own)
        } catch { }
    }
    return $list
}
function Shot-Area([string]$name) {
    $h = Get-AppHwnd
    $r = [QA.Native]::VisibleRect($h)
    $l = $r.Left; $t = $r.Top; $rr = $r.Right; $b = $r.Bottom
    $fg = [QA.Native]::GetForegroundWindow()
    if ($fg -ne $h -and $fg -ne [IntPtr]::Zero) {
        [uint32]$pidFg = 0; [void][QA.Native]::GetWindowThreadProcessId($fg, [ref]$pidFg)
        if ((Get-AppPids) -contains [int]$pidFg) {
            $r2 = [QA.Native]::VisibleRect($fg)
            $l = [math]::Min($l, $r2.Left); $t = [math]::Min($t, $r2.Top); $rr = [math]::Max($rr, $r2.Right); $b = [math]::Max($b, $r2.Bottom)
        }
    }
    $l = [math]::Max(0, $l); $t = [math]::Max(0, $t)
    $w = $rr - $l; $hh = $b - $t
    if ($w -le 0 -or $hh -le 0) { return }
    $bmp = New-Object System.Drawing.Bitmap($w, $hh)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($l, $t, 0, 0, $bmp.Size); $g.Dispose()
    $bmp.Save((Join-Path $Shots "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Log "   captura: $name.png ($w x $hh)"
}
# Envia una tecla y anota lo que cambio: ventanas nuevas, nombres nuevos o perdidos en la ventana, foco y primer plano.
function Probe-Key([string]$label, [scriptblock]$send, [string]$shot = '') {
    [void](Focus-App)
    $w0 = Top-Wins; $n0 = @(Get-Names); $f0 = Focused-Desc
    & $send
    Start-Sleep -Milliseconds 1600
    $w1 = Top-Wins
    $newKeys = @($w1.Keys | Where-Object { -not $w0.ContainsKey($_) })
    $n1 = @(); try { $n1 = @(Get-Names) } catch { }
    $added = @($n1 | Where-Object { $n0 -notcontains $_ } | Select-Object -Unique -First 14)
    $removed = @($n0 | Where-Object { $n1 -notcontains $_ } | Select-Object -Unique -First 14)
    $fg = [QA.Native]::GetForegroundWindow()
    $fgIsApp = ($fg -eq (Get-AppHwnd))
    Log "TECLA [$label]: ventanas nuevas=$($newKeys.Count) $(($newKeys | ForEach-Object { $w1[$_] }) -join ' ; ')"
    Log "   nombres nuevos ($($added.Count)): $($added -join ' | ')"
    Log "   nombres que se fueron ($($removed.Count)): $($removed -join ' | ')"
    Log "   foco antes: $f0 -> despues: $(Focused-Desc); primer plano es la app = $fgIsApp (clase $([QA.Native]::ClassOf($fg))); proceso vivo = $((Get-AppProc).Count -eq 1)"
    if ($shot) { Shot-Area $shot }
    return [pscustomobject]@{ NewHandles = $newKeys; NewDesc = @($newKeys | ForEach-Object { $w1[$_] }); Added = $added; Removed = $removed; FgIsApp = $fgIsApp }
}
# Cierra lo que una tecla haya abierto: Escape y, si una ventana nueva sigue ahi, cierre normal de esa ventana.
function Dismiss($probe) {
    if ($probe.NewHandles.Count -eq 0 -and $probe.FgIsApp) { return }
    Press 'ESC'; Start-Sleep -Milliseconds 700
    $now = Top-Wins
    foreach ($k in $probe.NewHandles) {
        if ($now.ContainsKey($k)) {
            Log "   sigue abierta tras Escape: $($now[$k]); se le manda el cierre normal"
            [void][QA.Native]::PostMessage([IntPtr][long]$k, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero); Start-Sleep -Milliseconds 700
        }
    }
    $left = @((Top-Wins).Keys | Where-Object { $probe.NewHandles -contains $_ })
    Log "   tras cerrar: ventanas nuevas que quedan = $($left.Count)"
}
# Clic derecho en un punto y lectura del menu que aparezca (elementos de menu bajo el puntero o en ventanas nuevas).
function Probe-RightClick([string]$label, [int]$x, [int]$y, [string]$shot = '') {
    [void](Focus-App)
    $w0 = Top-Wins
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 120
    [QA.Native]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40
    [QA.Native]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 1200
    $w1 = Top-Wins
    $newKeys = @($w1.Keys | Where-Object { -not $w0.ContainsKey($_) })
    $miCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuItem)
    $items = New-Object System.Collections.Generic.List[string]
    $under = ''
    try {
        $el = $AE::FromPoint((New-Object System.Windows.Point(($x + 14), ($y + 14))))
        $under = "[$(Type-Of $el)] `"$($el.Current.Name)`" clase=$($el.Current.ClassName)"
        $cur = $el
        for ($k = 0; $k -lt 6 -and $null -ne $cur; $k++) {
            if ((Type-Of $cur) -in @('Menu', 'MenuBar', 'Window', 'Pane') -and @($cur.FindAll($TreeScope::Descendants, $miCond)).Count -gt 0) {
                foreach ($m in $cur.FindAll($TreeScope::Descendants, $miCond)) { $items.Add($m.Current.Name) }
                break
            }
            $cur = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($cur)
        }
    } catch { }
    foreach ($k in $newKeys) {
        try { foreach ($m in $AE::FromHandle([IntPtr][long]$k).FindAll($TreeScope::Descendants, $miCond)) { if (-not $items.Contains($m.Current.Name)) { $items.Add($m.Current.Name) } } } catch { }
    }
    try { foreach ($m in (Get-Root).FindAll($TreeScope::Descendants, $miCond)) { if (-not $items.Contains($m.Current.Name)) { $items.Add($m.Current.Name) } } } catch { }
    Log "CLIC DERECHO [$label]: ventanas nuevas=$($newKeys.Count) $(($newKeys | ForEach-Object { $w1[$_] }) -join ' ; ')"
    Log "   bajo el puntero (+14,+14): $under"
    Log "   elementos de menu ($($items.Count)): $($items -join ' | ')"
    if ($shot) { Shot-Area $shot }
    Press 'ESC'; Start-Sleep -Milliseconds 500
    return [pscustomobject]@{ Items = @($items); NewHandles = $newKeys }
}

# ---------- Estado ----------
# Deja la boveda de prueba abierta venga de donde venga: app cerrada, bienvenida o pantalla de desbloqueo.
function Ensure-VaultOpen {
    if ((Get-AppProc).Count -eq 0) { Start-App; [void](Wait-Text 'Bienvenida' 40) }
    [void](Focus-App)
    if (Has-El -Name 'Cancelar' -Type Button -Timeout 0.5) { Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 400 }
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 1) { return }
    if (Has-El -Id 'welcome-title' -Timeout 1) { Click-Name -Name 'Abrir rf-uno.kdbx'; [void](Find-El -Id 'unlock-title' -Timeout 5); Start-Sleep -Milliseconds 400 }
    if (Has-El -Id 'unlock-password' -Timeout 3) { [void](Unlock-Timed 'preparacion') }
    if (-not (Has-El -Name 'Bloquear' -Type Button -Timeout 3)) { throw 'No se pudo dejar la boveda abierta' }
}
function Screen-Now {
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.4) { return 'boveda abierta' }
    if (Has-El -Id 'unlock-password' -Timeout 0.4) { return 'pantalla de desbloqueo' }
    if (Has-El -Id 'welcome-title' -Timeout 0.4) { return 'bienvenida' }
    return 'otra'
}
