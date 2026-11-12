# Pase de interaccion (6): tarea d del recorrido. Editar la entrada canario (el dialogo abre con los datos, guardar,
# el detalle muestra lo nuevo), Notas crece, y Escape con cambios sin guardar: una vez (pregunta) y REPETIDO rapido
# (riesgo conocido: Chromium puede cerrar el <dialog> de todos modos). Tras cada rafaga se anotan cuatro cosas: el
# dialogo sigue abierto, la pregunta esta encima, el borrador sigue en el campo, la entrada guardada no cambio.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
$AskTitle = '¿Descartar los cambios?'   # ojo: $ask y $Ask son la misma variable en PowerShell
$only = if ($StepArgs.Count) { $StepArgs[0] } else { '' }
$NewUser = 'FAROPX-usuario-editado@example.com'
function Win-Open([string]$name) { return $null -ne (Find-El -Name $name -Type Window -Timeout 0.35 -Quiet) }
function Title-Value { $t = Find-El -Id 'entry-dialog-title' -Timeout 0.8 -Quiet; if ($null -eq $t) { return $null }; return (Get-Value $t) }
function Detail-Has([string]$text) { return @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -eq $text -and (LRect $_) -and (LRect $_).X -ge 572 }).Count -gt 0 }
# Estado tras una rafaga de Escape. $dialog: 'Editar entrada' o 'Nueva entrada'.
function Burst-State([string]$dialog, [string]$marker) {
    Start-Sleep -Milliseconds 700
    $ask = Win-Open $AskTitle; $dlg = Win-Open $dialog
    $val = Title-Value
    $draftNow = ($null -ne $val -and $val.Contains($marker))
    $draftAfter = $draftNow
    if ($ask) {
        # La pregunta tapa el dialogo en el arbol: se responde "Seguir editando" y se mira el campo.
        Click-Name -Name 'Seguir editando'; Start-Sleep -Milliseconds 500
        $val = Title-Value; $draftAfter = ($null -ne $val -and $val.Contains($marker)); $dlg = Win-Open $dialog
    }
    return [pscustomobject]@{ Ask = $ask; Dialog = $dlg; Draft = $draftAfter; Text = "pregunta encima = $ask; dialogo abierto$(if ($ask) { ' (tras «Seguir editando»)' }) = $dlg; borrador en el campo = $draftAfter; foco: $(Focused-Desc)" }
}
function Esc-Burst([int]$n, [switch]$NoPause) {
    [void](Focus-App)
    if ($NoPause) { for ($k = 0; $k -lt $n; $k++) { [QA.Native]::keybd_event(0x1B, 0, 0, [UIntPtr]::Zero); [QA.Native]::keybd_event(0x1B, 0, 2, [UIntPtr]::Zero) } }
    else { for ($k = 0; $k -lt $n; $k++) { [QA.Native]::Key(0x1B) } }
}
function Open-Edit {
    if (Win-Open 'Editar entrada') { return }
    Pick-Group 'Personal'; Pick-Entry $Rf.Title
    Click-Name -Name 'Editar'
    [void](Find-El -Name 'Editar entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
}
function Put-Marker([string]$marker) {
    $v = Title-Value
    if ($null -ne $v -and $v.Contains($marker)) { return }
    Click-El (Find-El -Id 'entry-dialog-title'); Press 'END'; Type-Text " $marker"; Start-Sleep -Milliseconds 200
}

if (-not $only -or $only -eq '60') { Step 'PX60 (d) Editar: abre con los datos, Notas crece, guardar, el detalle muestra lo nuevo' -Continue {
    Ensure-Cuentas; [void](Size-Window 1120 720)
    Open-Edit
    $vals = "titulo=`"$(Title-Value)`" usuario=`"$(Get-Value (Find-El -Id 'entry-dialog-username'))`" sitio=`"$(Get-Value (Find-El -Id 'entry-dialog-url'))`" notas=`"$(Get-Value (Find-El -Id 'entry-dialog-notes'))`""
    $pw = Find-El -Id 'entry-dialog-password'
    Check 'El dialogo "Editar entrada" abre con los datos de la entrada' ((Title-Value) -ceq $Rf.Title -and (Get-Value (Find-El -Id 'entry-dialog-username')) -ceq $Rf.User -and (Get-Value (Find-El -Id 'entry-dialog-url')) -ceq $Rf.Url -and (Get-Value (Find-El -Id 'entry-dialog-notes')) -ceq $Rf.Notes) "$vals; contrasena oculta = $($pw.Current.IsPassword); foco: $(Focused-Desc); grupo: $((Get-Names | Where-Object { $_ -like 'En el grupo*' -or $_ -eq 'Personal' }) -join '')"
    $n0 = LRect (Find-El -Id 'entry-dialog-notes')
    Click-El (Find-El -Id 'entry-dialog-notes'); [QA.Native]::Chord(0x11, 0x23); Start-Sleep -Milliseconds 100
    Type-Text "`nlinea dos`nlinea tres`nlinea cuatro`nlinea cinco`nlinea seis"; Start-Sleep -Milliseconds 400
    $n1 = LRect (Find-El -Id 'entry-dialog-notes')
    $still = Win-Open 'Editar entrada'
    Check 'Notas crece al escribir varias lineas (y Enter no envia el formulario)' ($still -and $n1.H -gt $n0.H + 30) "alto antes $($n0.H) px, con 6 lineas $($n1.H) px; dialogo sigue abierto = $still"
    Shot-Client 'd-editar-notas-seis-lineas'
    Fill-Id 'entry-dialog-username' $NewUser
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Guardar entrada'
    $closeMs = -1
    while ($sw.Elapsed.TotalSeconds -lt 15 -and $closeMs -lt 0) { if (-not (Win-Open 'Editar entrada')) { $closeMs = [int]$sw.ElapsedMilliseconds }; Start-Sleep -Milliseconds 50 }
    Start-Sleep -Milliseconds 1500
    $notesEl = @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -like "$($Rf.Notes)*" -and (LRect $_).X -ge 572 }) | Select-Object -First 1
    $nh = if ($notesEl) { (LRect $notesEl).H } else { -1 }
    Check 'Tras guardar, el detalle muestra el usuario nuevo y las notas con sus saltos de linea' ((Detail-Has $NewUser) -and $notesEl -and $notesEl.Current.Name -like '*linea seis' -and $nh -gt 100) "dialogo cerrado a los $closeMs ms; disco cambio = $((Get-Item $Vault).LastWriteTimeUtc -ne $stamp0); alto de las notas en el detalle $nh px; foco: $(Focused-Desc); aviso: $((Get-Names | Where-Object { $_ -like '*guardad*' }) -join ' | ')"
    Shot-Client 'd-detalle-editado'
} }
if (-not $only -or $only -eq '61') { Step 'PX61 (d) Escape y Cancelar con cambios sin guardar, una vez' -Continue {
    Ensure-Cuentas; Open-Edit; Put-Marker $Rf.Draft
    Press 'ESC'; Start-Sleep -Milliseconds 600
    $ask = Win-Open $AskTitle
    Log "   tras un Escape: pregunta = $ask; textos: $((Get-Names | Where-Object { $_ -notlike 'AlmarioKS*' }) -join ' | '); foco: $(Focused-Desc)"
    Shot-Client 'd-pregunta-descartar'
    Check 'Un Escape con cambios pregunta antes de cerrar, con el foco en "Seguir editando"' ($ask -and (Focused-Desc) -like '*Seguir editando*')
    Click-Name -Name 'Seguir editando'; Start-Sleep -Milliseconds 500
    Check '"Seguir editando" vuelve al dialogo con lo escrito' ((Win-Open 'Editar entrada') -and "$(Title-Value)".Contains($Rf.Draft)) "foco: $(Focused-Desc)"
    Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 600
    $ask2 = Win-Open $AskTitle
    Check '"Cancelar" con cambios tambien pregunta' $ask2
    if ($ask2) { Click-Name -Name 'Seguir editando'; Start-Sleep -Milliseconds 400 }
    Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 600
    $ask3 = Win-Open $AskTitle
    Check 'La X de cerrar con cambios tambien pregunta' $ask3
    if ($ask3) { Click-Name -Name 'Seguir editando'; Start-Sleep -Milliseconds 400 }
    # Escape con la pregunta abierta: ¿cierra la pregunta o descarta?
    Press 'ESC'; Start-Sleep -Milliseconds 600
    if (Win-Open $AskTitle) { Press 'ESC'; Start-Sleep -Milliseconds 700 }
    Log "   Escape, pausa, Escape (0,6 s entre los dos): pregunta = $(Win-Open $AskTitle); dialogo = $(Win-Open 'Editar entrada'); borrador = $("$(Title-Value)".Contains($Rf.Draft))"
} }
foreach ($dialog in @('Editar entrada', 'Nueva entrada')) {
    if ($only -and $only -ne '62') { continue }
    Step "PX62 (d) Escape REPETIDO rapido en «$dialog»" -Continue {
        Ensure-Cuentas
        $lost = 0
        foreach ($burst in @(@(2, $false), @(3, $false), @(5, $false), @(4, $true), @(8, $true))) {
            if ($dialog -eq 'Editar entrada') { Open-Edit }
            elseif (-not (Win-Open 'Nueva entrada')) { Pick-Group 'Personal'; Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350 }
            Put-Marker $Rf.Draft
            Esc-Burst $burst[0] -NoPause:$burst[1]
            $s = Burst-State $dialog $Rf.Draft
            $silent = (-not $s.Ask) -and (-not $s.Dialog)
            if (-not $s.Draft) { $lost++ }
            $saved = if ($dialog -eq 'Editar entrada' -and -not $s.Dialog) { "; la entrada guardada conserva su titulo = $(Has-Entry $Rf.Title)" } else { '' }
            Log "RAFAGA [$dialog] $($burst[0]) Escape $(if ($burst[1]) { 'sin pausa entre teclas' } else { 'a ~30 ms' }): $($s.Text)$saved$(if ($silent) { ' >>> CERRADO SIN PREGUNTAR' })"
        }
        Check "Escape repetido en «$dialog»: el borrador nunca se pierde en silencio" ($lost -eq 0) "rafagas con el borrador perdido: $lost de 5"
        # Dejar limpio: descartar a proposito.
        if (Win-Open $dialog) { Press 'ESC'; Start-Sleep -Milliseconds 600; if (Win-Open $AskTitle) { Click-Name -Name 'Descartar'; Start-Sleep -Milliseconds 600 } }
        Check "«Descartar» cierra «$dialog» y no guarda el borrador" (-not (Win-Open $dialog) -and -not (Win-Open $AskTitle) -and (Has-El -Name 'Bloquear' -Type Button -Timeout 2) -and -not (Has-Name "*$($Rf.Draft)*")) "foco: $(Focused-Desc)"
    }
}
if (-not $only -or $only -eq '63') { Step 'PX63 Guardar sin titulo' -Continue {
    Ensure-Cuentas; Pick-Group 'Personal'
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
    Click-Name -Name 'Guardar entrada'; Start-Sleep -Milliseconds 600
    Log "   guardar sin titulo: dialogo abierto = $(Win-Open 'Nueva entrada'); mensaje: $((Get-Names | Where-Object { $_ -like '*título*' -and $_ -ne 'Título' }) -join ' | '); foco: $(Focused-Desc)"
    Shot-Client 'd-sin-titulo'
    Press 'ESC'; Start-Sleep -Milliseconds 500
    if (Win-Open $AskTitle) { Click-Name -Name 'Descartar'; Start-Sleep -Milliseconds 400 }
} }
Summary
