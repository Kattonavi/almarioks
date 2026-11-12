# Paso 0: estado del PC antes de tocar nada. Solo lee. De los ajustes anota tiempos y cuantas recientes hay, nunca rutas.
Log "Instalacion: carpeta existe = $(Test-Path $InstallDir)"
if (Test-Path $AppExe) {
    $exe = Get-Item $AppExe
    Log "  almarioks.exe instalado: $($exe.Length) bytes, $($exe.LastWriteTime.ToString('yyyy-MM-dd HH:mm')), sha256 $(Sha $AppExe)"
}
$built = Join-Path $Repo 'target\debug\almarioks.exe'
if (Test-Path $built) {
    $b = Get-Item $built
    Log "  almarioks.exe compilado (target\debug): $($b.Length) bytes, $($b.LastWriteTime.ToString('yyyy-MM-dd HH:mm')), sha256 $(Sha $built)"
}
$running = Get-AppProc
Log "Procesos almarioks instalados en marcha: $($running.Count)"
foreach ($p in $running) { Log "  pid $($p.Id) desde $($p.StartTime.ToString('HH:mm'))" }
$other = @(Get-Process almarioks -ErrorAction SilentlyContinue | Where-Object { $_.Path -ine $AppExe })
Log "Procesos almarioks de otra ruta: $($other.Count)"

$uninstall = 'Software\Microsoft\Windows\CurrentVersion\Uninstall'
$hkcu = @(Get-ItemProperty "HKCU:\$uninstall\*" -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -like '*lmario*' })
foreach ($k in $hkcu) { Log "HKCU: $($k.DisplayName) | $($k.DisplayVersion) | $($k.PSChildName)" }
$hklm = @(@("HKLM:\$uninstall\*", "HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*") |
    ForEach-Object { Get-ItemProperty $_ -ErrorAction SilentlyContinue } | Where-Object { $_.DisplayName -like '*lmario*' })
Log "HKLM claves de desinstalacion: $($hklm.Count)"

Log "Carpeta de ajustes (%APPDATA%\io.github.kattonavi.almarioks) existe = $(Test-Path $ConfigDir)"
Log "Carpeta de datos WebView2 (%LOCALAPPDATA%\io.github.kattonavi.almarioks) existe = $(Test-Path $DataDir)"
if (Test-Path $SettingsFile) {
    try {
        $s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
        Log "settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
    } catch { Log "settings.json existe pero no se pudo leer como JSON: $($_.Exception.Message)" }
} else { Log 'settings.json: no existe' }
if (Test-Path $ConfigDir) { Get-ChildItem $ConfigDir | ForEach-Object { Log "  ajustes: $($_.Name) ($($_.Length) bytes)" } }

Log "KeePass 2: $(Test-Path 'C:\Program Files\KeePass Password Safe 2\KeePass.exe')"
$xc = @(Get-Command keepassxc-cli -ErrorAction SilentlyContinue)
Log "keepassxc-cli en PATH: $($xc.Count)"
$i = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
Log "Pantalla (px fisicos): $($i.Width) x $($i.Height)"
Log "Carpeta de trabajo existe = $(Test-Path $Work)"
