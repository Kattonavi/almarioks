# Paso 1: cerrar la instancia vieja con cierre normal (sin leer su ventana), instalar en silencio y comprobar.
$setup = Join-Path $Repo 'target\debug\bundle\nsis\AlmarioKS_0.0.2_x64-setup.exe'
$built = Join-Path $Repo 'target\debug\almarioks.exe'
New-Item -ItemType Directory -Force $Work | Out-Null
(Get-Date).ToString('o') | Set-Content (Join-Path $Work 'inicio.txt')

Step 'Cerrar la instancia que estaba abierta' {
    if ((Get-AppProc).Count -gt 0) {
        $closed = Close-App 20
        Check 'La instancia anterior cerro con WM_CLOSE (sin matar el proceso)' $closed
    } else { Log 'No habia instancia abierta' }
}
Step 'Instalar en silencio' {
    Log "Instalador: $((Get-Item $setup).Length) bytes, sha256 $(Sha $setup)"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
    Log "Instalador /S: codigo de salida = $($p.ExitCode) en $([int]$sw.Elapsed.TotalSeconds) s"
    Check 'Instalador /S termina con codigo 0' ($p.ExitCode -eq 0)
    Get-ChildItem $InstallDir | ForEach-Object { Log "  $($_.Name) ($($_.Length) bytes, $($_.LastWriteTime.ToString('HH:mm')))" }
    $same = (Sha $AppExe) -eq (Sha $built)
    Check 'El exe instalado es el compilado en target\debug (mismo SHA-256)' $same "instalado $(Sha $AppExe) / compilado $(Sha $built)"
    $hkcu = @(Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -like '*lmario*' })
    foreach ($k in $hkcu) { Log "HKCU: $($k.DisplayName) | $($k.DisplayVersion) | $($k.Publisher)" }
    Check 'Clave de desinstalacion en HKCU' ($hkcu.Count -eq 1)
}
Step 'Nada en HKLM' {
    $hklm = @(@('HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*', 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*') |
        ForEach-Object { Get-ItemProperty $_ -ErrorAction SilentlyContinue } | Where-Object { $_.DisplayName -like '*lmario*' })
    Check 'HKLM: 0 claves de desinstalacion de AlmarioKS' ($hklm.Count -eq 0) "encontradas: $($hklm.Count)"
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $out = & reg.exe query 'HKLM\SOFTWARE' /s /k /f 'almarioks' 2>&1 | Out-String
    $keys = @($out -split "`r?`n" | Where-Object { $_ -match '^HKEY_' })
    Log "reg query HKLM\SOFTWARE /s /k /f almarioks ($([int]$sw.Elapsed.TotalSeconds) s): $($keys.Count) claves"
    foreach ($k in $keys) { Log "  $k" }
    Check 'HKLM\SOFTWARE: ninguna clave con "almarioks" en el nombre' ($keys.Count -eq 0)
}
Step 'Primer arranque' {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-App | Out-Null
    $ms = Wait-Text 'Bienvenida' 20
    Log "Arranque hasta 'Bienvenida' legible por UIA: $([int]$sw.ElapsedMilliseconds) ms (incluye el primer enlace de UIA)"
    Check 'La app abre en la bienvenida' ($ms -ge 0)
    $i = Get-WinInfo
    Log "Ventana: exterior $($i.OuterW)x$($i.OuterH) px, interior $($i.ClientW)x$($i.ClientH) px, escala $($i.Scale), interior logico $($i.LogicalW)x$($i.LogicalH)"
    Shot 'A01-bienvenida-primer-arranque' | Out-Null
    Dump-Tree 'A01-bienvenida.uia.txt' | Out-Null
    Show-Names
    foreach ($h in [QA.Native]::WindowsOfPid([uint32](Get-AppProc)[0].Id)) { Log "  ventana del proceso: clase=$([QA.Native]::ClassOf($h)) titulo=`"$([QA.Native]::TitleOf($h))`"" }
    Show-Net 'arranque'
}
Summary
