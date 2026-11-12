# A (2/4): nueva entrada con contrasena GENERADA -> guardar (cronometro) -> esta en la lista y en disco.
$vault = Join-Path $Work 'qa-uno.kdbx'
Step 'A5 Formulario de entrada nueva' {
    Click-Name -Name 'Nueva entrada'
    Check 'Se abre el formulario "Nueva entrada en qa-uno"' ((Wait-Text 'Nueva entrada en qa-uno' 5) -ge 0)
    Fill-Id 'entry-title' $Canary.Title
    Fill-Id 'entry-username' $Canary.User
    Fill-Id 'entry-url' $Canary.Url
    Fill-Id 'entry-notes' $Canary.Notes
    Fill-Id 'entry-tags' $Canary.Tag
    Dump-Tree 'A08-formulario-entrada.uia.txt' | Out-Null
}
Step 'A6 Generador' {
    Click-Name -Name 'Generar contraseña'
    Check 'Aparece el generador' ((Wait-Text 'Generador de contraseñas' 5) -ge 0)
    Click-Name -Name 'Generar'
    $use = Find-El -Name 'Usar esta contraseña' -Type Button -Timeout 5
    $strength = (Get-Names | Where-Object { $_ -like '* de 4)*' -or $_ -like 'Fortaleza:*' -or $_ -match '^(muy |)(débil|buena|aceptable|fuerte)' }) -join ' '
    Log "   generador: $strength"
    $len = Find-El -Id 'gen-length' -Timeout 2 -Quiet
    if ($len) { Log "   longitud por defecto: $(Get-Value $len)" }
    Shot 'A08-formulario-con-generador'
    Click-El $use
    Start-Sleep -Milliseconds 400
    Get-Names (Find-El -Id 'entry-strength' -Timeout 2) | ForEach-Object { Log "     fortaleza en el formulario: $_" }
    Shot 'A09-formulario-contrasena-aplicada'
}
Step 'A7 Guardar (cronometro)' {
    $before = (Get-Item $vault).Length
    $btn = Find-El -Name 'Guardar entrada' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $ms = Wait-Text 'Entrada guardada.' 15
    $total = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO guardar entrada (clic -> 'Entrada guardada.'): $total ms"
    Check 'Aparece "Entrada guardada."' ($ms -ge 0) "$total ms"
    Shot 'A10-entrada-guardada'
    Dump-Tree 'A10-entrada-guardada.uia.txt' | Out-Null
    Show-Names
    Check 'La entrada esta en la lista' (Has-El -Name $Canary.Title -Type Button -Timeout 3)
    Check 'El grupo cuenta 1 entrada' (Has-El -Name 'qa-uno · 1 entrada' -Type Button -Timeout 3)
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length)" }
    Check 'El archivo crecio en disco' ((Get-Item $vault).Length -gt $before) "$before -> $((Get-Item $vault).Length) bytes"
    Check 'RF-22: hay .kdbx.bak junto a la boveda' (Test-Path "$vault.bak")
    Check 'No quedan .tmp' (@(Get-ChildItem $Work -Filter '*.tmp*').Count -eq 0)
}
Summary
