# R-1 y R-4, limpieza: borra la carpeta de trabajo, quita la boveda de prueba de "recientes" desde la app, comprueba
# los ajustes, deja el portapapeles y el historial sin canarios y la app instalada cerrada. RF_VARIANT=vn-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
# En vn3 la comprobacion "bloquear deja el portapapeles sin canarios" fallo: miraba un prefijo que comparten el titulo
# y la contrasena. Aqui se repite distinguiendo: que hay exactamente en el portapapeles tras bloquear cuando lo ultimo
# copiado fue un texto NO protegido por la via del navegador (Ctrl+C en un campo del formulario).
Step 'VN29 Que queda en el portapapeles tras bloquear, si lo ultimo copiado fue el titulo por Ctrl+C' -Continue {
    if (-not (Test-Path $RfVault)) { throw 'La boveda de prueba ya no existe' }
    Start-Checked 'comprobacion del portapapeles tras bloquear'
    Click-Name -Name 'Abrir rf-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    [void](Unlock-Timed 'comprobacion')
    Open-Entry $RootName $Rf.Title
    Click-Name -Name 'Editar'; [void](Wait-Text 'Editar *' 4)
    Set-ClipExternal 'centinela-vn-bloqueo'; Start-Sleep -Milliseconds 300
    Click-El (Find-El -Id 'entry-title'); Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $before = "$(Get-Clip)"; $fmt = @(Get-ClipFormats)
    for ($k = 0; $k -lt 3 -and (Has-El -Name 'Guardar entrada' -Type Button -Timeout 0.4); $k++) { Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 400 }
    Start-Sleep -Milliseconds 1500
    [void](Lock-Timed)
    $after = "$(Get-Clip)"
    $hist = History-Has $Rf.Title
    Log "PORTAPAPELES: antes de bloquear = el titulo exacto: $($before -ceq $Rf.Title); tras bloquear = el titulo exacto: $($after -ceq $Rf.Title); contiene la contrasena: $($after.Contains($Rf.Pass)); formatos de exclusion de esa copia: $((@($ExclFormats | Where-Object { $fmt -contains $_ })) -join ', '); el titulo en el historial: $hist"
    Check 'Tras bloquear, el portapapeles NO lleva la contrasena (lo que queda es el titulo copiado con Ctrl+C, que no es de la app)' (-not $after.Contains($Rf.Pass))
    Start-Sleep -Milliseconds 800
    [void](Close-App 20); Start-Sleep -Seconds 3
}
Step 'VN30 Limpieza' {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 2 }
    if ((Any-AlmarioProc).Count -gt 0) { throw "Quedan procesos almarioks* vivos: $((Any-AlmarioProc | ForEach-Object { $_.Name }) -join ', ')" }
    if ($Work -ne (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa4-20261112')) { throw 'La carpeta de trabajo no es la esperada' }
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Start-Checked 'limpieza'
    Start-Sleep -Milliseconds 800
    if (Has-El -Name 'Quitar rf-uno.kdbx de la lista' -Type Button -Timeout 3) { Click-Name -Name 'Quitar rf-uno.kdbx de la lista'; Start-Sleep -Milliseconds 600 }
    Check 'Recientes de prueba quitadas desde la app' (-not (Has-El -Like '*rf-uno.kdbx*' -Timeout 1))
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes en 5 minutos y 20 s' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    $closed = Close-App 20
    Start-Sleep -Milliseconds 1500
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    [System.Windows.Forms.Clipboard]::Clear()
    $n = Purge-Mine
    Check 'App cerrada, carpeta de trabajo borrada, ningun almarioks* vivo' ($closed -and -not (Test-Path $Work) -and (Any-AlmarioProc).Count -eq 0)
    Check 'Portapapeles vacio e historial sin canarios ni centinelas de esta prueba' ((Get-Clip) -eq '' -and (History-Has 'CENTELLA') -eq 'no' -and (History-Has 'centinela-vn') -eq 'no') "borrados ahora: $n"
}
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Log "Estado final: procesos almarioks* = $((Any-AlmarioProc).Count); carpeta de trabajo existe = $(Test-Path $Work); settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
Log "Instalado: $((Get-Item $InstalledExe).Length) bytes, sha256 $(Sha $InstalledExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($InstalledExe).ProductVersion); portapapeles: $("$(Get-Clip)".Length) caracteres"
$left = @(Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Temp') -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-qa4*' -or $_.Name -like 'almarioks-qa2-control*' })
Log "Restos de esta QA en %TEMP%: $($left.Count)"
Summary
