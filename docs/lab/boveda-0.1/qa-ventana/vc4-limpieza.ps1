# Cierre de la superficie de navegador (4/4): limpieza. Borra la carpeta de trabajo, quita la boveda de prueba de
# "recientes" desde la app, comprueba los ajustes y deja la app instalada y cerrada. RF_VARIANT=vc-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'VC14 Limpieza' {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 2 }
    if ($Work -ne (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa3-20261112')) { throw 'La carpeta de trabajo no es la esperada' }
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Start-App
    [void](Find-El -Id 'welcome-title' -Timeout 30)
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
    Shot 'VC40-bienvenida-final'
    $closed = Close-App 20
    Start-Sleep -Milliseconds 1500
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Check 'App cerrada y carpeta de trabajo borrada' ($closed -and -not (Test-Path $Work))
    if ((Get-Clip) -ne '') { [System.Windows.Forms.Clipboard]::Clear() }
}
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Log "Estado final: procesos almarioks = $((Get-AppProc).Count); carpeta de trabajo existe = $(Test-Path $Work); settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
Log "Instalado: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($AppExe).ProductVersion); portapapeles: $((Get-Clip).Length) caracteres; EBWebView existe = $(Test-Path $EbDir)"
$left = @(Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Temp') -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-qa3*' -or $_.Name -like 'almarioks-qa2*' })
Log "Restos de esta QA en %TEMP%: $($left.Count)"
Summary
