# Ayudas de la matriz de completitud (se cargan desde los pasos b* y siguientes).
$script:Vault = Join-Path $Work 'qa-uno.kdbx'
function Group-Btn([string]$name) { Find-El -Like "$name*·*entrada*" -Type Button -Timeout 5 }
function Select-Group([string]$name) { Click-El (Group-Btn $name); Start-Sleep -Milliseconds 350 }
# Los botones del arbol se llaman "Nombre · N entradas".
function Tree-Lines {
    @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name })
}
function Show-Tree([string]$when) { Log "   ARBOL [$when]: $((Tree-Lines) -join '  |  ')" }
# Mensajes de resultado (parrafos de estado), sin los nombres de los grupos ni de los botones.
function Status-Text {
    (Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -match 'Grupo creado|renombrado|se movió|Se borró|guardada\.|No había cambios|quedó vacía' } |
        ForEach-Object { $_.Current.Name } | Select-Object -Unique) -join ' | '
}
function Alert-Text {
    (Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -match 'No se p|no se p|no puede|No puede|necesita|Escribe|dentro de sí|sí mismo|ya existe|Ya existe|no coinciden|dañad|no abren|no es una|no está|No hay otro|no existe|No existe' } |
        ForEach-Object { $_.Current.Name } | Select-Object -Unique) -join ' | '
}
# Elige una opcion de un <select>. Primero con el raton (abrir y clic en la opcion). Si la lista desplegada no se deja
# manejar, con el teclado y la lista cerrada: Inicio y flecha abajo hasta que el valor coincida (queda anotado).
# Devuelve la lista de opciones vistas en $script:LastOptions.
function Select-Option([string]$comboId, [string]$like) {
    $script:LastOptions = @()
    $combo = Find-El -Id $comboId
    $liCond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::ListItem)
    Click-El $combo
    Start-Sleep -Milliseconds 500
    $opts = @((Find-El -Id $comboId).FindAll($TreeScope::Descendants, $liCond))
    $visible = @($opts | Where-Object { $r = $_.Current.BoundingRectangle; -not $r.IsEmpty -and $r.Width -gt 1 -and -not $_.Current.IsOffscreen })
    if ($opts.Count) { $script:LastOptions = @($opts | ForEach-Object { $_.Current.Name }) }
    $target = $visible | Where-Object { $_.Current.Name -like $like } | Select-Object -First 1
    if ($target) {
        $r = $target.Current.BoundingRectangle
        [QA.Native]::ClickAt([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)); Start-Sleep -Milliseconds 400
        Log "   opciones de $comboId (lista abierta con el raton): $($LastOptions -join ' | ')"
    } else {
        Press 'ESC'; Start-Sleep -Milliseconds 250
        (Find-El -Id $comboId).SetFocus(); Start-Sleep -Milliseconds 150
        Press 'HOME'; Start-Sleep -Milliseconds 150
        $seen = New-Object System.Collections.Generic.List[string]
        for ($k = 0; $k -lt 30; $k++) {
            $v = Get-Value (Find-El -Id $comboId)
            if ($seen.Count -and $seen[$seen.Count - 1] -eq $v) { break }
            $seen.Add($v)
            if ($v -like $like) { break }
            Press 'DOWN'; Start-Sleep -Milliseconds 150
        }
        $script:LastOptions = @($seen)
        Log "   ALTERNATIVA: la lista desplegada no expone opciones clicables por UIA; elegido con teclado. Valores recorridos: $($seen -join ' | ')"
    }
    $now = Get-Value (Find-El -Id $comboId)
    Log "   elegido en $comboId : $now"
    return ($now -like $like)
}
# Recorre TODAS las opciones de un <select> con el teclado (Inicio, flecha abajo) y las devuelve. Deja la primera.
function List-Options([string]$comboId) {
    (Find-El -Id $comboId).SetFocus(); Start-Sleep -Milliseconds 150
    Press 'HOME'; Start-Sleep -Milliseconds 150
    $seen = New-Object System.Collections.Generic.List[string]
    for ($k = 0; $k -lt 30; $k++) {
        $v = Get-Value (Find-El -Id $comboId)
        if ($seen.Count -and $seen[$seen.Count - 1] -eq $v) { break }
        $seen.Add($v); Press 'DOWN'; Start-Sleep -Milliseconds 150
    }
    Press 'HOME'; Start-Sleep -Milliseconds 150
    return @($seen)
}
function New-Group([string]$parent, [string]$name) {
    Select-Group $parent
    Click-Name -Name 'Nuevo grupo…'
    Fill-Id 'group-name' $name
    Click-Name -Name 'Crear grupo'
    Start-Sleep -Milliseconds 700
    return (Has-El -Like "$name · * entrada*" -Type Button -Timeout 4)
}
# Crea una entrada en el grupo dado. Devuelve los tres tiempos: mensaje de la entrada, archivo reemplazado, "Guardado.".
function New-Entry([string]$group, [string]$title, [string]$password, [string]$user = '', [string]$url = '', [string]$notes = '', [string]$tags = '') {
    Select-Group $group
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text "Nueva entrada en *" 5)
    $heading = (Get-Names | Where-Object { $_ -like 'Nueva entrada en *' } | Select-Object -First 1)
    Fill-Id 'entry-title' $title
    if ($user) { Fill-Id 'entry-username' $user }
    Fill-Id 'entry-password' $password -Secret
    if ($url) { Fill-Id 'entry-url' $url }
    if ($notes) { Fill-Id 'entry-notes' $notes }
    if ($tags) { Fill-Id 'entry-tags' $tags }
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    $btn = Find-El -Name 'Guardar entrada' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $tMsg = -1; $tDisk = -1; $tSaved = -1; $sawSaving = $false
    while ($sw.Elapsed.TotalSeconds -lt 20 -and ($tMsg -lt 0 -or $tSaved -lt 0)) {
        $names = Get-Names
        if ($tMsg -lt 0 -and $names -contains 'Entrada guardada.') { $tMsg = [int]$sw.ElapsedMilliseconds }
        if ($names -contains 'Guardando…') { $sawSaving = $true }
        if ($tDisk -lt 0 -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $tDisk = [int]$sw.ElapsedMilliseconds }
        if ($tDisk -ge 0 -and $names -contains 'Guardado.') { $tSaved = [int]$sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 20
    }
    Log "   entrada `"$title`" ($heading): 'Entrada guardada.' a los $tMsg ms; archivo reemplazado a los $tDisk ms; 'Guardado.' a los $tSaved ms; se vio 'Guardando…' = $sawSaving"
    return [pscustomobject]@{ Msg = $tMsg; Disk = $tDisk; Saved = $tSaved; Heading = $heading }
}
function Open-Entry([string]$group, [string]$title) { Select-Group $group; Click-Name -Name $title; Start-Sleep -Milliseconds 400 }
function Relock-Unlock {
    Start-Sleep -Milliseconds 2500   # deja terminar el ultimo guardado
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Start-Sleep -Milliseconds 600
}
