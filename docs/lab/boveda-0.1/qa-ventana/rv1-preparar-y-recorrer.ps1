# Reverificacion (1/3): guardar la evidencia anterior como control positivo, borrar SOLO EBWebView, instalar la
# compilacion con la webview en modo privado y repetir lo minimo que antes dejo rastro, con canarios nuevos.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rv-lib.ps1'), [Text.Encoding]::UTF8)))
$setup = Join-Path $Repo 'target\debug\bundle\nsis\AlmarioKS_0.0.2_x64-setup.exe'
$built = Join-Path $Repo 'target\debug\almarioks.exe'
Step 'RV1 Lo que habia y control positivo sobre la evidencia anterior' {
    if ((Get-AppProc).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    New-Item -ItemType Directory -Force $OldCopy | Out-Null
    (Get-Date).ToString('o') | Set-Content (Join-Path $Work 'inicio.txt')
    Log "Carpeta de datos: $((Get-ChildItem $DataDir -Force | ForEach-Object { $_.Name }) -join ', ')"
    $files = @(Get-ChildItem $EbDir -Recurse -File -Force)
    Log ("EBWebView antes de borrar: {0} archivos, {1:n1} MB" -f $files.Count, (($files | Measure-Object Length -Sum).Sum / 1MB))
    $wd = Join-Path $EbDir 'Default\Web Data'
    Check 'Antes de borrar existe EBWebView\Default\Web Data' (Test-Path $wd)
    Copy-Item -LiteralPath $wd (Join-Path $OldCopy 'Web Data')
    $lines = Run-Inspector (Rv-OldSecrets) @("evidencia-anterior=$OldCopy")
    $hashHits = @($lines | Where-Object { $_ -like 'COINCIDE_HASH|*' }).Count
    Check 'CONTROL POSITIVO: sobre el Web Data anterior el inspector ve los canarios viejos en claro' ((Hits-Of $lines 'evidencia-anterior') -ge 3) "apariciones: $(Hits-Of $lines 'evidencia-anterior')"
    Check 'CONTROL POSITIVO: y ve las filas enmascaradas que son el SHA-1 de 8 bytes de lo tecleado como maestra' ($hashHits -ge 2) "filas que coinciden: $hashHits"
    if ($EbDir -notlike '*\io.github.kattonavi.almarioks\EBWebView') { throw 'Ruta inesperada: no se borra nada' }
    Remove-Item -LiteralPath $EbDir -Recurse -Force
    Check 'EBWebView borrada (solo esa carpeta)' (-not (Test-Path $EbDir)) "queda en la carpeta de datos: $((Get-ChildItem $DataDir -Force | ForEach-Object { $_.Name }) -join ', ')"
}
Step 'RV2 Instalar la compilacion nueva' {
    $conf = Get-Content -Raw (Join-Path $Repo 'src-tauri\tauri.conf.json')
    Check 'tauri.conf.json del repo trae "incognito": true' ($conf -match '"incognito"\s*:\s*true')
    Log "Instalador: $((Get-Item $setup).Length) bytes, $((Get-Item $setup).LastWriteTime.ToString('yyyy-MM-dd HH:mm')), sha256 $(Sha $setup)"
    $p = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
    Check 'Instalador /S termina con codigo 0' ($p.ExitCode -eq 0)
    $latin = [Text.Encoding]::GetEncoding(28591)
    $a = $latin.GetString([IO.File]::ReadAllBytes($AppExe))
    $b = $latin.GetString([IO.File]::ReadAllBytes($built)).Replace('__TAURI_BUNDLE_TYPE_VAR_UNK', '__TAURI_BUNDLE_TYPE_VAR_NSS')
    Log "Instalado: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe); compilado: $((Get-Item $built).LastWriteTime.ToString('yyyy-MM-dd HH:mm')), sha256 $(Sha $built)"
    Check 'El exe instalado es el de target\debug (igual salvo la marca del empaquetado NSIS)' ([string]::Equals($a, $b, [StringComparison]::Ordinal))
    Check 'El exe instalado ya no es el de la primera corrida (sha256 595A406B7B27C295)' ((Sha $AppExe) -ne '595A406B7B27C295')
}
Step 'RV3 Crear la boveda tecleando la maestra y su repeticion' {
    Start-App
    $ms = Wait-Text 'Bienvenida' 30
    Check 'La app abre en la bienvenida' ($ms -ge 0) "ventana a los $LastStartMs ms; bienvenida legible $ms ms despues"
    Shot 'RV01-bienvenida'
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $RvVault
    Fill-Id 'create-password' $RvMaster -Secret
    Fill-Id 'create-confirm' $RvOther -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 800
    $msg = @(Get-Names (Find-El -Id 'create-error' -Timeout 2)) -join ' '
    Check 'Repeticion distinta tecleada y rechazada (antes dejaba su hash)' ($msg -like '*no coinciden*') $msg
    Fill-Id 'create-password' $RvMaster -Secret
    Fill-Id 'create-confirm' $RvMaster -Secret
    Click-Name -Name 'Crear bóveda'
    Check 'Boveda creada y abierta' ($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 60 -Quiet))
}
Step 'RV4 Entrada, grupo, busqueda, ajustes, bloquear y desbloquear' {
    $script:Vault = $RvVault
    $e = New-Entry 'rv-uno' $Rv.Title $Rv.Pass $Rv.User $Rv.Url $Rv.Notes $Rv.Tag
    Check 'Entrada con titulo, usuario, URL, notas y etiquetas canario' ($e.Msg -ge 0 -and $e.Saved -ge 0)
    Check 'Grupo con nombre canario' (New-Group 'rv-uno' $Rv.Group)
    Fill (Find-El -Id 'vault-search') 'centinela-rv-tit'
    Start-Sleep -Milliseconds 700
    Check 'Busqueda con resultado' (Has-El -Name $Rv.Title -Type Button -Timeout 3)
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    Fill (Find-El -Id 'vault-search') $Rv.Search
    Start-Sleep -Milliseconds 700
    Press 'ENTER'; Start-Sleep -Milliseconds 400
    Press 'ESC'; Start-Sleep -Milliseconds 400
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Check 'Ajustes: se elige 15 minutos' (Select-Option 'settings-auto-lock' '15 minutos')
    Fill-Id 'settings-clipboard' '30'
    Click-Name -Name 'Guardar ajustes'
    Check 'Ajustes guardados (15 minutos, 30 s)' ((Wait-Text 'Ajustes guardados.' 5) -ge 0)
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 400
    Open-Entry 'rv-uno' $Rv.Title
    Shot 'RV02-boveda-con-detalle'
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Fill-Id 'unlock-password' $RvMaster -Secret
    Click-Name -Name 'Desbloquear'
    Check 'Bloquear y desbloquear con la maestra nueva' ($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 20 -Quiet))
    Check 'La entrada sigue ahi' (Has-El -Name $Rv.Title -Type Button -Timeout 4)
    Start-Sleep -Seconds 3
    $closed = Close-App 20
    Check 'App cerrada con cierre normal' $closed
    Start-Sleep -Seconds 2
}
Summary
