# E: bloqueo automatico a 1 minuto desde Ajustes (tiempo real), barrido UIA, cierre con contrasena copiada (RF-75),
# persistencia en disco (D), reciente tras reabrir (RF-19) e instancia unica (RF-81).
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
$all = @($Canary.Title, $Canary.User, $Canary.Url, $Canary.Notes, $Canary.Tag, $Canary.Pass2, 'CANARIO')
function Sweep([string]$file) {
    $text = (Dump-Tree $file) -join ' ## '
    return @($all | Where-Object { $text.Contains($_) })
}
function Settings-Json {
    if (-not (Test-Path $SettingsFile)) { return $null }
    return (Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json)
}
Step 'E1 Ajustes: bloqueo automatico a 1 minuto' {
    Click-Name -Name 'Ajustes y Acerca de'
    Check 'Se abre "Ajustes"' (Has-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 500
    Log "   valores al abrir: bloqueo = $(Get-Value (Find-El -Id 'settings-auto-lock')); portapapeles = $(Get-Value (Find-El -Id 'settings-clipboard')) s"
    Shot 'G06-ajustes-800x600'
    Dump-Tree 'G06-ajustes.uia.txt' | Out-Null
    $picked = Select-Option 'settings-auto-lock' '1 minuto'
    Log "   opciones de bloqueo: $($LastOptions -join ' | ')"
    Check 'Se puede elegir "1 minuto"' $picked
    Click-Name -Name 'Guardar ajustes'
    $ms = Wait-Text 'Ajustes guardados.' 5
    Check 'Ajustes: "Ajustes guardados."' ($ms -ge 0)
    $s = Settings-Json
    Check 'settings.json guarda auto_lock_minutes = 1' ($null -ne $s -and $s.auto_lock_minutes -eq 1) "claves: $(($s.PSObject.Properties.Name) -join ', ')"
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 400
}
Step 'E2 Bloqueo automatico sin tocar la ventana (RF-29)' {
    Open-Entry 'Personal QA' $Canary.Title
    Click-Name -Name 'Mostrar contraseña'     # un valor revelado en pantalla: tambien debe desaparecer
    Start-Sleep -Milliseconds 400
    Check 'Control positivo: abierta y con la contrasena mostrada, el barrido ve los canarios' ((Sweep '').Count -ge 6)
    $i = Get-WinInfo
    [void][QA.Native]::SetCursorPos($i.X + $i.OuterW + 300, $i.Y + $i.OuterH + 100)   # fuera de la ventana
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Log "   ultima entrada real: ahora. Cuenta en la ventana: $((Get-Names | Where-Object { $_ -match '^\d+:\d\d$' }) -join '')"
    $locked = -1; $lastCount = ''
    while ($sw.Elapsed.TotalSeconds -lt 100) {
        if (Has-El -Name 'Desbloquear' -Type Button -Timeout 0.1) { $locked = $sw.Elapsed.TotalSeconds; break }
        $count = (Get-Names | Where-Object { $_ -match '^\d+:\d\d$' }) -join ''
        if ($count -and $count -ne $lastCount -and ([int]$sw.Elapsed.TotalSeconds % 15) -eq 0) { Log ("   a los {0:n0} s la ventana dice: {1}" -f $sw.Elapsed.TotalSeconds, $count); $lastCount = $count }
        Start-Sleep -Milliseconds 400
    }
    Log ("TIEMPO bloqueo automatico: {0:n1} s desde la ultima entrada real (ajuste: 1 minuto)" -f $locked)
    Check 'RF-29: se bloquea sola al minuto' ($locked -ge 48 -and $locked -le 66) ("{0:n1} s" -f $locked)
    Start-Sleep -Milliseconds 1500
    $hits = Sweep 'E02-bloqueo-automatico.uia.txt'
    Check 'RF-32: tras el bloqueo automatico el arbol UIA no expone canarios ni la contrasena mostrada' ($hits.Count -eq 0) ($hits -join ', ')
    Shot 'E02-bloqueo-automatico'
}
Step 'E3 Cerrar la app con una contrasena recien copiada (RF-75)' {
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Open-Entry 'Personal QA' $Canary.Title
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 500
    Check 'Antes de cerrar: el portapapeles tiene la contrasena' ((Get-Clip) -ceq $Canary.Pass2)
    $closed = Close-App 20
    Start-Sleep -Milliseconds 500
    $clip = Get-Clip
    Check 'La app cierra con el cierre normal de ventana' $closed
    Check 'RF-75: al cerrar la app el portapapeles queda vacio' ($clip -eq '') "longitud $($clip.Length)"
}
Step 'D1 Persistencia en disco tras cerrar' {
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length) bytes" }
    Check 'El .kdbx existe' (Test-Path $Vault)
    Check 'Hay un .kdbx.bak junto a el (RF-22)' (Test-Path "$Vault.bak")
    Check 'No quedan .tmp ni archivos de recuperacion' (@(Get-ChildItem $Work | Where-Object { $_.Name -match '\.tmp|recuperacion' }).Count -eq 0)
    $head = [IO.File]::ReadAllBytes($Vault)[0..11]
    Log "   cabecera: $(($head | ForEach-Object { $_.ToString('X2') }) -join ' ')"
    Check 'La cabecera es KDBX 4 (firma 03D9A29A 67FB4BB5, version mayor 4)' ($head[0] -eq 0x03 -and $head[1] -eq 0xD9 -and $head[4] -eq 0x67 -and $head[10] -eq 0x04)
    Get-ChildItem $Work -Filter 'app-*.log' | ForEach-Object { Log "  salida del proceso: $($_.Name) $($_.Length) bytes" }
}
Step 'E4 Reabrir: reciente (RF-19), desbloquear, instancia unica (RF-81)' {
    Start-App
    $ms = Wait-Text 'Bienvenida' 20
    Log "   arranque: ventana a los $LastStartMs ms; bienvenida legible $ms ms despues"
    Check 'RF-19: la bienvenida ofrece "Abrir qa-uno.kdbx"' (Has-El -Name 'Abrir qa-uno.kdbx' -Type Button -Timeout 5)
    Shot 'G01-bienvenida-con-reciente-800x600'
    $before = (Get-AppProc).Count
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $AppExe; $psi.UseShellExecute = $true; $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    $second = [System.Diagnostics.Process]::Start($psi)
    Start-Sleep -Seconds 4
    $procs = (Get-AppProc).Count
    $wins = 0
    foreach ($p in Get-AppProc) { foreach ($h in [QA.Native]::WindowsOfPid([uint32]$p.Id)) { if ([QA.Native]::ClassOf($h) -eq 'Tauri Window') { $wins++ } } }
    Log "   procesos antes = $before; tras abrir otra vez = $procs; ventanas de Tauri = $wins; la segunda instancia termino = $($second.HasExited); ventana en primer plano = $([QA.Native]::GetForegroundWindow() -eq (Get-AppHwnd))"
    Check 'RF-81: al abrir dos veces queda un solo proceso y una sola ventana' ($procs -eq 1 -and $wins -eq 1)
    Click-Name -Name 'Abrir qa-uno.kdbx'
    Check 'Va a "Desbloquear qa-uno.kdbx"' (Has-El -Id 'unlock-title' -Timeout 5)
    Shot 'G02-desbloquear-800x600'
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Check 'Tras reiniciar la app, la entrada sigue en "Personal QA"' (Has-El -Name 'Personal QA · 1 entrada' -Type Button -Timeout 4)
    Log "   cuenta de bloqueo tras reabrir: $((Get-Names | Where-Object { $_ -match '^\d+:\d\d$' }) -join '') (el ajuste de 1 minuto persiste)"
    Show-Net 'tras reabrir'
}
Summary
