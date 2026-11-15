# Reverificacion final (3), con la app cerrada: todo lo que quedo en disco tras el recorrido con canarios nuevos
# (perfil de WebView2, ajustes, carpeta de trabajo, %TEMP%), con su control positivo, y lectura con KeePassLib.
# Argumento opcional: una etiqueta para la pasada ("segunda" repite solo el perfil, los ajustes y la carpeta de trabajo).
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$pass = if ($StepArgs.Count) { $StepArgs[0] } else { 'primera' }
if ((Get-AppProc).Count -gt 0) { throw 'La app debe estar cerrada' }
$secrets = Rf-Secrets
$since = Start-Epoch
Log "PASADA: $pass; agujas en claro: $(if (Get-State 'generada') { 'canarios + contrasena generada' } else { 'canarios' })"
Step 'RF7 Control positivo del inspector' {
    $dir = Join-Path $Work 'control'
    New-Item -ItemType Directory -Force $dir | Out-Null
    $sha = [Security.Cryptography.SHA1]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($RfMaster))
    $big = [byte[]]$sha[0..7]; [Array]::Reverse($big)
    $dec = [BitConverter]::ToUInt64($big, 0).ToString()
    [IO.File]::WriteAllBytes((Join-Path $dir 'plantado.bin'), [byte[]]([Text.Encoding]::UTF8.GetBytes("aa $($Rf.Title) bb $dec cc") + [Text.Encoding]::Unicode.GetBytes("dd $($Rf.Notes) ee")))
    $env:RV_SECRETS = $secrets
    $planted = @(& py -3 -I (Join-Path $QaDir 'rf-plantar-control.py') $dir 2>&1 | ForEach-Object { "$_" })
    foreach ($l in $planted) { Log "   $l" }
    $lines = Run-Inspector $secrets @("control=$dir")
    Remove-Item -LiteralPath $dir -Recurse -Force
    $kinds = @($lines | Where-Object { $_ -like 'APARICION|*' } | ForEach-Object { ($_ -split '\|')[3] })
    Check 'CONTROL POSITIVO: ve un canario en UTF-8, otro en UTF-16LE y el SHA-1 de 8 bytes de la maestra en decimal' (($kinds -contains 'titulo, en claro (UTF-8)') -and ($kinds -contains 'notas, en claro (UTF-16LE)') -and ($kinds -contains 'maestra, SHA-1 8 bytes en decimal (UTF-8)')) "$($kinds.Count) apariciones"
    $sq = @($lines | Where-Object { $_ -like 'SQLITE|control|Web Data|*autofill_edge_field_values=2*' -or $_ -like 'SQLITE|control|Web Data|*en claro=1, enmascaradas=1*' }).Count
    $hh = @($lines | Where-Object { $_ -like 'COINCIDE_HASH|control|*maestra*' }).Count
    Check 'CONTROL POSITIVO: en una base SQLite como Web Data cuenta las 2 filas autofill y reconoce la fila enmascarada como el SHA-1 de la maestra' ($sq -eq 2 -and $hh -eq 1) "lineas de conteo: $sq de 2; coincidencias de hash: $hh"
}
Step 'RF8 Carpeta de datos de WebView2 completa' -Continue {
    Log "Carpeta de datos: $((Get-ChildItem $DataDir -Force | ForEach-Object { $_.Name }) -join ', ')"
    if (Test-Path $EbDir) { Log "EBWebView: $((Get-ChildItem $EbDir -Force | ForEach-Object { $_.Name }) -join ', ')" }
    $def = Join-Path $EbDir 'Default'
    if (Test-Path $def) { Log "EBWebView\Default: $((Get-ChildItem $def -Force | ForEach-Object { $_.Name }) -join ', ')" }
    $wd = Join-Path $EbDir 'Default\Web Data'
    Log "Web Data existe = $(Test-Path $wd)$(if (Test-Path $wd) { " ($((Get-Item $wd).Length) bytes, modificado $((Get-Item $wd).LastWriteTime.ToString('HH:mm:ss')))" })"
    $lines = Run-Inspector $secrets @("webview2=$DataDir") -Tree
    Check "[$pass] RF-77: ningun canario en claro en la carpeta de datos de WebView2" (@($lines | Where-Object { $_ -like 'APARICION|*en claro*' }).Count -eq 0 -and (Hits-Of $lines 'webview2') -eq 0) "apariciones: $(Hits-Of $lines 'webview2')"
    Check "[$pass] RF-77: ninguna forma del SHA-1 de la maestra ni de su repeticion" (@($lines | Where-Object { $_ -like 'APARICION|*SHA-1*' -or $_ -like 'COINCIDE_HASH|*' }).Count -eq 0)
    $auto = @($lines | Where-Object { $_ -like 'SQLITE|*filas autofill=*' })
    $rows = 0; foreach ($l in $auto) { if ($l -match 'filas autofill=(\d+)') { $rows += [int]$Matches[1] } }
    Check "[$pass] Las tablas autofill* de todas las bases SQLite del perfil estan vacias" ($rows -eq 0) "bases SQLite: $($auto.Count); con tablas autofill: $(@($auto | Where-Object { $_ -notlike '*tablas autofill=0|*' }).Count); filas en total: $rows"
    $bad = @($lines | Where-Object { $_ -like 'SQLITE|*no se pudo abrir*' -or $_ -like 'SQLITE|*error al leer*' })
    Check "[$pass] Todas las bases SQLite se pudieron leer" ($bad.Count -eq 0) "$($bad.Count) sin leer"
}
Step 'RF9 Ajustes, carpeta de trabajo y temporales' -Continue {
    $l2 = Run-Inspector $secrets @("ajustes=$ConfigDir", "trabajo=$Work")
    Check "[$pass] Nada en la carpeta de ajustes" ((Hits-Of $l2 'ajustes') -eq 0)
    Check "[$pass] Nada en la carpeta de trabajo (boveda, .bak, salida del proceso)" ((Hits-Of $l2 'trabajo') -eq 0) "apariciones: $(Hits-Of $l2 'trabajo')"
    $temp = Join-Path $env:LOCALAPPDATA 'Temp'
    # Control positivo del filtro por fecha: un archivo plantado ahora en la raiz de %TEMP% tiene que salir.
    $plant = Join-Path $temp 'almarioks-qa2-control-plantado.bin'
    [IO.File]::WriteAllBytes($plant, [Text.Encoding]::UTF8.GetBytes("xx $($Rf.Tag) yy"))
    $l3 = Run-Inspector $secrets @("temp=$temp") $since @((Join-Path $temp 'claude'), $Work)
    Remove-Item -LiteralPath $plant -Force
    $own = @($l3 | Where-Object { $_ -like 'APARICION|temp|almarioks-qa2-control-plantado.bin|*' }).Count
    $other = @($l3 | Where-Object { $_ -like 'APARICION|temp|*' -and $_ -notlike 'APARICION|temp|almarioks-qa2-control-plantado.bin|*' }).Count
    Check "[$pass] CONTROL POSITIVO en %TEMP%: el archivo plantado ahora si aparece" ($own -ge 1) "apariciones del plantado: $own"
    Check "[$pass] Nada en %TEMP% entre los archivos escritos durante la prueba (sin %TEMP%\claude ni la carpeta de trabajo)" ($other -eq 0 -and (Hits-Of $l3 'temp') -eq $own) "apariciones fuera del plantado: $other"
    $start = [datetime](Get-State 'inicio')
    $newDirs = @(Get-ChildItem $env:LOCALAPPDATA -Directory -Force -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $start }) + @(Get-ChildItem $temp -Directory -Force -ErrorAction SilentlyContinue | Where-Object { $_.CreationTime -ge $start -and $_.FullName -ne $Work })
    Log "Carpetas nuevas en %LOCALAPPDATA% y %TEMP% desde el inicio: $(($newDirs | ForEach-Object { $_.Name }) -join ', ')"
    $webDirs = @(Get-ChildItem $env:LOCALAPPDATA -Directory -Recurse -Depth 2 -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'EBWebView*' -and $_.LastWriteTime -ge $start })
    Log "Carpetas EBWebView tocadas durante la prueba: $(($webDirs | ForEach-Object { Redact $_.FullName }) -join ' ; ')"
    $raw = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
    Log "settings.json: claves = $(($raw.PSObject.Properties.Name) -join ', '); auto_lock_minutes=$($raw.auto_lock_minutes) clipboard_seconds=$($raw.clipboard_seconds) recientes=$(@($raw.recent_vaults).Count)"
    Get-ChildItem $Work -Filter 'app-*.log' | ForEach-Object { Log "   salida del proceso: $($_.Name) $($_.Length) bytes" }
}
Step 'RF10 KeePassLib abre la boveda creada por la app de release' -Continue {
    $exe = Join-Path $env:ProgramFiles 'KeePass Password Safe 2\KeePass.exe'
    if (-not (Test-Path $exe)) { Check 'KeePass 2 esta instalado' $false 'no esta: no se instala'; return }
    [Reflection.Assembly]::LoadFrom($exe) | Out-Null
    Log "   KeePassLib $([Diagnostics.FileVersionInfo]::GetVersionInfo($exe).ProductVersion)"
    function Open-Db([string]$path) {
        $key = [KeePassLib.Keys.CompositeKey]::new()
        $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($RfMaster))
        $db = [KeePassLib.PwDatabase]::new()
        $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($path), $key, $null)
        return $db
    }
    function Describe($db, [string]$label) {
        $kdf = $db.KdfParameters
        $kdfName = ([KeePassLib.Cryptography.KeyDerivation.KdfPool]::Get($kdf.KdfUuid)).Name
        Log ("   [{0}] KDF {1}: memoria {2} MiB, iteraciones {3}, hilos {4}; cifrado {5}; papelera activa = {6}" -f $label, $kdfName, ($kdf.GetUInt64('M', 0) / 1MB), $kdf.GetUInt64('I', 0), $kdf.GetUInt32('P', 0), ([KeePassLib.Cryptography.Cipher.CipherPool]::GlobalPool.GetCipher($db.DataCipherUuid)).DisplayName, $db.RecycleBinEnabled)
        $lines = New-Object System.Collections.Generic.List[string]
        $gen = Get-State 'generada'
        $walk = {
            param($g, $pathText)
            $here = if ($pathText) { "$pathText › $($g.Name)" } else { $g.Name }
            $lines.Add("grupo: $here ($($g.Entries.UCount) entradas)")
            foreach ($e in $g.Entries) {
                $pw = $e.Strings.ReadSafe('Password')
                $pwNote = if ($pw -ceq $Rf.Pass) { 'contrasena = canario' } elseif ($gen -and $pw -ceq $gen) { 'contrasena = la generada y copiada' } else { "contrasena de $($pw.Length) caracteres" }
                $lines.Add("  entrada: $($e.Strings.ReadSafe('Title')) | $($e.Strings.ReadSafe('UserName')) | $($e.Strings.ReadSafe('URL')) | notas: $($e.Strings.ReadSafe('Notes')) | etiquetas: $(($e.Tags) -join ',') | $pwNote | historial: $($e.History.UCount)")
            }
            foreach ($sub in $g.Groups) { & $walk $sub $here }
        }
        & $walk $db.RootGroup ''
        foreach ($l in $lines) { Log "   [$label] $l" }
        Log "   [$label] objetos borrados anotados: $($db.DeletedObjects.UCount)"
        return $lines
    }
    $head = [IO.File]::ReadAllBytes($RfVault)[0..11]
    Check 'Cabecera KDBX 4' ($head[0] -eq 0x03 -and $head[1] -eq 0xD9 -and $head[4] -eq 0x67 -and $head[10] -eq 0x04) (($head | ForEach-Object { $_.ToString('X2') }) -join ' ')
    $db = Open-Db $RfVault
    $l1 = Describe $db 'rf-uno.kdbx'; $kdf1 = $db.KdfParameters; $db.Close()
    $joined = $l1 -join ' ## '
    Check "[$pass] KeePassLib abre rf-uno.kdbx con la maestra con ñ y €" ($l1.Count -gt 0)
    Check "[$pass] KeePassLib ve el grupo canario y la entrada con todos sus campos" ($joined -like "*rf-uno › $($Rf.Group) (*" -and $joined -like "*$($Rf.Title) | $($Rf.User) | $($Rf.Url) | notas: $($Rf.Notes) | etiquetas: $($Rf.Tag) | contrasena = canario*")
    Check 'RF-2: Argon2id con 64 MiB o mas y de 2 a 50 iteraciones' (([KeePassLib.Cryptography.KeyDerivation.KdfPool]::Get($kdf1.KdfUuid)).Name -like '*Argon2id*' -and $kdf1.GetUInt64('M', 0) -ge 64MB -and $kdf1.GetUInt64('I', 0) -ge 2 -and $kdf1.GetUInt64('I', 0) -le 50)
    $bak = Open-Db "$RfVault.bak"
    $lb = Describe $bak 'rf-uno.kdbx.bak'; $bak.Close()
    Check 'RF-22: el .bak abre con la misma llave' ($lb.Count -gt 0)
    $rejected = $false
    try { $key = [KeePassLib.Keys.CompositeKey]::new(); $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new('no-es-la-maestra')); $x = [KeePassLib.PwDatabase]::new(); $x.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($RfVault), $key, $null); $x.Close() } catch { $rejected = $true }
    Check 'KeePassLib rechaza una contrasena distinta' $rejected
}
Summary
