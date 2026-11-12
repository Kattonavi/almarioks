# Pase de interaccion: ayudas de los pasos px* con los nombres accesibles de la interfaz nueva (descubiertos por UIA
# con px0-ver.ps1, no supuestos). Se carga despues de rf-lib.ps1 (RF_VARIANT=px-lib).

# Deja la boveda de muestra abierta venga de donde venga: app cerrada, bienvenida, desbloqueo o un dialogo abierto.
function Ensure-Cuentas {
    if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'preparacion' }
    [void](Focus-App)
    # Un dialogo modal abierto esconde el resto del arbol: se cierra con Escape (sin cambios no pregunta).
    # Si el dialogo tenia cambios, Escape pregunta: se responde "Descartar".
    for ($k = 0; $k -lt 4 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) {
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet
        if ($null -ne $ask) { Click-El $ask } else { Press 'ESC' }
        Start-Sleep -Milliseconds 500
    }
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.6) { return }
    if (Has-El -Id 'welcome-title' -Timeout 0.6) {
        Click-Name -Name 'Abrir Cuentas.kdbx'      # nombre exacto: en la lista hay recientes que no son de esta prueba
        [void](Find-El -Id 'unlock-title' -Timeout 6); Start-Sleep -Milliseconds 400
    }
    if (Has-El -Id 'unlock-password' -Timeout 3) {
        Fill-Id 'unlock-password' $RfMaster -Secret
        Click-Name -Name 'Desbloquear'
        [void](Find-El -Name 'Bloquear' -Type Button -Timeout 30); Start-Sleep -Milliseconds 600
        Log '   (la boveda estaba bloqueada: desbloqueada)'
    }
    if (-not (Has-El -Name 'Bloquear' -Type Button -Timeout 3)) { throw 'No se pudo dejar la boveda de muestra abierta' }
}
function Group-BtnPx([string]$name) { Find-El -Like "$name · * entrada*" -Type Button -Timeout 5 }
function Pick-Group([string]$name) { Click-El (Group-BtnPx $name); Start-Sleep -Milliseconds 350 }
function Pick-All { Click-El (Find-El -Like 'Todas las entradas · *' -Type Button -Timeout 5); Start-Sleep -Milliseconds 350 }
# Las filas de la lista: su nombre empieza por el titulo.
function Entry-Row([string]$title, [double]$timeout = 1.5) {
    $list = Find-El -Like 'Entradas de *' -Type Group -Timeout 2 -Quiet
    if ($null -eq $list) { $list = Find-El -Id 'entries-title' -Timeout 2 -Quiet; if ($null -ne $list) { $list = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($list) } }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($el in @(Get-All)) {
            $t = Type-Of $el; $n = $el.Current.Name
            if (($t -eq 'Button' -or $t -eq 'ListItem') -and ($n -eq $title -or $n -like "$title *" -or $n -like "$title·*" -or $n -like "$title,*")) {
                $r = LRect $el
                if ($null -ne $r -and $r.X -ge 232 -and $r.X -lt 572) { return $el }
            }
        }
        Start-Sleep -Milliseconds 120
    } while ($sw.Elapsed.TotalSeconds -lt $timeout)
    return $null
}
function Has-Entry([string]$title) { return $null -ne (Entry-Row $title 1.2) }
function Pick-Entry([string]$title) { $row = Entry-Row $title 4; if ($null -eq $row) { throw "No esta la fila de $title" }; Click-El $row; Start-Sleep -Milliseconds 450 }
function Dialog-Open([string]$name) { return $null -ne (Find-El -Name $name -Type Window -Timeout 0.4 -Quiet) }
# Nueva entrada con contrasena generada: un clic en "Generar" y uno en "Guardar entrada". Devuelve tiempos y si la
# contrasena generada quedo a la vista (entonces se anota como aguja para la inspeccion del disco).
function New-EntryPx([string]$title, [string]$user, [string]$url, [string]$notes, [string]$password = '') {
    Click-Name -Name 'Nueva entrada'
    [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 300
    $focus0 = Focused-Desc
    # El foco ya esta en Titulo: se teclea sin hacer clic.
    Type-Text $title
    if ($user) { Fill-Id 'entry-dialog-username' $user }
    $genVisible = $false; $genLen = 0
    if ($password) { Fill-Id 'entry-dialog-password' $password -Secret }
    else {
        Click-Name -Name 'Generar'; Start-Sleep -Milliseconds 500
        $pw = Find-El -Id 'entry-dialog-password' -Timeout 3
        if (-not $pw.Current.IsPassword) { $v = Get-Value $pw; if ($v) { $genVisible = $true; $genLen = $v.Length; Add-Generated $v } }
    }
    if ($url) { Fill-Id 'entry-dialog-url' $url }
    if ($notes) { Fill-Id 'entry-dialog-notes' $notes }
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    $btn = Find-El -Name 'Guardar entrada' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $closeMs = -1; $diskMs = -1
    while ($sw.Elapsed.TotalSeconds -lt 20 -and ($closeMs -lt 0 -or $diskMs -lt 0)) {
        if ($closeMs -lt 0 -and -not (Dialog-Open 'Nueva entrada')) { $closeMs = [int]$sw.ElapsedMilliseconds }
        if ($diskMs -lt 0 -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $diskMs = [int]$sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 40
    }
    Start-Sleep -Milliseconds 500
    return [pscustomobject]@{ Closed = ($closeMs -ge 0); CloseMs = $closeMs; DiskMs = $diskMs; GenVisible = $genVisible; GenLen = $genLen; Focus0 = $focus0; FocusAfter = (Focused-Desc) }
}
