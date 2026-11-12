# A (1/4): bienvenida -> crear boveda (ruta por dialogo nativo, contrasena con medidor, sin archivo llave) -> boveda abierta y vacia.
$vault = Join-Path $Work 'qa-uno.kdbx'
Step 'A1 Bienvenida -> Crear boveda' {
    if (Has-El -Id 'create-title' -Timeout 1) { Log '   (ya en Crear boveda)'; return }
    Check 'Bienvenida visible' (Has-El -Name 'Bienvenida' -Type Text -Timeout 5)
    Click-Name -Name 'Crear bóveda nueva'
    Check 'Pantalla "Crear bóveda"' (Has-El -Id 'create-title' -Timeout 5)
    Shot 'A02-crear-vacia'
    Dump-Tree 'A02-crear-vacia.uia.txt' | Out-Null
}
Step 'A2 Elegir ruta con el dialogo nativo' {
    if ((Find-FileDialog 0.3) -eq [IntPtr]::Zero) { Click-Name -Name 'Elegir dónde guardarla…' } else { Log '   (el dialogo ya estaba abierto de la corrida anterior)' }
    Answer-FileDialog -path $vault -shot 'A03-dialogo-nativo-guardar'
    $ms = Wait-Text "*qa-uno.kdbx" 5
    Check 'La ruta elegida se muestra en el formulario' ($ms -ge 0)
    Check 'El dialogo de guardar no crea el archivo' (-not (Test-Path $vault))
}
Step 'A3 Contrasena maestra con medidor (RF-7)' {
    Fill-Id 'create-password' '123456' -Secret
    Start-Sleep -Milliseconds 600
    $m = Find-El -Name 'Fortaleza' -Timeout 3 -Quiet
    $weak = (Get-Names | Where-Object { $_ -like '*contraseña débil*' })
    Log "   con '123456': medidor=$(if ($m) { Get-Value $m } else { '<no hay>' }) aviso=`"$weak`""
    Get-Names (Find-El -Id 'create-strength') | ForEach-Object { Log "     fortaleza: $_" }
    Check 'RF-7: 123456 avisa que es debil' ([bool]$weak)
    Shot 'A04-crear-contrasena-debil'
    Fill-Id 'create-password' $Master -Secret
    Start-Sleep -Milliseconds 600
    Get-Names (Find-El -Id 'create-strength') | ForEach-Object { Log "     fortaleza (maestra de prueba): $_" }
    Fill-Id 'create-confirm' $Master -Secret
    Shot 'A05-crear-lista'
}
Step 'A4 Crear (cronometro)' {
    $btn = Find-El -Name 'Crear bóveda' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $sawProgress = $false; $done = -1
    while ($sw.Elapsed.TotalSeconds -lt 60) {
        $names = Get-Names
        if (-not $sawProgress -and ($names | Where-Object { $_ -like 'Creando la bóveda*' })) { $sawProgress = $true; $tp = $sw.ElapsedMilliseconds; Shot 'A06-crear-derivando' }
        if ($names -contains 'Bloquear' -or $names -contains 'Bóveda creada') { $done = $sw.ElapsedMilliseconds; break }
        if ($names | Where-Object { $_ -like '*no se pudo*' -or $_ -like '*No se pudo*' }) { break }
        Start-Sleep -Milliseconds 30
    }
    Log "TIEMPO crear boveda (clic -> boveda abierta): $done ms; progreso 'derivando' visto = $sawProgress"
    Check 'La boveda se crea y se abre' ($done -ge 0) "$done ms"
    Check 'RF-18: se ve el progreso mientras deriva' $sawProgress
    Shot 'A07-boveda-recien-creada'
    Dump-Tree 'A07-boveda-recien-creada.uia.txt' | Out-Null
    Show-Names
    Check 'El archivo .kdbx existe en disco' (Test-Path $vault) "$(if (Test-Path $vault) { (Get-Item $vault).Length } else { 0 }) bytes"
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length)" }
    Show-Net 'tras crear'
}
Summary
