# Verificacion de la capa nativa aislada (R-1) y del valor mostrado no seleccionable (R-4): variante de rf-lib.ps1
# (RF_VARIANT=vn-lib). Carga vc-lib.ps1 (detector de la superficie de navegador) y pone encima canarios y carpetas
# nuevos, el binario a probar (VN_EXE=aislada usa el ejecutable de prueba, sin instalar) y las ayudas del
# portapapeles: copiar por el navegador, historial de Windows (Win+V) por WinRT y arrastre hacia una ventana propia.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'vc-lib.ps1'), [Text.Encoding]::UTF8)))
$script:Shots = Join-Path $Repo '.impeccable\review\qa4'
New-Item -ItemType Directory -Force $Shots | Out-Null
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa4-20261112'
$script:RfMaster = "VN-Maestra-Nativa-52-$([char]0x00F1)$([char]0x20AC)"
$script:RfOther = 'VN-Repeticion-Otra-08'
# La contrasena canario no lleva separadores: asi el doble clic la selecciona entera.
$script:Rf = @{
    Title = 'CENTELLA-VN-Titulo'; User = 'CENTELLA-VN-usuario@example.com'; Url = 'https://example.com/CENTELLA-VN-url'
    Notes = 'CENTELLA-VN-notas reservadas'; Tag = 'CENTELLA-VN-etiqueta'; Group = 'CENTELLA-VN-Grupo'; Search = 'CENTELLA-VN-busqueda'
    Partial = 'centella-vn-tit'; Pass = 'CENTELLAVNclave4821x'; Title2 = 'CENTELLA-VN-Pegada'; Pass2 = 'CENTELLAVNotra7730y'
}
$script:RfVault = Join-Path $Work 'rf-uno.kdbx'
$script:Vault = $RfVault
$script:Master = $RfMaster
function Rf-Secrets {
    $plain = @{ 'titulo' = $Rf.Title; 'usuario' = $Rf.User; 'URL' = $Rf.Url; 'notas' = $Rf.Notes; 'etiqueta' = $Rf.Tag; 'grupo' = $Rf.Group
        'contrasena de la entrada' = $Rf.Pass; 'prefijo CENTELLA-VN' = 'CENTELLA-VN'; 'prefijo CENTELLAVN' = 'CENTELLAVN'; 'prefijo de la corrida anterior' = 'VIGIA-VC' }
    New-Secrets $plain @{ 'maestra' = $RfMaster; 'repeticion distinta' = $RfOther }
}

# ---------- Que binario se prueba ----------
$script:InstalledExe = $AppExe
$script:IsolatedExe = Join-Path $Repo 'target\qa\almarioks-solo-capa-nativa.exe'
if ($env:VN_EXE -eq 'aislada') { $script:AppExe = $IsolatedExe; $script:InstallDir = Split-Path $IsolatedExe }
function Get-AppProc {
    @(Get-Process ([IO.Path]::GetFileNameWithoutExtension($AppExe)) -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -ieq $AppExe })
}
function Any-AlmarioProc { @(Get-Process 'almarioks*' -ErrorAction SilentlyContinue) }
# Lanza el binario elegido solo si no hay ningun almarioks* vivo (comparten identificador: la instancia unica
# devolveria la ventana del otro) y comprueba que la ventana que se maneja es de ESE binario.
function Start-Checked([string]$label) {
    $alive = Any-AlmarioProc
    if ($alive.Count) { throw "Hay procesos almarioks* vivos antes de lanzar: $(($alive | ForEach-Object { $_.Name }) -join ', ')" }
    [void]$AE::RootElement.FindAll($TreeScope::Children, [System.Windows.Automation.Condition]::TrueCondition).Count
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-App 40
    $ms = Wait-Text 'Bienvenida' 40
    $h = Get-AppHwnd
    if ($h -eq [IntPtr]::Zero -or $ms -lt 0) { throw "El binario no llego a la bienvenida ($label)" }
    [uint32]$owner = 0; [void][QA.Native]::GetWindowThreadProcessId($h, [ref]$owner)
    $path = (Get-Process -Id $owner).Path
    $names = (Any-AlmarioProc | ForEach-Object { $_.Name } | Select-Object -Unique) -join ', '
    Log "ARRANQUE [$label]: 'Bienvenida' a los $([int]$sw.ElapsedMilliseconds) ms; la ventana es de $(Split-Path $path -Leaf) (el binario pedido = $($path -ieq $AppExe)); procesos almarioks*: $names"
    if ($path -ine $AppExe) { throw 'La ventana no es del binario pedido' }
}

# ---------- Historial del portapapeles (Win+V) por WinRT ----------
# Solo se cuentan y se borran los elementos que contienen textos de esta prueba; el texto de los demas no se registra.
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.ApplicationModel.DataTransfer.Clipboard, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
$null = [Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
$null = [Windows.ApplicationModel.DataTransfer.StandardDataFormats, Windows.ApplicationModel.DataTransfer, ContentType = WindowsRuntime]
$script:AsTaskOp = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' } | Select-Object -First 1
function Await-Op($op, [Type]$type) {
    $task = $AsTaskOp.MakeGenericMethod($type).Invoke($null, @($op))
    [void]$task.Wait(8000)
    return $task.Result
}
function History-Items {
    $res = Await-Op ([Windows.ApplicationModel.DataTransfer.Clipboard]::GetHistoryItemsAsync()) ([Windows.ApplicationModel.DataTransfer.ClipboardHistoryItemsResult])
    $list = @()
    if ("$($res.Status)" -eq 'Success') {
        foreach ($it in $res.Items) {
            $t = ''
            try { if ($it.Content.Contains([Windows.ApplicationModel.DataTransfer.StandardDataFormats]::Text)) { $t = Await-Op ($it.Content.GetTextAsync()) ([string]) } } catch { }
            $list += [pscustomobject]@{ Text = "$t"; Item = $it }
        }
    }
    return [pscustomobject]@{ Status = "$($res.Status)"; Items = $list }
}
# 'si', 'no' o 'sin acceso (<estado>)'.
function History-Has([string]$needle) {
    try {
        $h = History-Items
        if ($h.Status -ne 'Success') { return "sin acceso ($($h.Status))" }
        if (@($h.Items | Where-Object { $_.Text.Contains($needle) }).Count -gt 0) { return 'si' }
        return 'no'
    } catch { return "sin acceso ($($_.Exception.Message))" }
}
function History-Purge([string[]]$needles) {
    $n = 0
    try {
        $h = History-Items
        foreach ($e in $h.Items) {
            foreach ($needle in $needles) {
                if ($e.Text.Contains($needle)) { if ([Windows.ApplicationModel.DataTransfer.Clipboard]::DeleteItemFromHistory($e.Item)) { $n++ }; break }
            }
        }
    } catch { Log "   historial: no se pudo purgar ($($_.Exception.Message))" }
    return $n
}
# Lo que esta prueba pudo dejar en el historial: canarios, centinelas y la ruta de su carpeta (Ctrl+A la copia).
function Purge-Mine { History-Purge @('CENTELLA', 'centinela-vn', 'almarioks-qa4', 'rf-uno') }

# ---------- Copiar por el navegador ----------
$script:SentinelN = 0
$ExclFormats = @('ExcludeClipboardContentFromMonitorProcessing', 'CanIncludeInClipboardHistory', 'CanUploadToCloudClipboard')
# Pone un centinela en el portapapeles, hace el gesto de seleccion, pulsa Ctrl+C y mira que quedo. Solo registra
# longitudes y si/no: con Ctrl+A el portapapeles lleva tambien la ruta de la boveda.
function Try-Copy([string]$stage, [string]$gesture, [scriptblock]$select, [string]$needle, [string]$shot = '') {
    $script:SentinelN++
    $sent = "centinela-vn-$SentinelN"
    Set-ClipExternal $sent; Start-Sleep -Milliseconds 300
    & $select
    Start-Sleep -Milliseconds 350
    if ($shot) { Shot-Area $shot }
    Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $clip = "$(Get-Clip)"; $formats = @(Get-ClipFormats)
    # Tambien cuenta un trozo: en vn1 el arrastre de punta a punta copio 19 de los 20 caracteres y la comparacion
    # exacta lo dio por limpio. Un trozo = 8 caracteres seguidos del centro del valor.
    $mid = if ($needle.Length -ge 14) { $needle.Substring(4, 8) } else { $needle }
    $whole = $clip.Contains($needle)
    $has = $whole -or $clip.Contains($mid)
    $exclNames = @($ExclFormats | Where-Object { $formats -contains $_ })
    $excl = $exclNames.Count
    Start-Sleep -Milliseconds 1800
    $hist = History-Has $(if ($whole) { $needle } else { $mid })
    $what = if ($whole -and $clip -ceq $needle) { 'EXACTAMENTE el valor' } elseif ($whole) { "el valor, dentro de $($clip.Length) caracteres" } elseif ($has) { "un TROZO del valor, dentro de $($clip.Length) caracteres" } elseif ($clip -ceq $sent) { 'sigue el centinela: no se copio nada' } else { "otro texto de $($clip.Length) caracteres, sin el valor" }
    $res = "portapapeles: $what; formatos de exclusion: $excl de 3 ($($exclNames -join ', ')); en el historial: $hist"
    Log "COPIA [$stage] $gesture -> $res"
    Row $stage "$gesture + Ctrl+C" $(if ($has) { "SENAL: $res" } else { $res })
    return [pscustomobject]@{ Has = $has; Excl = $excl; Hist = $hist; Clip = $clip }
}
function Pass-Rect {
    [void](Show-Pass)
    $el = Find-El -Name $Rf.Pass -Type Text -Timeout 1.5 -Quiet
    if ($null -eq $el) { throw 'El valor mostrado no esta en el arbol de la ventana' }
    return $el.Current.BoundingRectangle
}
function Mouse-Drag([int]$x0, [int]$y0, [int]$x1, [int]$y1, [int]$steps = 14) {
    [void][QA.Native]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 150
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150
    for ($k = 1; $k -le $steps; $k++) {
        [void][QA.Native]::SetCursorPos([int]($x0 + ($x1 - $x0) * $k / $steps), [int]($y0 + ($y1 - $y0) * $k / $steps))
        [QA.Native]::mouse_event(0x0001, 0, 0, 0, [UIntPtr]::Zero)
        [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30
    }
    for ($k = 0; $k -lt 10; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30 }
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    for ($k = 0; $k -lt 15; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30 }
}
# Los gestos de seleccion sobre el valor mostrado (los mismos en la version sin arreglo y en la arreglada).
$script:SelDouble = { $r = Pass-Rect; $x = [int]($r.X + $r.Width / 2); $y = [int]($r.Y + $r.Height / 2); [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($x, $y) }
$script:SelTriple = { $r = Pass-Rect; $x = [int]($r.X + $r.Width / 2); $y = [int]($r.Y + $r.Height / 2); [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($x, $y) }
$script:SelDrag = { $r = Pass-Rect; $y = [int]($r.Y + $r.Height / 2); Mouse-Drag ([int]($r.X + 1)) $y ([int]($r.X + $r.Width - 1)) $y }
# Arrastre que empieza fuera del valor (en el rotulo de arriba, "Usuario") y termina debajo de el: lo cruza entero.
$script:SelAcross = {
    $r = Pass-Rect
    $u = (Find-El -Name $Rf.User -Type Text -Timeout 2).Current.BoundingRectangle
    $n = (Find-El -Name $Rf.Notes -Type Text -Timeout 2).Current.BoundingRectangle
    Mouse-Drag ([int]($u.X + 2)) ([int]($u.Y + $u.Height / 2)) ([int]($n.X + $n.Width - 2)) ([int]($n.Y + $n.Height / 2)) 24
}
$script:SelAll = {
    [void](Pass-Rect)
    $n = (Find-El -Name $Rf.Notes -Type Text -Timeout 2).Current.BoundingRectangle
    [QA.Native]::ClickAt([int]($n.X + $n.Width + 60), [int]($n.Y + $n.Height / 2)); Start-Sleep -Milliseconds 150
    Press-Ctrl 'a'
}
# Los cinco gestos con Ctrl+C. Devuelve cuantos dejaron el valor en el portapapeles.
function Copy-Gestures([string]$stage, [string]$shotPrefix) {
    $hits = 0
    foreach ($g in @(@('doble clic sobre el valor', $SelDouble, 'doble-clic'), @('triple clic sobre el valor', $SelTriple, 'triple-clic'), @('arrastre de punta a punta del valor', $SelDrag, 'arrastre'),
            @('arrastre que empieza fuera y cruza el valor', $SelAcross, 'arrastre-cruzado'), @('Ctrl+A en la pagina', $SelAll, 'ctrl-a'))) {
        try { $r = Try-Copy $stage $g[0] $g[1] $Rf.Pass "$shotPrefix-$($g[2])"; if ($r.Has) { $hits++ } }
        catch { Row $stage "$($g[0]) + Ctrl+C" "NO PROBADO: $($_.Exception.Message)" }
        [QA.Native]::ClickAt(([int]((Find-El -Name $Rf.Notes -Type Text -Timeout 2).Current.BoundingRectangle.Right) + 60), ([int]((Find-El -Name $Rf.Notes -Type Text -Timeout 2).Current.BoundingRectangle.Y) + 8)); Start-Sleep -Milliseconds 200
    }
    return $hits
}

# Otro sitio donde un secreto es texto: la candidata del generador (todavia no guardada). Triple clic + Ctrl+C.
function Generator-Copy([string]$stage) {
    Open-Entry $RootName $Rf.Title
    Click-Name -Name 'Editar'; [void](Wait-Text 'Editar *' 4)
    Click-Name -Name 'Generar contraseña'; [void](Wait-Text 'Generador de contraseñas' 5)
    $before = @(Get-Names)
    Click-Name -Name 'Generar'; $use = Find-El -Name 'Usar esta contraseña' -Type Button -Timeout 8 -Quiet; Start-Sleep -Milliseconds 400
    $cand = @()
    if ($null -ne $use) { $cand = @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name.Length -ge 12 -and $_.Current.Name -notmatch '\s' -and $before -notcontains $_.Current.Name } | Select-Object -First 1) }
    if ($cand.Count -eq 0) { Row $stage 'candidata del generador: triple clic + Ctrl+C' 'NO PROBADO: la candidata no aparece como texto en el arbol' }
    else {
        $value = $cand[0].Current.Name; $c = Center-Of $cand[0]
        Set-ClipExternal 'centinela-vn-generador'; Start-Sleep -Milliseconds 300
        [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 350
        Shot-Area ("VN-generador-" + ($stage -replace '[^A-Za-z0-9]+', '-'))
        Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
        $clip = "$(Get-Clip)".Trim()
        $whole = $clip.Contains($value); $part = (-not $whole) -and $clip.Length -ge 4 -and $value.Contains($clip)
        $res = if ($whole) { "SENAL: el portapapeles lleva la candidata entera ($($value.Length) caracteres)" } elseif ($part) { "SENAL: el portapapeles lleva un trozo de la candidata ($($clip.Length) de $($value.Length) caracteres)" } elseif ($clip -ceq 'centinela-vn-generador') { 'sigue el centinela: no se copio nada' } else { "otro texto de $($clip.Length) caracteres, sin la candidata" }
        Row $stage 'candidata del generador: triple clic + Ctrl+C' $res
        [void](History-Purge @($value))
    }
    for ($k = 0; $k -lt 3 -and (Has-El -Name 'Guardar entrada' -Type Button -Timeout 0.4); $k++) { Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 400 }
}

# ---------- Arrastrar hacia fuera: una ventana propia que acepta texto ----------
function New-DropBox {
    $script:Dropped = $null
    $f = New-Object System.Windows.Forms.Form
    $f.Text = 'destino-qa'; $f.TopMost = $true; $f.StartPosition = 'Manual'; $f.ShowInTaskbar = $false
    $wa = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
    $f.Size = New-Object System.Drawing.Size(230, 220); $f.Location = New-Object System.Drawing.Point(($wa.Right - 240), 300)
    $tb = New-Object System.Windows.Forms.TextBox
    $tb.Multiline = $true; $tb.Dock = 'Fill'; $tb.AllowDrop = $true
    $tb.add_DragEnter({ param($s, $e) $e.Effect = [System.Windows.Forms.DragDropEffects]::Copy })
    $tb.add_DragOver({ param($s, $e) $e.Effect = [System.Windows.Forms.DragDropEffects]::Copy })
    $tb.add_DragDrop({ param($s, $e) $script:Dropped = "$($e.Data.GetData([System.Windows.Forms.DataFormats]::UnicodeText))" })
    $f.Controls.Add($tb); $f.Show()
    for ($k = 0; $k -lt 10; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30 }
    return $f
}
# Selecciona con doble clic el texto en (x, y), lo arrastra hasta la ventana propia y devuelve lo que llego.
function Drag-Out([int]$x, [int]$y, $box) {
    $script:Dropped = $null
    [void](Focus-App)
    [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 400
    $b = $box.Bounds
    Mouse-Drag $x $y ([int]($b.X + $b.Width / 2)) ([int]($b.Y + $b.Height / 2 + 20)) 30
    for ($k = 0; $k -lt 15; $k++) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 30 }
    return $Dropped
}
