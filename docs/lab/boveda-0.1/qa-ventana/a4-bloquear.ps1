# A (4/4) y E (parte): bloquear -> la ventana no expone nada de la boveda -> llave mala -> desbloquear (cronometro) -> la entrada sigue.
$all = @($Canary.Title, $Canary.User, $Canary.Url, $Canary.Notes, $Canary.Tag)
function Sweep([string]$file) {
    $lines = Dump-Tree $file
    $text = $lines -join "`n"
    $hits = @($all | Where-Object { $text.Contains($_) })
    if ($text -match 'CANARIO') { $hits += '<la palabra CANARIO>' }
    if ($script:Pw -and $text.Contains($script:Pw)) { $hits += '<la contrasena generada>' }
    return [pscustomobject]@{ Lines = $lines.Count; Hits = $hits }
}
Step 'A13 Control positivo: con la boveda abierta el barrido SI ve los canarios' {
    Click-Name -Name $Canary.Title
    Start-Sleep -Milliseconds 500
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    $script:Pw = Get-Clip
    Check 'El portapapeles tiene la contrasena recien copiada' ($Pw.Length -eq 20)
    $s = Sweep 'E00-abierta.uia.txt'
    Log "   arbol abierto: $($s.Lines) nodos; canarios vistos: $($s.Hits.Count)"
    Check 'Control positivo: el barrido encuentra los 5 canarios con la boveda abierta' ($s.Hits.Count -ge 5) ($s.Hits -join ', ')
}
Step 'A14 Bloquear (cronometro) y barrido del arbol UIA' {
    $btn = Find-El -Name 'Bloquear' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $un = Find-El -Name 'Desbloquear' -Type Button -Timeout 10
    $ms = [int]$sw.ElapsedMilliseconds
    Log "TIEMPO bloquear (clic -> pantalla de desbloqueo): $ms ms (incluye ~200 ms de espera del clic)"
    Check 'RF-28: aparece la pantalla de desbloqueo' ($null -ne $un) "$ms ms"
    Start-Sleep -Milliseconds 600
    $clip = Get-Clip
    Check 'RF-31: al bloquear el portapapeles queda vacio' ($clip -eq '') "longitud $($clip.Length)"
    Start-Sleep -Milliseconds 1200   # la webview se recarga al bloquear: se barre el arbol ya asentado
    $s = Sweep 'E01-bloqueada.uia.txt'
    Log "   arbol bloqueado: $($s.Lines) nodos"
    Check 'RF-32: bloqueada, el arbol UIA no expone ningun canario ni la contrasena' ($s.Hits.Count -eq 0) ($s.Hits -join ', ')
    Shot 'A14-bloqueada'
    Show-Names
}
Step 'A15 Contrasena mala: un solo mensaje' {
    Fill-Id 'unlock-password' 'no-es-la-llave-QA' -Secret
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Desbloquear'
    $err = $null
    while ($sw.Elapsed.TotalSeconds -lt 15) {
        $err = Find-El -Id 'unlock-error' -Timeout 0.3 -Quiet
        if ($err -and $err.Current.Name) { break }
        $t = @(Get-Names $err) -join ''
        if ($t) { break }
        Start-Sleep -Milliseconds 50
    }
    $msg = @(Get-Names (Find-El -Id 'unlock-error')) -join ' '
    Log "   mensaje ($([int]$sw.ElapsedMilliseconds) ms): $msg"
    Check 'RF-11: llave mala da un mensaje de llave incorrecta y sigue bloqueada' ($msg -like '*incorrect*' -and (Has-El -Name 'Desbloquear' -Type Button)) $msg
    Shot 'A15-llave-incorrecta'
}
Step 'A16 Desbloquear con la contrasena (cronometro)' {
    Fill-Id 'unlock-password' $Master -Secret
    $btn = Find-El -Name 'Desbloquear' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $saw = $false; $done = -1
    while ($sw.Elapsed.TotalSeconds -lt 30) {
        $names = Get-Names
        if (-not $saw -and ($names | Where-Object { $_ -like 'Abriendo la bóveda*' })) { $saw = $true }
        if ($names -contains 'Bloquear') { $done = [int]$sw.ElapsedMilliseconds; break }
        Start-Sleep -Milliseconds 25
    }
    Log "TIEMPO desbloquear (clic -> boveda abierta): $done ms (incluye ~200 ms de espera del clic); progreso visto = $saw"
    Check 'RF-10: desbloquea con la contrasena maestra (con ñ y €)' ($done -ge 0) "$done ms"
    Check 'La entrada sigue ahi tras desbloquear' (Has-El -Name $Canary.Title -Type Button -Timeout 4)
    Shot 'A16-desbloqueada'
    Show-Net 'tras desbloquear'
}
Summary
