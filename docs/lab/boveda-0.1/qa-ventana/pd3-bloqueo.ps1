# Decisiones del dueno (3), RF-114: Ctrl+L con teclas reales (keybd_event) bloquea desde la lista, desde "Nueva entrada"
# con cambios y el foco en "Título" (sin preguntar) y desde la ventana de la entrada con la contrasena a la vista; la
# ayuda del boton dice "Bloquear (Ctrl+L)". Controles: Ctrl+K sigue llevando a la busqueda; Ctrl+Alt+L (AltGr) no
# bloquea; Esc en el mismo formulario sucio si pregunta; el recorrido del arbol si ve lo tecleado y lo mostrado antes de
# bloquear. RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$Draft = 'FAROPD-borrador-b'; $DraftPass = 'FAROPDborrador7731'
function Ask-Now { (Names-Like 'escartar los cambios|Seguir editando') -join ' | ' }

Step 'PD30 (d) Ayuda del boton y controles Ctrl+K y Ctrl+Alt+L' -Continue {
    Ensure-Pd; [void](Size-Window 1120 720); Clear-Search
    $b = Find-El -Name 'Bloquear' -Type Button -Timeout 3
    $help = Help-Of $b
    Check '(1d) El boton "Bloquear" tiene la ayuda "Bloquear (Ctrl+L)"' ($help -ceq 'Bloquear (Ctrl+L)') "nombre `"$($b.Current.Name)`"; ayuda `"$help`""
    Pick-Entry $PdShort
    $before = Focused-Desc
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 500
    $f = $AE::FocusedElement
    Check '(control) Ctrl+K sigue llevando a la busqueda' ($f.Current.AutomationId -eq 'vault-search') "foco antes: $before; despues: $(Focused-Desc)"
    Pick-Entry $PdShort; $foc = Focused-Desc
    Keys @(0xA2, 0xA5, 0x4C); Start-Sleep -Milliseconds 1500
    $still = (Has-El -Name 'Bloquear' -Type Button -Timeout 0.5) -and -not (Has-El -Id 'unlock-password' -Timeout 0.3)
    Check '(control) Ctrl+Alt+L (AltGr: Ctrl izq. + Alt der. + L) NO bloquea' $still "foco al pulsar: $foc; boveda abierta 1,5 s despues = $still; foco ahora: $(Focused-Desc)"
    Set-State 'pd-altgr-foco' $foc
}
Step 'PD31 (a) Ctrl+L con la lista a la vista' -Continue {
    Ensure-Pd
    if (-not (Has-El -Name 'Bloquear' -Type Button -Timeout 0.5)) { throw 'La boveda no esta abierta' }
    Pick-Entry $PdShort; $foc = Focused-Desc
    Press-Ctrl 'l'; $ms = Wait-Locked 4
    Check '(1a) Ctrl+L con la lista a la vista bloquea (pantalla "Desbloquear …")' ($ms -ge 0) "foco al pulsar: $foc; pantalla de desbloqueo a los $ms ms; titulo: $((Names-Like '^Desbloquear') -join ' | ')"
    Shot-Client 'pd-1a-bloqueada'
    Unlock-Pd
}
Step 'PD32 (b) Ctrl+L con "Nueva entrada" con cambios y el foco en Titulo' -Continue {
    Ensure-Pd
    # Control del detector: el mismo formulario sucio con Esc SI pregunta.
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    Type-Text $Draft; Fill-Id 'entry-dialog-password' $DraftPass -Secret; Start-Sleep -Milliseconds 500
    Press 'ESC'; Start-Sleep -Milliseconds 700
    $askEsc = Ask-Now
    $d = Find-El -Name 'Descartar' -Type Button -Timeout 1 -Quiet
    if ($null -ne $d) { Click-El $d; Start-Sleep -Milliseconds 600 }
    Check '(control) En ese formulario sucio, Esc SI pregunta' ($askEsc -match 'escartar') "textos: `"$askEsc`""
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    Type-Text $Draft; Fill-Id 'entry-dialog-password' $DraftPass -Secret; Start-Sleep -Milliseconds 500
    Click-El (Find-El -Id 'entry-dialog-title'); Start-Sleep -Milliseconds 300
    $foc = Focused-Desc
    $pre = Tree-Find @{ 'titulo tecleado' = $Draft; 'contrasena tecleada' = $DraftPass }
    $pwLen = "$(Raw-Value (Find-El -Id 'entry-dialog-password'))".Length
    Press-Ctrl 'l'
    $sw = [Diagnostics.Stopwatch]::StartNew(); $askSeen = ''; $ms = -1
    while ($sw.Elapsed.TotalSeconds -lt 4 -and $ms -lt 0) { $a = Ask-Now; if ($a) { $askSeen = $a }; if (Is-Locked) { $ms = [int]$sw.ElapsedMilliseconds }; Start-Sleep -Milliseconds 80 }
    $atLock = Tree-Find @{ 'titulo tecleado' = $Draft; 'contrasena tecleada' = $DraftPass }
    Check '(1b) Ctrl+L con el formulario sucio y el foco en Titulo bloquea SIN preguntar' ($ms -ge 0 -and -not $askSeen) "foco al pulsar: $foc; antes, el arbol ve: $($pre.Hits -join ', ') (contrasena en el campo: $pwLen caracteres); bloqueada a los $ms ms; pregunta vista = `"$askSeen`"; en la pantalla de desbloqueo: $($atLock.Hits.Count) hallazgos"
    Unlock-Pd; Start-Sleep -Milliseconds 800
    $post = Tree-Find @{ 'titulo tecleado' = $Draft; 'contrasena tecleada' = $DraftPass }
    $dlg = Dialog-Open 'Nueva entrada'
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    $t = "$(Get-Value (Find-El -Id 'entry-dialog-title'))"; $p = "$(Raw-Value (Find-El -Id 'entry-dialog-password'))"
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $d = Find-El -Name 'Descartar' -Type Button -Timeout 0.5 -Quiet; if ($null -ne $d) { Click-El $d; Start-Sleep -Milliseconds 500 }
    Check '(1b) Tras desbloquear lo tecleado no esta en el arbol ni vuelve al reabrir "Nueva entrada"' ($post.Hits.Count -eq 0 -and $pre.Hits.Count -ge 1 -and -not $dlg -and $t -eq '' -and $p -eq '' -and -not (Has-Entry $Draft)) "tras desbloquear: $($post.Hits.Count) hallazgos en $($post.Count) textos (control antes: $($pre.Hits.Count)); dialogo abierto solo = $dlg; reabierto: titulo `"$t`", contrasena de $($p.Length) caracteres; entrada guardada = $(Has-Entry $Draft)"
}
Step 'PD33 (c) Ctrl+L con la ventana de la entrada y la contrasena a la vista' -Continue {
    Ensure-Pd; Clear-Search
    Pick-Entry $PdShort; Click-Name -Name "Abrir $PdShort"
    $w = Entry-Window $PdShort 4; Start-Sleep -Milliseconds 500
    Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root $w); Start-Sleep -Milliseconds 600
    $pre = Tree-Find @{ 'contrasena corta' = $PdShortPass }
    $foc = Focused-Desc
    Press-Ctrl 'l'; $ms = Wait-Locked 4
    $atLock = Tree-Find @{ 'contrasena corta' = $PdShortPass }
    Check '(1c) Ctrl+L con la ventana de la entrada y la contrasena mostrada bloquea' ($ms -ge 0 -and $pre.Hits.Count -ge 1 -and $atLock.Hits.Count -eq 0) "antes, el arbol la ve: $($pre.Hits -join ', '); foco al pulsar: $foc; bloqueada a los $ms ms; en la pantalla de desbloqueo: $($atLock.Hits.Count) hallazgos"
    Unlock-Pd; Start-Sleep -Milliseconds 800
    $post = Tree-Find @{ 'contrasena corta' = $PdShortPass }
    Start-Sleep -Milliseconds 1500; $post2 = Tree-Find @{ 'contrasena corta' = $PdShortPass }
    $win = $null -ne (Entry-Window $PdShort 0.5)
    Check '(1c) Tras desbloquear la contrasena no esta a la vista ni en el arbol' ($post.Hits.Count -eq 0 -and $post2.Hits.Count -eq 0 -and -not $win) "hallazgos tras desbloquear: $($post.Hits.Count), 1,5 s despues: $($post2.Hits.Count); ventana de la entrada abierta = $win"
}
Summary
