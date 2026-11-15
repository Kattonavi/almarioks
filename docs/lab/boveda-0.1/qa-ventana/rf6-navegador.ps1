# Reverificacion final (6): superficie de navegador de WebView2 en release (I-7 de la revision de seguridad) y panel
# de actualizacion. Con una entrada abierta: clic derecho, teclas de navegador (buscar, herramientas, ver codigo,
# imprimir, guardar, zoom, recargar). Nunca se confirma una impresion; si "Guardar como" se abre, se guarda solo dentro
# de la carpeta de trabajo y se inspecciona lo guardado.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$script:Findings = New-Object System.Collections.Generic.List[string]
function Note([string]$text) { $Findings.Add($text); Log "HALLAZGO: $text" }
function Probe-And-Note([string]$label, [byte[]]$vks, [string]$shot = '') {
    $p = Probe-Key $label { Keys $vks } $shot
    $changed = ($p.NewHandles.Count -gt 0) -or (-not $p.FgIsApp) -or ($p.Added.Count -gt 0)
    if ($changed) { Note "$label -> ventanas nuevas: $($p.NewDesc -join ' ; ') | nombres nuevos: $($p.Added -join ' | ')" } else { Log "SIN EFECTO VISIBLE: $label" }
    return $p
}
function Probe-Reload([string]$label, [byte[]]$vks) {
    Ensure-VaultOpen
    Fill (Find-El -Id 'vault-search') 'zz-recarga'
    Start-Sleep -Milliseconds 600
    $p = Probe-Key $label { Keys $vks }
    Start-Sleep -Milliseconds 1200
    $search = Find-El -Id 'vault-search' -Timeout 2 -Quiet
    $val = if ($search) { Get-Value $search } else { '<el campo no existe>' }
    $screen = Screen-Now
    $reloaded = ($val -ne 'zz-recarga')
    Log "RECARGA [$label]: campo de busqueda tras la tecla = '$val'; pantalla = $screen; se recargo = $reloaded"
    if ($reloaded) { Note "$label recarga la pagina: la pantalla queda en '$screen'" }
    if ($search -and -not $reloaded) { Click-El $search; Press 'ESC'; Start-Sleep -Milliseconds 300 }
    return $reloaded
}
function Handle-SaveDialog($probe) {
    foreach ($k in $probe.NewHandles) {
        $h = [IntPtr][long]$k
        if ([QA.Native]::ClassOf($h) -ne '#32770') { continue }
        $dlg = $AE::FromHandle($h)
        Log "   dialogo nativo abierto: `"$($dlg.Current.Name)`""
        $edit = Find-El -Id '1001' -Type 'Edit' -Root $dlg -Timeout 2.5 -Quiet
        if ($null -eq $edit) { Log '   sin campo de nombre: no se guarda'; return $false }
        $dir = Join-Path $Work 'guardado-ctrl-s'
        New-Item -ItemType Directory -Force $dir | Out-Null
        $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue((Join-Path $dir 'pagina-guardada.html'))
        [void][QA.Native]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300
        Press 'ENTER'; Start-Sleep -Seconds 4
        $files = @(Get-ChildItem $dir -Recurse -File -Force)
        Log "   guardado en la carpeta de trabajo: $($files.Count) archivos, $(($files | Measure-Object Length -Sum).Sum) bytes: $(($files | Select-Object -First 8 | ForEach-Object { $_.Name }) -join ', ')"
        $lines = Run-Inspector (Rf-Secrets) @("guardado=$dir")
        Note "Ctrl+S abre 'Guardar como' y guarda la pagina: $($files.Count) archivos; apariciones de canarios en lo guardado = $(Hits-Of $lines 'guardado')"
        Remove-Item -LiteralPath $dir -Recurse -Force
        return $true
    }
    return $false
}
Step 'RF22 Boveda abierta con una entrada a la vista' {
    Ensure-VaultOpen
    [void](Set-WinClient 1120 720)
    Open-Entry $RootName $Rf.Title
    Check 'Entrada abierta: los canarios estan en pantalla' (((Get-Names) -contains $Rf.Notes) -and ((Get-Names) -contains $Rf.User))
}
Step 'RF23 Clic derecho' -Continue {
    $note = Find-El -Name $Rf.Notes -Type Text -Timeout 3
    $r = $note.Current.BoundingRectangle
    $x = [int]($r.X + 40); $y = [int]($r.Y + $r.Height / 2)
    $a = Probe-RightClick 'sobre el texto de las notas' $x $y 'RF20-clic-derecho-texto'
    [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 400
    $b = Probe-RightClick 'sobre una palabra seleccionada de las notas' $x $y 'RF21-clic-derecho-seleccion'
    $s = (Find-El -Id 'vault-search').Current.BoundingRectangle
    $c = Probe-RightClick 'sobre el campo de busqueda' ([int]($s.X + $s.Width / 2)) ([int]($s.Y + $s.Height / 2)) 'RF22-clic-derecho-campo'
    $i = Get-WinInfo
    $d = Probe-RightClick 'sobre el fondo de la barra de grupos' ($i.X + 120) ($i.Y + $i.OuterH - 160) 'RF23-clic-derecho-fondo'
    $any = @($a, $b, $c, $d | Where-Object { $_.Items.Count -gt 0 -or $_.NewHandles.Count -gt 0 })
    if ($any.Count) { Note "El clic derecho abre un menu contextual: texto [$($a.Items -join ' | ')] ; seleccion [$($b.Items -join ' | ')] ; campo [$($c.Items -join ' | ')] ; fondo [$($d.Items -join ' | ')]" } else { Log 'SIN EFECTO VISIBLE: clic derecho en texto, seleccion, campo y fondo' }
    Check 'El clic derecho no ofrece menu de navegador (Inspeccionar, Recargar, Guardar como, Imprimir)' ($any.Count -eq 0) "menus: $($any.Count)"
    Press 'ESC'
}
Step 'RF24 Teclas de navegador con la boveda abierta' -Continue {
    $p = Probe-Key 'Ctrl+F' { Keys @(0x11, 0x46) } 'RF24-ctrl-f'
    $onSearch = (Focused-Desc) -like '*vault-search*'
    Check 'Ctrl+F lleva a la busqueda de la app y no abre la del navegador' ($onSearch -and $p.NewHandles.Count -eq 0 -and $p.Added.Count -eq 0) (Focused-Desc)
    if (-not $onSearch -or $p.NewHandles.Count -gt 0) { Note "Ctrl+F: foco en $(Focused-Desc); ventanas nuevas $($p.NewDesc -join ' ; '); nombres nuevos $($p.Added -join ' | ')" }
    Dismiss $p
    Open-Entry $RootName $Rf.Title
    foreach ($k in @(
            @('F3', @(0x72)), @('Ctrl+G', @(0x11, 0x47)), @('F12', @(0x7B)), @('Ctrl+Shift+I', @(0x11, 0x10, 0x49)),
            @('Ctrl+Shift+J', @(0x11, 0x10, 0x4A)), @('Ctrl+Shift+C', @(0x11, 0x10, 0x43)), @('Ctrl+U', @(0x11, 0x55)),
            @('Ctrl+J', @(0x11, 0x4A)), @('Ctrl+H', @(0x11, 0x48)), @('F7', @(0x76)), @('Ctrl+N', @(0x11, 0x4E)),
            @('Ctrl+O', @(0x11, 0x4F)))) {
        $p = Probe-And-Note $k[0] ([byte[]]$k[1]) "RF25-$($k[0] -replace '[^A-Za-z0-9]', '-')"
        Dismiss $p
        if ((Screen-Now) -ne 'boveda abierta') { Log "   tras $($k[0]) la pantalla es: $(Screen-Now)"; Ensure-VaultOpen; Open-Entry $RootName $Rf.Title }
    }
}
Step 'RF25 Zoom del navegador' -Continue {
    Ensure-VaultOpen
    $w0 = (Find-El -Name 'Bloquear' -Type Button).Current.BoundingRectangle.Width
    Keys @(0x11, 0xBB); Start-Sleep -Milliseconds 700
    $w1 = (Find-El -Name 'Bloquear' -Type Button).Current.BoundingRectangle.Width
    Keys @(0x11, 0x30); Start-Sleep -Milliseconds 500
    $w2 = (Find-El -Name 'Bloquear' -Type Button).Current.BoundingRectangle.Width
    Log "ZOOM: ancho de 'Bloquear' antes $w0; tras Ctrl++ $w1; tras Ctrl+0 $w2"
    if ([math]::Abs($w1 - $w0) -gt 1) { Note "Ctrl++ cambia el zoom de la pagina (ancho de 'Bloquear' $w0 -> $w1); Ctrl+0 lo devuelve ($w2)" }
}
Step 'RF26 Imprimir' -Continue {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $p = Probe-And-Note 'Ctrl+P' ([byte[]]@(0x11, 0x50)) 'RF26-ctrl-p'
    Dismiss $p
    Start-Sleep -Milliseconds 500
    $p2 = Probe-And-Note 'Ctrl+Shift+P' ([byte[]]@(0x11, 0x10, 0x50)) 'RF27-ctrl-shift-p'
    Dismiss $p2
    Check 'Ctrl+P y Ctrl+Shift+P no abren la impresion' ($p.NewHandles.Count -eq 0 -and $p.FgIsApp -and $p.Added.Count -eq 0 -and $p2.NewHandles.Count -eq 0 -and $p2.FgIsApp -and $p2.Added.Count -eq 0)
    Log "   pantalla tras imprimir: $(Screen-Now); primer plano es la app = $([QA.Native]::GetForegroundWindow() -eq (Get-AppHwnd))"
}
Step 'RF27 Guardar la pagina' -Continue {
    Ensure-VaultOpen
    Open-Entry $RootName $Rf.Title
    $p = Probe-And-Note 'Ctrl+S' ([byte[]]@(0x11, 0x53)) 'RF28-ctrl-s'
    $saved = Handle-SaveDialog $p
    Dismiss $p
    Check 'Ctrl+S no guarda la pagina' (-not $saved -and $p.NewHandles.Count -eq 0 -and $p.FgIsApp)
}
Step 'RF28 Recargar' -Continue {
    $r1 = Probe-Reload 'F5' ([byte[]]@(0x74))
    $r2 = Probe-Reload 'Ctrl+R' ([byte[]]@(0x11, 0x52))
    $r3 = Probe-Reload 'Ctrl+Shift+R' ([byte[]]@(0x11, 0x10, 0x52))
    $r4 = Probe-Reload 'Ctrl+F5' ([byte[]]@(0x11, 0x74))
    Check 'F5, Ctrl+R, Ctrl+Shift+R y Ctrl+F5 no recargan la pagina' (-not ($r1 -or $r2 -or $r3 -or $r4)) "F5=$r1 Ctrl+R=$r2 Ctrl+Shift+R=$r3 Ctrl+F5=$r4"
}
Step 'RF29 Ctrl+F con la boveda bloqueada y en la bienvenida' -Continue {
    Ensure-VaultOpen
    Start-Sleep -Milliseconds 2500
    [void](Lock-Timed)
    $p = Probe-And-Note 'Ctrl+F (boveda bloqueada)' ([byte[]]@(0x11, 0x46)) 'RF29-ctrl-f-bloqueada'
    Dismiss $p
    $p = Probe-And-Note 'Ctrl+P (boveda bloqueada)' ([byte[]]@(0x11, 0x50))
    Dismiss $p
    Ensure-VaultOpen
}
Step 'RF30 Panel de actualizacion con la boveda abierta (sin descargar ni instalar)' -Continue {
    Ensure-VaultOpen
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $btn = Find-El -Name 'Buscar actualizaciones' -Type Button -Timeout 3 -Quiet
    Check 'Con la boveda abierta, Ajustes tiene el panel "Actualizaciones" con "Buscar actualizaciones"' (((Get-Names) -contains 'Actualizaciones') -and $null -ne $btn)
    $before = @(Get-Names)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $sawChecking = $false; $text = ''
    while ($sw.Elapsed.TotalSeconds -lt 45) {
        $now = @(Get-Names)
        if ($now -contains 'Buscando actualizaciones…') { $sawChecking = $true; Start-Sleep -Milliseconds 150; continue }
        $new = @($now | Where-Object { $before -notcontains $_ } | Select-Object -Unique)
        if ($new.Count) { $text = $new -join ' | '; break }
        Start-Sleep -Milliseconds 150
    }
    Log ("ACTUALIZACION: se vio 'Buscando actualizaciones…' = {0}; respuesta a los {1:n1} s: {2}" -f $sawChecking, $sw.Elapsed.TotalSeconds, $text)
    Shot 'RF30-buscar-actualizaciones'
    Show-Net 'tras buscar actualizaciones'
    if (Has-El -Name 'Ahora no' -Type Button -Timeout 0.5) { Log '   hay una version nueva ofrecida: se pulsa "Ahora no" (no se descarga ni se instala)'; Click-Name -Name 'Ahora no' }
    Check '"Buscar actualizaciones" responde con un texto' ([bool]$text) $text
    Check 'La boveda sigue abierta tras buscar (nada se cerro ni se instalo)' ((Get-AppProc).Count -eq 1)
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 400
    Check 'Tras cerrar Ajustes la boveda sigue abierta' ((Screen-Now) -eq 'boveda abierta')
}
Log '--- hallazgos de la superficie de navegador ---'
foreach ($f in $Findings) { Log "  * $f" }
Summary
