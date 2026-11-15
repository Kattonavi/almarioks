# Decisiones del dueno (2): por la interfaz, boveda de prueba "Decisiones" en la carpeta temporal propia ("Cambiar
# ubicación…", "Sin archivo llave") con dos entradas: una con contrasena corta tecleada y otra con una de 500
# generada en el dialogo (como pzA-generador.ps1). La de 500 se guarda en el estado, no se imprime. RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$ExactId = 'entry-dialog-password-length-exact'
Step 'PD20 Boveda de prueba' {
    if (Test-Path -LiteralPath $Vault) { throw 'La boveda de prueba ya existe: no se pisa' }
    [void](To-Welcome); Check 'Ventana a 1120 x 720' (Size-Window 1120 720)
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
    $r = Radios
    Fill-Id 'create-name' $PdName
    Pick-Dialog 'Cambiar ubicación…' $Vault
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-Name -Name 'Crear bóveda' -Type Button
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 45 -Quiet); Start-Sleep -Milliseconds 800
    Check 'Boveda creada en la carpeta temporal, sin archivo llave' ($ok -and (Test-Path -LiteralPath $Vault) -and @(Get-ChildItem -LiteralPath $Pz -Filter '*.keyx').Count -eq 0) "opciones: $($r -join ' | '); archivos: $((Get-ChildItem -LiteralPath $Pz | ForEach-Object { $_.Name }) -join ', ')"
}
Step 'PD21 Entrada corta' {
    $e = New-EntryPy $PdShort 'ana.pd' '' 'Notas de la corta' $PdShortPass
    Check 'Entrada con contrasena corta creada' ($e.Closed -and $e.Disk -and (Has-Entry $PdShort)) "cerrado=$($e.Closed) disco=$($e.Disk)"
}
Step 'PD22 Entrada con 500 generada' {
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    Type-Text $PdLong
    Fill-Id 'entry-dialog-username' 'FAROPD-usuario'
    Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 500
    Fill-Id $ExactId '500'; Start-Sleep -Milliseconds 300
    Click-El (Find-El -Name 'Generar' -Type Button); Start-Sleep -Milliseconds 700
    $pw = Find-El -Id 'entry-dialog-password' -Timeout 3
    if ($pw.Current.IsPassword) { Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Find-El -Name 'Nueva entrada' -Type Window)); Start-Sleep -Milliseconds 400; $pw = Find-El -Id 'entry-dialog-password' -Timeout 3 }
    $v = "$(Raw-Value $pw)"
    Check 'Generar con 500 llena el campo con 500' ($v.Length -eq 500) "longitud $($v.Length)"
    Set-State 'pd-500' $v; Add-Generated $v
    Fill-Id 'entry-dialog-notes' 'Notas de la de quinientos'
    $stamp0 = (Get-Item -LiteralPath $Vault).LastWriteTimeUtc
    Click-El (Find-El -Name 'Guardar entrada' -Type Button)
    $sw = [Diagnostics.Stopwatch]::StartNew(); $closed = $false; $disk = $false
    while ($sw.Elapsed.TotalSeconds -lt 20 -and -not ($closed -and $disk)) {
        if (-not $closed -and -not (Dialog-Open 'Nueva entrada')) { $closed = $true }
        if (-not $disk -and (Get-Item -LiteralPath $Vault).LastWriteTimeUtc -ne $stamp0) { $disk = $true }
        Start-Sleep -Milliseconds 60
    }
    Start-Sleep -Milliseconds 700
    Check 'Entrada de 500 guardada' ($closed -and $disk -and (Has-Entry $PdLong)) "cerrado=$closed disco=$disk"
}
Summary
