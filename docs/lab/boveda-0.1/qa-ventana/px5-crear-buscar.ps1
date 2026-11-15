# Pase de interaccion (5): tareas b y c del recorrido sobre la boveda de muestra.
#  b. Crear una entrada con contrasena generada: Titulo -> "Generar" -> "Guardar entrada".
#  c. Buscar y copiar: Ctrl+K, escribir, Enter, copiar la contrasena; portapapeles, historial y borrado a los ~20 s.
# Crea antes la entrada canario (contrasena conocida) que usan esta tarea y la pasada de seguridad.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
Step 'PX50 Entrada canario en Personal' {
    Ensure-Cuentas; [void](Size-Window 1120 720)
    Pick-Group 'Personal'
    if (-not (Has-Entry $Rf.Title)) {
        $r = New-EntryPx $Rf.Title $Rf.User $Rf.Url $Rf.Notes $Rf.Pass
        Check 'Entrada canario creada (contrasena tecleada)' ($r.Closed -and (Has-Entry $Rf.Title)) "cerrado a los $($r.CloseMs) ms; disco a los $($r.DiskMs) ms"
    } else { Log '   la entrada canario ya existe' }
}
Step 'PX51 (b) Entrada con contrasena generada' -Continue {
    Ensure-Cuentas
    Pick-Group 'Personal'
    if (Has-Entry $Rf.Title2) { Log '   ya existe; no se repite'; return }
    $clicks = 0
    Click-Name -Name 'Nueva entrada'; $clicks++
    [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 300
    Log "   al abrir: foco en $(Focused-Desc); grupo anunciado: $((Get-Names | Where-Object { $_ -eq 'Personal' -or $_ -like 'En el grupo*' }) -join '')"
    Type-Text $Rf.Title2
    $pw = Find-El -Id 'entry-dialog-password'
    Log "   antes de Generar: campo de contrasena oculto = $($pw.Current.IsPassword); nombres de fortaleza: $((Get-Names | Where-Object { $_ -like 'Fortaleza*' -or $_ -like '*buena*' -or $_ -like '*débil*' }) -join ' ')"
    Click-Name -Name 'Generar'; $clicks++; Start-Sleep -Milliseconds 500
    $pw = Find-El -Id 'entry-dialog-password'
    $visible = -not $pw.Current.IsPassword; $gen = if ($visible) { Get-Value $pw } else { '' }
    if ($gen) { Add-Generated $gen; Set-State 'generada-b' $gen }
    Log "   tras Generar (1 clic): contrasena a la vista = $visible, $($gen.Length) caracteres; foco en $(Focused-Desc); fortaleza: $((Get-Names | Where-Object { $_ -like 'Fortaleza*' -or $_ -in @('muy buena', 'buena', 'regular', 'débil') }) -join ' ')"
    Shot-Client 'b-generada-en-el-dialogo'
    # Opciones del generador: solo mirar que despliega, sin cambiar nada.
    Click-Name -Name 'Opciones del generador'; Start-Sleep -Milliseconds 400
    Log "   opciones del generador: $((Get-All | Where-Object { (Type-Of $_) -in @('CheckBox', 'Slider', 'Spinner', 'Edit') -and $_.Current.Name -notin @('Título', 'Usuario o correo', 'Contraseña', 'Sitio', 'Notas') } | ForEach-Object { "[$(Type-Of $_)] $($_.Current.Name)" }) -join ' | ')"
    $pw2 = Get-Value (Find-El -Id 'entry-dialog-password')
    Log "   abrir las opciones cambia la contrasena generada = $($pw2 -cne $gen)"
    Click-Name -Name 'Ocultar opciones del generador'; Start-Sleep -Milliseconds 300
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Guardar entrada'; $clicks++
    $closeMs = -1; $diskMs = -1
    while ($sw.Elapsed.TotalSeconds -lt 20 -and ($closeMs -lt 0 -or $diskMs -lt 0)) {
        if ($closeMs -lt 0 -and -not (Dialog-Open 'Nueva entrada')) { $closeMs = [int]$sw.ElapsedMilliseconds }
        if ($diskMs -lt 0 -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $diskMs = [int]$sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 40
    }
    Start-Sleep -Milliseconds 400
    $detail = @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -eq $Rf.Title2 -and (LRect $_).X -ge 572 }).Count -gt 0
    Log "   tras Guardar: dialogo cerrado a los $closeMs ms; disco a los $diskMs ms; foco en $(Focused-Desc); avisos: $((Get-Names | Where-Object { $_ -like '*guardad*' -or $_ -like 'Guardando*' }) -join ' | ')"
    Check '(b) Un clic en Generar y uno en Guardar (mas el de Nueva entrada): la entrada queda en la lista y elegida en el detalle' ($closeMs -ge 0 -and (Has-Entry $Rf.Title2) -and $detail) "clics de raton: $clicks (se abrieron ademas las opciones del generador, 2 clics que no cuentan); detalle muestra el titulo = $detail"
    Shot-Client 'b-guardada'
}
Step 'PX52 (c) Buscar y copiar' -Continue {
    Ensure-Cuentas
    Pick-Group 'Trabajo'
    # Control positivo del historial: un texto sin marcar puesto por otro proceso tiene que aparecer.
    Set-ClipExternal 'centinela-px-historial'; Start-Sleep -Milliseconds 1800
    $ctl = History-Has 'centinela-px-historial'
    Log "   CONTROL del historial (Win+V): un texto sin marcar de otro proceso aparece = $ctl"
    [void](Focus-App)
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Check 'Ctrl+K lleva el foco a la busqueda' ((Focused-Desc) -like '*vault-search*') (Focused-Desc)
    Type-Text 'faropx-tit'; Start-Sleep -Milliseconds 700
    $head = (Find-El -Id 'entries-title' -Timeout 2).Current.Name
    $rows = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and (LRect $_) -and (LRect $_).X -ge 232 -and (LRect $_).X -lt 572 -and (LRect $_).Y -gt 120 } | ForEach-Object { $_.Current.Name })
    Log "   al escribir: cabecera de la lista = `"$head`"; filas: $($rows -join ' | '); conteo: $((Get-Names | Where-Object { $_ -match '^\d+ (entradas?|resultados?)' }) -join ' ')"
    Shot-Client 'c-resultados'
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $detail = @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -eq $Rf.Title -and (LRect $_).X -ge 572 }).Count -gt 0
    $f = Focused-Desc
    Check 'Enter abre el primer resultado en el detalle' $detail "foco tras Enter: $f"
    $seq0 = Get-ClipSeq
    $copy = Find-El -Name 'Copiar contraseña' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $copy; Start-Sleep -Milliseconds 500
    $clip = "$(Get-Clip)"; $formats = @(Get-ClipFormats)
    $toast = (Get-Names | Where-Object { $_ -like '*opiad*' -or $_ -like '*se borra*' -or $_ -like '*portapapeles*' }) -join ' | '
    $excl = @($ExclFormats | Where-Object { $formats -contains $_ })
    Check '(c) Copiar contrasena: el portapapeles lleva exactamente la contrasena de la entrada' ($clip -ceq $Rf.Pass) "secuencia del portapapeles $seq0 -> $(Get-ClipSeq); formatos de exclusion: $($excl.Count) de 3 ($($excl -join ', '))"
    Log "   aviso al pie: $toast"
    Shot-Client 'c-copiado'
    Start-Sleep -Milliseconds 1500
    $hist = History-Has $Rf.Pass
    Check '(c) La contrasena copiada NO entra al historial del portapapeles' ($hist -eq 'no' -and $ctl -eq 'si') "historial: $hist; control positivo: $ctl"
    $seen = New-Object System.Collections.Generic.List[string]; $cleared = -1
    while ($sw.Elapsed.TotalSeconds -lt 32) {
        $t = (Get-Names | Where-Object { $_ -like '*opiad*' -or $_ -like '*se borra*' }) -join ' | '
        if ($t -and ($seen.Count -eq 0 -or $seen[$seen.Count - 1] -ne $t)) { $seen.Add($t) }
        if ("$(Get-Clip)" -cne $Rf.Pass) { $cleared = [int]$sw.ElapsedMilliseconds; break }
        Start-Sleep -Milliseconds 500
    }
    $after = "$(Get-Clip)"
    Check '(c) El portapapeles se vacia solo a los ~20 s' ($cleared -ge 17000 -and $cleared -le 24000 -and $after -eq '') "dejo de llevar la contrasena a los $cleared ms; despues lleva $($after.Length) caracteres"
    Log "   textos del aviso vistos, en orden: $(($seen | Select-Object -First 6) -join '  ->  ')$(if ($seen.Count -gt 6) { "  ... ($($seen.Count) distintos)" })"
    Start-Sleep -Milliseconds 800
    Log "   tras vaciarse: aviso = $((Get-Names | Where-Object { $_ -like '*opiad*' -or $_ -like '*se borr*' -or $_ -like '*portapapeles*' }) -join ' | ')"
    # Escape en la busqueda la limpia
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 200; Press 'ESC'; Start-Sleep -Milliseconds 400
    Log "   Escape en la busqueda: valor = `"$(Get-Value (Find-El -Id 'vault-search'))`"; cabecera = `"$((Find-El -Id 'entries-title' -Timeout 2).Current.Name)`"; foco: $(Focused-Desc)"
    [void](History-Purge @('centinela-px-historial'))
}
Summary
