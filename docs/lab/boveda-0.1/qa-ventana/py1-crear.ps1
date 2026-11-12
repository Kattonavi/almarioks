# Ronda 2 (1): abre la app instalada, captura "Crear boveda" recien abierta con "Familia" (antes de cambiar la
# ubicacion), anota la opcion de llave marcada por defecto, crea la boveda de muestra "Cuentas" en la carpeta de
# trabajo (sin archivo llave) con sus grupos, y la primera entrada. RF_VARIANT=py-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'PY10 Arranque y Crear boveda' {
    if (Test-Path $RfVault) { throw 'La boveda de muestra ya existe: este paso no la pisa' }
    New-Item -ItemType Directory -Force $Work | Out-Null
    if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'crear' }
    Check 'Ventana a 1120 x 720' (Size-Window 1120 720)
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
    Fill-Id 'create-name' 'Familia'
    Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 900
    $radios = @(Get-All | Where-Object { (Type-Of $_) -eq 'RadioButton' } | ForEach-Object {
            $p = $null; $sel = '?'
            if ($_.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$p)) { $sel = $p.Current.IsSelected }
            "$($_.Current.Name) = $sel" })
    Log "   OPCIONES DE LLAVE al abrir: $($radios -join ' | ')"
    Check 'La opcion marcada por defecto es "Sin archivo llave"' (@($radios | Where-Object { $_ -eq 'Sin archivo llave = True' }).Count -eq 1) ($radios -join ' | ')
    Shot-Client 'app-crear'
    Print-Rects
}
Step 'PY11 Boveda "Cuentas" y grupos' {
    Fill-Id 'create-name' 'Cuentas'
    Set-Location-Py 'Cuentas.kdbx'
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-Name -Name 'Crear bóveda' -Type Button
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 45); Start-Sleep -Milliseconds 800
    Check 'Boveda creada en la carpeta de trabajo, sin archivo llave al lado' ((Test-Path $RfVault) -and @(Get-ChildItem $Work -Filter '*.keyx').Count -eq 0) "archivos: $((Get-ChildItem $Work | ForEach-Object { $_.Name }) -join ', ')"
    foreach ($g in @('Personal', 'Trabajo', 'Bancos', 'Compras')) {
        Click-Name -Name 'Nuevo grupo'; [void](Find-El -Id 'group-name' -Timeout 5); Start-Sleep -Milliseconds 250
        Type-Text $g; Click-Name -Name 'Crear grupo'
        [void](Find-El -Like "$g · * entrada*" -Type Button -Timeout 8); Start-Sleep -Milliseconds 2300
    }
    Click-Name -Name 'Más acciones de Trabajo'; Start-Sleep -Milliseconds 400
    Click-El (Find-El -Name 'Nuevo subgrupo…' -Type MenuItem); [void](Find-El -Id 'group-name' -Timeout 5); Start-Sleep -Milliseconds 250
    Type-Text 'Clientes'; Click-El (Find-El -Like 'Crear*' -Type Button)
    [void](Find-El -Like 'Clientes · * entrada*' -Type Button -Timeout 8); Start-Sleep -Milliseconds 2300
    $tree = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name -replace ' · .*', '' })
    Check 'Grupos en el orden del prototipo' (($tree -join '|') -eq 'Todas las entradas|Personal|Trabajo|Clientes|Bancos|Compras') ($tree -join ' | ')
}
Step 'PY12 Primera entrada (GitHub) y volcado de la lista' -Continue {
    Pick-Group 'Trabajo'
    $o = [char]0x00F3
    $r = New-EntryPx 'GitHub' 'ana@example.com' 'https://github.com' "Cuenta de la organizaci${o}n. Renovar el token de acceso en enero." $PyPass['GitHub']
    Log "   dialogo cerrado = $($r.Closed) a los $($r.CloseMs) ms; disco a los $($r.DiskMs) ms; foco despues: $($r.FocusAfter)"
    Print-Rects -All
}
Summary
