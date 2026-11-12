# Reverificacion (3/3): el modo privado no rompio nada visible (recientes y ajustes tras reiniciar), segunda
# inspeccion del disco tras el segundo cierre, y limpieza.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rv-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'RV8 Tras reiniciar: recientes y ajustes' -Continue {
    Start-App
    $ms = Wait-Text 'Bienvenida' 30
    Log "   segundo arranque: ventana a los $LastStartMs ms; bienvenida legible $ms ms despues"
    Check 'La boveda reciente aparece tras reiniciar (RF-19)' (Has-El -Name 'Abrir rv-uno.kdbx' -Type Button -Timeout 5)
    Shot 'RV03-bienvenida-con-reciente'
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 500
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Los ajustes se conservan tras reiniciar (15 minutos, 30 s)' ($lock -eq '15 minutos' -and $clip -eq '30') "$lock / $clip"
    Shot 'RV04-ajustes'
    [void](Select-Option 'settings-auto-lock' '5 minutos')
    Fill-Id 'settings-clipboard' '20'
    Click-Name -Name 'Guardar ajustes'
    Check 'Ajustes devueltos a 5 minutos y 20 s' ((Wait-Text 'Ajustes guardados.' 5) -ge 0)
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    Click-Name -Name 'Abrir rv-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Fill-Id 'unlock-password' $RvMaster -Secret
    Click-Name -Name 'Desbloquear'
    Check 'La boveda abre tras reiniciar y la entrada y el grupo estan' (($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 20 -Quiet)) -and (Has-El -Name $Rv.Title -Type Button -Timeout 4) -and (Has-El -Like "$($Rv.Group) · *" -Type Button -Timeout 2))
    Open-Entry 'rv-uno' $Rv.Title
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    Check 'Copiar sigue funcionando (portapapeles con la contrasena de la entrada)' ((Get-Clip) -ceq $Rv.Pass)
    Shot 'RV05-boveda-tras-reiniciar'
    Show-Net 'modo privado, boveda abierta'
    $closed = Close-App 20
    Start-Sleep -Seconds 4
    Check 'Cierre normal y portapapeles vacio' ($closed -and (Get-Clip) -eq '')
}
Step 'RV9 Segunda inspeccion del perfil, tras el segundo cierre' -Continue {
    $lines = Run-Inspector (Rv-NewSecrets) @("webview2=$DataDir")
    $rows = 0; foreach ($l in @($lines | Where-Object { $_ -like 'SQLITE|*filas autofill=*' })) { if ($l -match 'filas autofill=(\d+)') { $rows += [int]$Matches[1] } }
    Check 'Segunda pasada: 0 apariciones y 0 filas en tablas autofill' ((Hits-Of $lines 'webview2') -eq 0 -and $rows -eq 0 -and @($lines | Where-Object { $_ -like 'COINCIDE_HASH|*' }).Count -eq 0) "apariciones $(Hits-Of $lines 'webview2'), filas $rows"
}
Step 'RV10 Limpieza' {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20) }
    if ($Work -notlike (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa-*')) { throw 'La carpeta de trabajo no es la esperada' }
    Remove-Item -LiteralPath $Work -Recurse -Force
    Start-App
    [void](Find-El -Id 'welcome-title' -Timeout 20)
    Start-Sleep -Milliseconds 600
    if (Has-El -Name 'Quitar rv-uno.kdbx de la lista' -Type Button -Timeout 3) { Click-Name -Name 'Quitar rv-uno.kdbx de la lista'; Start-Sleep -Milliseconds 500 }
    Check 'Recientes de prueba quitadas desde la app' (Has-El -Like 'Todavía no has abierto ninguna bóveda*' -Timeout 3)
    $closed = Close-App 20
    Start-Sleep -Milliseconds 800
    Remove-Item -LiteralPath $Work -Recurse -Force
    Check 'App cerrada y carpeta de trabajo borrada' ($closed -and -not (Test-Path $Work))
}
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Log "Estado final: procesos almarioks = $((Get-AppProc).Count); carpeta de trabajo existe = $(Test-Path $Work); settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
Log "Instalado: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe); portapapeles: $((Get-Clip).Length) caracteres; EBWebView existe = $(Test-Path $EbDir)"
Summary
