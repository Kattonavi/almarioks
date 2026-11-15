# H y D, con la app cerrada: busqueda de canarios y de la contrasena maestra en claro en disco, contenido de los
# ajustes, y lectura de las bovedas con KeePassLib (KeePass 2 instalado, solo lectura) y KeePassXC si esta en el PC.
$uno = Join-Path $Work 'qa-uno.kdbx'; $dos = Join-Path $Work 'qa-dos.kdbx'; $keyx = Join-Path $Work 'qa-dos.keyx'
if ((Get-AppProc).Count -gt 0) { throw 'La app debe estar cerrada para este paso' }

# ---------- H: canarios en disco ----------
$latin = [Text.Encoding]::GetEncoding(28591)   # 1 byte = 1 caracter: permite buscar bytes con String.Contains
$secrets = [ordered]@{
    'contrasena maestra' = $Master; 'canario titulo' = $Canary.Title; 'canario usuario' = $Canary.User; 'canario url' = $Canary.Url
    'canario notas' = $Canary.Notes; 'canario etiqueta' = $Canary.Tag; 'canario contrasena' = $Canary.Pass2
    'contrasena x-QA' = 'x-QA-con-llave-1'; 'prefijo CANARIO-QA' = 'CANARIO-QA'
}
$needles = @()
foreach ($k in $secrets.Keys) {
    $needles += [pscustomobject]@{ What = "$k (UTF-8)"; Text = $latin.GetString([Text.Encoding]::UTF8.GetBytes($secrets[$k])) }
    $needles += [pscustomobject]@{ What = "$k (UTF-16LE)"; Text = $latin.GetString([Text.Encoding]::Unicode.GetBytes($secrets[$k])) }
}
function Scan([string]$label, $files) {
    $hits = 0; $bytes = 0; $count = 0; $skipped = 0
    foreach ($f in $files) {
        if ($f.Length -gt 200MB) { $skipped++; continue }
        try { $data = [IO.File]::ReadAllBytes($f.FullName) } catch { $skipped++; continue }
        $count++; $bytes += $data.Length
        $text = $latin.GetString($data)
        foreach ($n in $needles) {
            if ($text.Contains($n.Text)) { $hits++; Log "   APARICION: $($n.What) en $($f.FullName)" }
        }
    }
    Log ("H [{0}]: {1} archivos, {2:n1} MB leidos, {3} sin leer (bloqueados o enormes), apariciones = {4}" -f $label, $count, ($bytes / 1MB), $skipped, $hits)
    return $hits
}
Step 'H0 Control positivo del buscador' {
    $probe = Join-Path $Work 'control-positivo.bin'
    [IO.File]::WriteAllBytes($probe, [Text.Encoding]::UTF8.GetBytes("xx $($Canary.Title) xx") + [Text.Encoding]::Unicode.GetBytes("yy $($Canary.Pass2) yy"))
    $h = Scan 'control positivo' @(Get-Item $probe)
    Remove-Item $probe
    Check 'Control positivo: el buscador ve un canario plantado en UTF-8 y otro en UTF-16LE' ($h -ge 2) "apariciones: $h"
}
Step 'H1 Carpetas de la app, temporales y carpeta de trabajo' -Continue {
    $start = [datetime](Get-Content (Join-Path $Work 'inicio.txt'))
    $h1 = Scan 'datos de WebView2: %LOCALAPPDATA%\io.github.kattonavi.almarioks' @(Get-ChildItem $DataDir -Recurse -File -Force -ErrorAction SilentlyContinue)
    Check 'RF-77: ningun canario ni la maestra en claro en los datos de WebView2' ($h1 -eq 0)
    $h2 = Scan 'ajustes: %APPDATA%\io.github.kattonavi.almarioks' @(Get-ChildItem $ConfigDir -Recurse -File -Force -ErrorAction SilentlyContinue)
    Check 'RF-77: nada en claro en la carpeta de ajustes' ($h2 -eq 0)
    $h3 = Scan 'carpeta de trabajo (bovedas, .bak, .keyx, salida del proceso)' @(Get-ChildItem $Work -Recurse -File -Force)
    Check 'RF-77: nada en claro en las bovedas, los .bak, el .keyx ni la salida del proceso' ($h3 -eq 0)
    # %TEMP%: solo lo escrito durante la prueba. Se excluye %TEMP%\claude (ahi estan los registros de esta misma QA,
    # que nombran los canarios) y la carpeta de trabajo, ya revisada.
    $temp = Join-Path $env:LOCALAPPDATA 'Temp'
    $tmpFiles = @(Get-ChildItem $temp -Recurse -File -Force -ErrorAction SilentlyContinue | Where-Object {
            $_.LastWriteTime -ge $start -and $_.FullName -notlike (Join-Path $temp 'claude\*') -and $_.FullName -notlike "$Work\*" })
    $h4 = Scan '%TEMP%, archivos escritos durante la prueba' $tmpFiles
    Check 'RF-77: nada en claro en los temporales escritos durante la prueba' ($h4 -eq 0) "$($tmpFiles.Count) archivos"
    $install = Scan 'carpeta de instalacion' @(Get-ChildItem $InstallDir -Recurse -File -Force | Where-Object { $_.Name -ne 'almarioks.exe' -and $_.Name -ne 'uninstall.exe' })
}
Step 'H2 El archivo de ajustes' -Continue {
    $raw = Get-Content -Raw -Encoding UTF8 $SettingsFile
    Log "   settings.json ($($raw.Length) caracteres): $($raw -replace '\s+', ' ')"
    $s = $raw | ConvertFrom-Json
    $keys = @($s.PSObject.Properties.Name)
    Check 'settings.json solo tiene tiempos y rutas recientes' ((@($keys | Where-Object { $_ -notin @('auto_lock_minutes', 'clipboard_seconds', 'recent_vaults') }).Count -eq 0) -and @($s.recent_vaults | Where-Object { $_ -isnot [string] }).Count -eq 0) "claves: $($keys -join ', '); recientes: $(@($s.recent_vaults).Count)"
    Get-ChildItem $ConfigDir -Force | ForEach-Object { Log "   en la carpeta de ajustes: $($_.Name) ($($_.Length) bytes)" }
    Get-ChildItem $Work -Filter 'app-*.log' | ForEach-Object { Log "   salida del proceso: $($_.Name) $($_.Length) bytes" }
}

# ---------- D: KeePass 2 (KeePassLib) ----------
Step 'D2 KeePassLib abre las bovedas creadas por la app' -Continue {
    $exe = Join-Path $env:ProgramFiles 'KeePass Password Safe 2\KeePass.exe'
    if (-not (Test-Path $exe)) { Check 'KeePass 2 esta instalado' $false 'no esta: no se instala'; return }
    [Reflection.Assembly]::LoadFrom($exe) | Out-Null
    Log "   KeePassLib $([Diagnostics.FileVersionInfo]::GetVersionInfo($exe).ProductVersion)"
    function Open-Db([string]$path, [string]$password, [string]$keyFile) {
        $key = [KeePassLib.Keys.CompositeKey]::new()
        if ($password) { $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($password)) }
        if ($keyFile) { $key.AddUserKey([KeePassLib.Keys.KcpKeyFile]::new($keyFile)) }
        $db = [KeePassLib.PwDatabase]::new()
        $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($path), $key, $null)
        return $db
    }
    function Describe($db, [string]$label) {
        $kdf = $db.KdfParameters
        $kdfName = ([KeePassLib.Cryptography.KeyDerivation.KdfPool]::Get($kdf.KdfUuid)).Name
        $mem = $kdf.GetUInt64('M', 0); $it = $kdf.GetUInt64('I', 0); $par = $kdf.GetUInt32('P', 0)
        Log ("   [{0}] KDF {1}: memoria {2} MiB, iteraciones {3}, hilos {4}; cifrado {5}; papelera activa = {6}" -f $label, $kdfName, ($mem / 1MB), $it, $par, ([KeePassLib.Cryptography.Cipher.CipherPool]::GlobalPool.GetCipher($db.DataCipherUuid)).DisplayName, $db.RecycleBinEnabled)
        $lines = New-Object System.Collections.Generic.List[string]
        $walk = {
            param($g, $pathText)
            $here = if ($pathText) { "$pathText › $($g.Name)" } else { $g.Name }
            $lines.Add("grupo: $here ($($g.Entries.UCount) entradas)")
            foreach ($e in $g.Entries) {
                $pw = $e.Strings.ReadSafe('Password')
                $pwNote = if ($pw -ceq $Canary.Pass2) { 'contrasena = canario con ñ y €' } else { "contrasena de $($pw.Length) caracteres" }
                $lines.Add("  entrada: $($e.Strings.ReadSafe('Title')) | $($e.Strings.ReadSafe('UserName')) | $($e.Strings.ReadSafe('URL')) | notas: $($e.Strings.ReadSafe('Notes')) | etiquetas: $(($e.Tags) -join ',') | $pwNote | historial: $($e.History.UCount)")
            }
            foreach ($sub in $g.Groups) { & $walk $sub $here }
        }
        & $walk $db.RootGroup ''
        foreach ($l in $lines) { Log "   [$label] $l" }
        Log "   [$label] objetos borrados anotados: $($db.DeletedObjects.UCount)"
        return $lines
    }
    $db = Open-Db $uno $Master ''
    $l1 = Describe $db 'qa-uno.kdbx'; $kdf1 = $db.KdfParameters; $db.Close()
    $joined = $l1 -join ' ## '
    Check 'KeePass 2 abre qa-uno.kdbx con la contrasena con ñ y €' ($l1.Count -gt 0)
    Check 'KeePass 2 ve los grupos (Archivo › Personal QA) y la entrada con todos sus campos' ($joined -like '*qa-uno › Archivo › Personal QA (1 entradas)*' -and $joined -like "*$($Canary.Title) | $($Canary.User) | $($Canary.Url) | notas: $($Canary.Notes) | etiquetas: $($Canary.Tag) | contrasena = canario*")
    Check 'RF-2: Argon2id con 64 MiB o mas y de 2 a 50 iteraciones' (([KeePassLib.Cryptography.KeyDerivation.KdfPool]::Get($kdf1.KdfUuid)).Name -like '*Argon2id*' -and $kdf1.GetUInt64('M', 0) -ge 64MB -and $kdf1.GetUInt64('I', 0) -ge 2 -and $kdf1.GetUInt64('I', 0) -le 50)
    $bak = Open-Db "$uno.bak" $Master ''
    $lb = Describe $bak 'qa-uno.kdbx.bak'; $bak.Close()
    Check 'RF-22: el .bak abre con la misma llave y trae el estado anterior' ($lb.Count -gt 0)
    $db2 = Open-Db $dos $Master $keyx
    $l2 = Describe $db2 'qa-dos.kdbx'; $db2.Close()
    Check 'KeePass 2 abre qa-dos.kdbx con contrasena + archivo llave .keyx y ve su entrada' (($l2 -join ' ') -like '*CANARIO-QA-ConLlave*')
    $rejected = $false
    try { $x = Open-Db $dos $Master ''; $x.Close() } catch { $rejected = $true; Log "   sin archivo llave: $($_.Exception.InnerException.GetType().Name)" }
    Check 'KeePass 2 rechaza qa-dos.kdbx sin el archivo llave' $rejected
}
Step 'D3 KeePassXC' -Continue {
    $roots = @((Join-Path $env:ProgramFiles 'KeePassXC'), (Join-Path $env:LOCALAPPDATA 'Programs'), (Join-Path $Repo '..\..\laboratorio'), (Join-Path $Repo '..\..\herramientas'), (Join-Path $Repo 'docs\lab'), (Join-Path $env:USERPROFILE 'Downloads'), (Join-Path $env:LOCALAPPDATA 'Temp')) | Where-Object { Test-Path $_ }
    $cli = $null
    foreach ($r in $roots) {
        $cli = Get-ChildItem $r -Recurse -Depth 4 -Filter 'keepassxc-cli.exe' -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($cli) { break }
    }
    if (-not $cli) { Log "   keepassxc-cli.exe no aparece en: $((($roots | ForEach-Object { Redact $_ })) -join ' ; ')"; Check 'KeePassXC disponible en el PC' $false 'no encontrado; no se instala'; return }
    Log "   keepassxc-cli: $($cli.FullName)"
    $OutputEncoding = New-Object System.Text.UTF8Encoding($false)
    $out = $Master | & $cli.FullName ls -R -f $uno 2>&1 | Out-String
    Log "   keepassxc-cli ls -R -f qa-uno.kdbx:`n$out"
    Check 'KeePassXC lista los grupos y la entrada de qa-uno.kdbx' ($out -like "*$($Canary.Title)*")
    $out2 = $Master | & $cli.FullName ls -R -f -k $keyx $dos 2>&1 | Out-String
    Log "   keepassxc-cli ls -R -f -k qa-dos.keyx qa-dos.kdbx:`n$out2"
    Check 'KeePassXC abre qa-dos.kdbx con contrasena + archivo llave' ($out2 -like '*CANARIO-QA-ConLlave*')
    $out3 = $Master | & $cli.FullName db-info $uno 2>&1 | Out-String
    Log "   keepassxc-cli db-info qa-uno.kdbx:`n$out3"
}
Summary
