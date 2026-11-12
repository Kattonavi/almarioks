# Ronda 2 (4): gestos de la lista y de la ventana de la entrada con raton y teclado reales.
#  (a) buscar + doble clic en CONTRASENA (portapapeles exacto, 3 formatos de exclusion, fuera del historial, vacio a
#      los ~20 s, aviso al pie), en USUARIO y en SITIO (navegador con una direccion *.example).
#  (b) doble clic en la FECHA abre la ventana con las dos fechas con hora; Editar, cambiar un dato, Guardar; Esc.
#  (c) camino de teclado: Ctrl+K, texto, Enter, Enter; flecha abajo y Enter sobre el titulo.
# Argumento opcional: a, b o c para correr solo esa parte. RF_VARIANT=py-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { '' }
function Notice-Now {
    @(Get-All | Where-Object { $_.Current.Name -and (Type-Of $_) -eq 'Text' } | Where-Object { $r = LRect $_; $null -ne $r -and $r.X -ge 232 -and $r.Y -ge 630 -and $r.Y -le 688 } | ForEach-Object { "`"$($_.Current.Name)`" (y=$((LRect $_).Y))" }) -join ' '
}
function Wait-ClipEmpty([datetime]$t0, [int]$maxSec = 28) {
    while (((Get-Date) - $t0).TotalSeconds -lt $maxSec) { if ("$(Get-Clip)" -eq '') { return [math]::Round(((Get-Date) - $t0).TotalSeconds, 1) }; Start-Sleep -Milliseconds 300 }
    return -1
}
function Fg-Title { [QA.Native]::TitleOf([QA.Native]::GetForegroundWindow()) }
function Browser-Sign([string]$needle) {
    $fg = Fg-Title
    if ($fg -like "*$needle*") { return 'ventana en primer plano con esa direccion en el titulo' }
    if (@(Get-Process | Where-Object { $_.MainWindowTitle -like "*$needle*" }).Count) { return 'ventana de otro proceso con esa direccion en el titulo' }
    $p = @(Get-CimInstance Win32_Process -Filter "CommandLine LIKE '%$needle%'" -ErrorAction SilentlyContinue | Where-Object { $_.Name -notlike 'powershell*' })
    if ($p.Count) { return "proceso $($p[0].Name) lanzado con esa direccion" }
    return ''
}
Step 'PY40 Preparacion' { Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search; Pick-Group 'Trabajo'; [void](Purge-Mine) }

if (-not $only -or $only -eq 'a') {
Step 'PY41 (a) Buscar y doble clic en la CONTRASENA' -Continue {
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Check 'Ctrl+K lleva el foco al buscador' ((Focused-Desc) -like '*Buscar en la b*') (Focused-Desc)
    Type-Text 'jira'; Start-Sleep -Milliseconds 900
    Set-ClipExternal 'centinela-py-antes'; Start-Sleep -Milliseconds 900
    Log "   CONTROL del historial de Windows: un texto puesto por otro programa SI entra = $(History-Has 'centinela-py-antes')"
    $cell = Cell-Of 'Jira' 'CONTRASEÑA'; $m = Mid $cell
    Log "   celda de la contrasena de Jira: `"$($cell.Current.Name)`" $(LRect-Text $cell); fila elegida antes = $(Has-El -Name 'Abrir Jira' -Type Button -Timeout 0.3)"
    Dbl-At $m[0] $m[1]; $t0 = Get-Date
    Start-Sleep -Milliseconds 350
    $st = Clip-State $PyPass['Jira']
    Check '(a) Doble clic real en la CONTRASENA (fila no elegida): el portapapeles tiene el valor exacto' $st.Exact "longitud en el portapapeles = $($st.Len) (esperada $($PyPass['Jira'].Length))"
    Check '(a) Con los tres formatos de exclusion' ($st.Excl -eq 3) "formatos: $($st.Formats)"
    Log "   AVISO a los 0,5 s: $(Notice-Now)"
    $h = History-Has $PyPass['Jira']
    Check '(a) La contrasena copiada NO esta en el historial de Windows (WinRT)' ($h -eq 'no') "historial = $h"
    $hits = (Tree-Find (Pass-Needles)).Hits
    Check '(a) Tras copiar, la contrasena no aparece en el arbol de accesibilidad y la celda sigue diciendo "Oculta"' ($hits.Count -eq 0 -and (Cell-Of 'Jira' 'CONTRASEÑA').Current.Name -eq 'Oculta') "hallazgos: $($hits -join ' | ')"
    Start-Sleep -Seconds 6
    Log "   AVISO a los ~9 s: $(Notice-Now)"
    $empty = Wait-ClipEmpty $t0
    Check '(a) A los ~20 s el portapapeles esta vacio' ($empty -ge 17 -and $empty -le 23) "vacio a los $empty s; formatos despues: $((Get-ClipFormats) -join ', ')"
    Start-Sleep -Milliseconds 800
    Log "   AVISO tras vaciarse: $(Notice-Now)"
}
Step 'PY42 (a) Doble clic en el USUARIO y en el SITIO' -Continue {
    Set-ClipExternal 'centinela-py-antes'; Start-Sleep -Milliseconds 300
    $cell = Cell-Of 'Jira' 'USUARIO'; $m = Mid $cell
    Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 350
    $st = Clip-State 'ana.g'
    Check '(a) Doble clic real en el USUARIO lo copia' $st.Exact "portapapeles = `"$($st.Text)`"; formatos de exclusion = $($st.Excl); aviso: $(Notice-Now)"
    $before = Browser-Sign 'jira.example'
    $cell = Cell-Of 'Jira' 'SITIO'; $m = Mid $cell
    Log "   celda del sitio: `"$($cell.Current.Name)`"; antes del gesto hay rastro del navegador con esa direccion = `"$before`""
    Dbl-At $m[0] $m[1]
    $sign = ''; $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt 10 -and -not $sign) { $sign = Browser-Sign 'jira.example'; if (-not $sign) { Start-Sleep -Milliseconds 150 } }
    $fgProc = ''; try { [uint32]$owner = 0; [void][QA.Native]::GetWindowThreadProcessId([QA.Native]::GetForegroundWindow(), [ref]$owner); $fgProc = (Get-Process -Id $owner).Name } catch { }
    Check '(a) Doble clic real en el SITIO abre el navegador con esa direccion' ([bool]$sign -and -not $before) "senal: $sign (a los $([int]$sw.ElapsedMilliseconds) ms); proceso en primer plano: $fgProc"
    Start-Sleep -Milliseconds 1500
    if ((Fg-Title) -like '*jira.example*') {
        [QA.Native]::Chord(0x11, [byte][char]'W'); Start-Sleep -Milliseconds 900
        Log "   pestana cerrada con Ctrl+W (solo esa: el titulo en primer plano llevaba la direccion); sigue alguna ventana con esa direccion = $([bool](@(Get-Process | Where-Object { $_.MainWindowTitle -like '*jira.example*' }).Count))"
    } else { Log '   NO se cerro la pestana: la ventana en primer plano no lleva esa direccion en el titulo' }
    [void](Focus-App); Start-Sleep -Milliseconds 400
    Clear-Search; Pick-Group 'Trabajo'; Start-Sleep -Milliseconds 300
    $cell = Cell-Of 'VPN de la oficina' 'SITIO'
    Set-ClipExternal 'centinela-py-antes'; $m = Mid $cell; $fg0 = [QA.Native]::GetForegroundWindow()
    Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 1200
    Check '(a) Doble clic en un sitio vacio (raya) no hace nada' ([QA.Native]::GetForegroundWindow() -eq $fg0 -and "$(Get-Clip)" -ceq 'centinela-py-antes' -and $null -eq (Find-El -Type Window -Timeout 0.3 -Quiet)) "celda `"$($cell.Current.Name)`""
}
}

if (-not $only -or $only -eq 'b') {
Step 'PY43 (b) Doble clic en la FECHA, ventana con fechas con hora, Editar y Guardar, Esc' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Trabajo'; Pick-Entry 'Figma'; Start-Sleep -Milliseconds 400
    $cell = Cell-Of 'Jira' 'MODIFICADA'; $m = Mid $cell
    Dbl-At $m[0] $m[1]
    $win = Entry-Window 'Jira' 3
    Check '(b) Doble clic real en la FECHA de una fila no elegida abre la ventana de la entrada' ($null -ne $win) "celda `"$($cell.Current.Name)`""
    if ($null -eq $win) { throw 'sin ventana' }
    $texts = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name } | ForEach-Object { $_.Current.Name })
    $buttons = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Button' } | ForEach-Object { $_.Current.Name })
    Log "   VENTANA: textos: $($texts -join ' | ')"
    Log "   VENTANA: botones: $($buttons -join ' | ')"
    $dates = @($texts | Where-Object { $_ -match '^\d{1,2} \w+ \d{4}, \d{1,2}:\d{2}\s[ap]\.\s?m\.$' })
    Check '(b) La ventana trae usuario, contrasena oculta, sitio y las dos fechas CON HORA' (($texts -contains 'ana.g') -and ($texts -contains '(oculto)') -and ($texts -contains 'https://jira.example') -and ($texts -contains 'Creada el ') -and ($texts -contains 'Modificada el ') -and $dates.Count -eq 2) "fechas: $($dates -join ' / ')"
    $created0 = $dates[0]
    $u = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -eq 'ana.g' })[0]
    Set-ClipExternal 'centinela-py-antes'; $m = Mid $u; Dbl-At $m[0] $m[1]; Start-Sleep -Milliseconds 400
    $notice = @(Get-Names (Entry-Window 'Jira' 1) | Where-Object { $_ -match 'opiad|borra' }) -join ' | '
    Check '(b) Doble clic sobre el usuario DENTRO de la ventana lo copia y el aviso sale en la ventana' ((Clip-State 'ana.g').Exact -and [bool]$notice) "aviso en la ventana: $notice"
    Click-Name -Name 'Editar'
    $form = Find-El -Name 'Editar entrada' -Type Window -Timeout 5; Start-Sleep -Milliseconds 500
    Check '(b) "Editar" cambia la ventana por el formulario' ($null -ne $form -and $null -eq (Entry-Window 'Jira' 0.3)) "foco: $(Focused-Desc)"
    Fill-Id 'entry-dialog-username' 'ana.gomez'
    $now = Get-Date
    Click-Name -Name 'Guardar entrada'
    $win = Entry-Window 'Jira' 6; Start-Sleep -Milliseconds 700
    $win = Entry-Window 'Jira' 2
    $texts = @($win.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name } | ForEach-Object { $_.Current.Name })
    $dates = @($texts | Where-Object { $_ -match '^\d{1,2} \w+ \d{4}, \d{1,2}:\d{2}\s[ap]\.\s?m\.$' })
    $ok = $false; $diff = 999
    if ($dates.Count -eq 2 -and $dates[1] -match ', (\d{1,2}):(\d{2})\s([ap])\.') {
        $hh = [int]$Matches[1] % 12; if ($Matches[3] -eq 'p') { $hh += 12 }
        $diff = [math]::Abs(($hh * 60 + [int]$Matches[2]) - ($now.Hour * 60 + $now.Minute)); $ok = ($diff -le 1)
    }
    Check '(b) "Guardar entrada" vuelve a la ventana con el dato nuevo' (($texts -contains 'ana.gomez') -and -not ($texts -contains 'ana.g') -and $null -eq (Find-El -Name 'Editar entrada' -Type Window -Timeout 0.3 -Quiet)) "foco: $(Focused-Desc)"
    Check '(b, punto 6) "Modificada" cambia a la hora local del guardado, al minuto; "Creada" no cambia' ($ok -and $dates[0] -eq $created0) "reloj del equipo al guardar: $($now.ToString('H:mm')); ventana: Creada $($dates[0]) / Modificada $($dates[1]); diferencia $diff min"
    Shot-Client 'extra-entrada-editada'
    Press 'ESC'; Start-Sleep -Milliseconds 700
    $chosen = Has-El -Name 'Abrir Jira' -Type Button -Timeout 1
    Check '(b) Esc en la ventana: vuelve a la lista con la fila elegida y el foco en ella' ($null -eq (Entry-Window 'Jira' 0.3) -and $chosen -and (Focused-Desc) -like '*"Jira"*') "fila elegida = $chosen; foco: $(Focused-Desc); usuario en la lista: $((Cell-Of 'Jira' 'USUARIO').Current.Name); fecha en la lista: $((Cell-Of 'Jira' 'MODIFICADA').Current.Name)"
}
}

if (-not $only -or $only -eq 'c') {
Step 'PY44 (c) Camino de teclado' -Continue {
    Ensure-Cuentas; Clear-Search; Pick-Group 'Personal'; Start-Sleep -Milliseconds 300
    Set-ClipExternal 'centinela-py-antes'
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Type-Text 'slack'; Start-Sleep -Milliseconds 900
    Press 'ENTER'; Start-Sleep -Milliseconds 500
    $f1 = Focused-Desc
    Check '(c) Ctrl+K, texto, Enter: el foco queda en "Copiar contraseña de Slack"' ($f1 -like '*Copiar contrase*a de Slack*') $f1
    Press 'ENTER'; Start-Sleep -Milliseconds 450
    $st = Clip-State $PyPass['Slack']
    Check '(c) Enter otra vez: copia la contrasena (valor exacto, 3 formatos de exclusion)' ($st.Exact -and $st.Excl -eq 3) "longitud $($st.Len) de $($PyPass['Slack'].Length); aviso: $(Notice-Now)"
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Press 'DOWN'; Start-Sleep -Milliseconds 400
    $f2 = Focused-Desc
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $win = Entry-Window 'Slack' 2
    Check '(c) Flecha abajo lleva al titulo de la fila y Enter sobre el titulo abre la ventana' ($f2 -like '*"Slack"*' -and $null -ne $win) "foco tras flecha abajo: $f2; foco con la ventana abierta: $(Focused-Desc)"
    Press 'ESC'; Start-Sleep -Milliseconds 600
    Log "   tras Esc: foco $(Focused-Desc)"
    Clear-Search
}
}
Step 'PY49 Portapapeles e historial limpios' -Continue {
    if ("$(Get-Clip)" -match 'FAROPY|centinela-py|ana\.g') { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'Portapapeles e historial sin canarios' (-not ("$(Get-Clip)" -match 'FAROPY|centinela-py') -and (History-Has 'FAROPY') -eq 'no' -and (History-Has 'centinela-py') -eq 'no') "borrados del historial: $n"
}
Summary
