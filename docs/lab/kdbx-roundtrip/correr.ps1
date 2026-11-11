# Lab 19: ida y vuelta de KDBX 4 entre KeePass 2 (KeePassLib, la referencia) y el crate `keepass` 0.17.
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File correr.ps1
# Todo dato es falso. El archivo va en ASCII: los caracteres no ASCII se arman con [char] (PS 5.1 lee sin BOM como ANSI).
param(
    [string]$KeePassExe = 'C:\Program Files\KeePass Password Safe 2\KeePass.exe',
    [string]$LabExe = (Join-Path $PSScriptRoot 'kdbx-lab\target\release\kdbx-lab.exe'),
    [string]$Work = (Join-Path $PSScriptRoot 'trabajo')
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
[Reflection.Assembly]::LoadFrom($KeePassExe) | Out-Null
Add-Type -AssemblyName System.Drawing

$enie = [string][char]0x00F1
$uAcute = [string][char]0x00FA
$euro = [string][char]0x20AC
$PwAscii = 'lab-Maestra-2026'
$PwUnicode = "$($enie)and$uAcute-$euro-2026"

if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
New-Item -ItemType Directory -Force $Work | Out-Null

function New-Key([string]$password, [string]$keyFile) {
    $key = [KeePassLib.Keys.CompositeKey]::new()
    if ($password) { $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($password)) }
    if ($keyFile) { $key.AddUserKey([KeePassLib.Keys.KcpKeyFile]::new($keyFile)) }
    return $key
}

function PStr([bool]$protect, [string]$value) { return [KeePassLib.Security.ProtectedString]::new($protect, $value) }

function Sha([byte[]]$bytes) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($bytes)) -replace '-', '').Substring(0, 16) } finally { $sha.Dispose() }
}

function New-Fixture([string]$path, [string]$password, [string]$keyFile, [string]$cipher) {
    $db = [KeePassLib.PwDatabase]::new()
    $db.New([KeePassLib.Serialization.IOConnectionInfo]::FromPath($path), (New-Key $password $keyFile))
    $kdf = [KeePassLib.Cryptography.KeyDerivation.Argon2Kdf]::new([KeePassLib.Cryptography.KeyDerivation.Argon2Type]::ID)
    $db.KdfParameters = $kdf.GetDefaultParameters()
    if ($cipher -eq 'chacha20') {
        $db.DataCipherUuid = [KeePassLib.PwUuid]::new([byte[]](0xD6, 0x03, 0x8A, 0x2B, 0x8B, 0x6F, 0x4C, 0xB5, 0xA5, 0x24, 0x33, 0x9A, 0x31, 0xDB, 0xB5, 0x9A))
    }
    $db.Name = "B$([char]0x00F3)veda de prueba"
    $db.Description = 'Fixture del lab 19'
    $db.DefaultUserName = 'usuario@example.com'
    $db.HistoryMaxItems = 7
    $db.CustomData.Set('lab19.meta', 'valor-meta')
    $db.PublicCustomData.SetString('lab19.public', 'valor-publico')

    $png = [Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==')
    $iconUuid = [KeePassLib.PwUuid]::new($true)
    $db.CustomIcons.Add([KeePassLib.PwCustomIcon]::new($iconUuid, $png))
    $db.UINeedsIconUpdate = $true

    $root = $db.RootGroup
    $work = [KeePassLib.PwGroup]::new($true, $true, 'Trabajo', [KeePassLib.PwIcon]::Folder)
    $work.Notes = "Notas del grupo con e$($enie)e"
    $work.CustomData.Set('lab19.group', 'valor-grupo')
    $work.EnableAutoType = $false
    $work.EnableSearching = $true
    $work.DefaultAutoTypeSequence = '{USERNAME}{TAB}{PASSWORD}{ENTER}'
    $work.Tags.Add('etiqueta-grupo')
    $work.CustomIconUuid = $iconUuid
    $root.AddGroup($work, $true)
    $deep = [KeePassLib.PwGroup]::new($true, $true, 'Subgrupo', [KeePassLib.PwIcon]::Key)
    $work.AddGroup($deep, $true)

    $e = [KeePassLib.PwEntry]::new($true, $true)
    $e.Strings.Set('Title', (PStr $false 'GitHub'))
    $e.Strings.Set('UserName', (PStr $false 'sara@example.com'))
    $e.Strings.Set('Password', (PStr $true 'primera-clave'))
    $e.Strings.Set('URL', (PStr $false 'https://github.com'))
    $e.Strings.Set('Notes', (PStr $false "linea 1`nlinea 2 con <xml> & $euro"))
    $e.Strings.Set('Campo propio', (PStr $false 'visible'))
    $e.Strings.Set('Campo protegido', (PStr $true "secreto-$enie"))
    $e.Binaries.Set('adjunto.txt', [KeePassLib.Security.ProtectedBinary]::new($false, [Text.Encoding]::UTF8.GetBytes('contenido del adjunto')))
    $e.Binaries.Set('protegido.bin', [KeePassLib.Security.ProtectedBinary]::new($true, [byte[]](0..255)))
    [void]$e.AddTag('trabajo'); [void]$e.AddTag('codigo')
    $e.Expires = $true
    $e.ExpiryTime = [DateTime]::new(2030, 1, 2, 3, 4, 5, [DateTimeKind]::Utc)
    $e.ForegroundColor = [Drawing.Color]::FromArgb(255, 10, 20, 30)
    $e.BackgroundColor = [Drawing.Color]::FromArgb(255, 200, 210, 220)
    $e.OverrideUrl = 'cmd://navegador {URL}'
    $e.QualityCheck = $false
    $e.CustomData.Set('lab19.entry', 'valor-entrada')
    $e.AutoType.DefaultSequence = '{USERNAME}{TAB}{PASSWORD}'
    $e.AutoType.Add([KeePassLib.Collections.AutoTypeAssociation]::new('GitHub - *', '{PASSWORD}{ENTER}'))
    $e.CustomIconUuid = $iconUuid
    $work.AddEntry($e, $true)
    $e.CreateBackup($db); $e.Strings.Set('Password', (PStr $true 'segunda-clave')); $e.Touch($true)
    $e.CreateBackup($db); $e.Strings.Set('Password', (PStr $true 'tercera-clave')); $e.Touch($true)

    $e2 = [KeePassLib.PwEntry]::new($true, $true)
    $e2.Strings.Set('Title', (PStr $false 'Banco'))
    $e2.Strings.Set('UserName', (PStr $false '1020304050'))
    $e2.Strings.Set('Password', (PStr $true "cl$([char]0x00E1)ve con espacios y `"comillas`""))
    $deep.AddEntry($e2, $true)

    $gone = [KeePassLib.PwEntry]::new($true, $true)
    $db.DeletedObjects.Add([KeePassLib.PwDeletedObject]::new($gone.Uuid, [DateTime]::new(2026, 11, 1, 0, 0, 0, [DateTimeKind]::Utc)))

    $db.Save($null)
    $db.Close()
}

function T([DateTime]$t) { return $t.ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss') }

function Add-EntryLines($lines, [string]$tag, [string]$path, $e) {
    $id = $e.Uuid.ToHexString()
    $lines.Add("$tag $id path=$path icon=$($e.IconId) cicon=$($e.CustomIconUuid.ToHexString()) fg=$($e.ForegroundColor.ToArgb()) bg=$($e.BackgroundColor.ToArgb()) override=$($e.OverrideUrl) tags=$([string]::Join(',', $e.Tags)) expires=$($e.Expires) expiry=$(T $e.ExpiryTime) quality=$($e.QualityCheck) created=$(T $e.CreationTime) modified=$(T $e.LastModificationTime) moved=$(T $e.LocationChanged) usage=$($e.UsageCount)")
    foreach ($kv in $e.Strings) { $lines.Add("  STR $id $($kv.Key) prot=$($kv.Value.IsProtected) value=$($kv.Value.ReadString() -replace "`n", '\n')") }
    foreach ($kv in $e.Binaries) { $d = $kv.Value.ReadData(); $lines.Add("  BIN $id $($kv.Key) prot=$($kv.Value.IsProtected) len=$($d.Length) sha=$(Sha $d)") }
    foreach ($kv in $e.CustomData) { $lines.Add("  CD $id $($kv.Key)=$($kv.Value)") }
    $lines.Add("  AT $id enabled=$($e.AutoType.Enabled) obf=$($e.AutoType.ObfuscationOptions) default=$($e.AutoType.DefaultSequence)")
    foreach ($a in $e.AutoType.Associations) { $lines.Add("  ATA $id window=$($a.WindowName) seq=$($a.Sequence)") }
}

function Add-GroupLines($lines, $g, [string]$path) {
    $lines.Add("GROUP $($g.Uuid.ToHexString()) path=$path notes=$($g.Notes) icon=$($g.IconId) cicon=$($g.CustomIconUuid.ToHexString()) tags=$([string]::Join(',', $g.Tags)) expanded=$($g.IsExpanded) at=$($g.EnableAutoType) search=$($g.EnableSearching) seq=$($g.DefaultAutoTypeSequence) created=$(T $g.CreationTime) modified=$(T $g.LastModificationTime)")
    foreach ($kv in $g.CustomData) { $lines.Add("  GCD $($g.Uuid.ToHexString()) $($kv.Key)=$($kv.Value)") }
    foreach ($e in $g.Entries) {
        Add-EntryLines $lines 'ENTRY' $path $e
        $i = 0
        foreach ($h in $e.History) { Add-EntryLines $lines "  HIST[$i]" $path $h; $i++ }
    }
    foreach ($sub in $g.Groups) { Add-GroupLines $lines $sub "$path/$($sub.Name)" }
}

# Resumen de una base leida con KeePassLib. Devuelve lineas comparables.
function Get-Summary([string]$path, [string]$password, [string]$keyFile) {
    $db = [KeePassLib.PwDatabase]::new()
    $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($path), (New-Key $password $keyFile), $null)
    $lines = [Collections.Generic.List[string]]::new()
    $lines.Add("META name=$($db.Name) desc=$($db.Description) user=$($db.DefaultUserName) recycle=$($db.RecycleBinEnabled) recycleUuid=$($db.RecycleBinUuid.ToHexString()) histItems=$($db.HistoryMaxItems) histSize=$($db.HistoryMaxSize) color=$($db.Color.ToArgb()) maint=$($db.MaintenanceHistoryDays) keyRec=$($db.MasterKeyChangeRec) keyForce=$($db.MasterKeyChangeForce)")
    $lines.Add("META cipher=$($db.DataCipherUuid.ToHexString()) compression=$($db.Compression) kdf=$($db.KdfParameters.KdfUuid.ToHexString())")
    $mp = $db.MemoryProtection
    $lines.Add("META protect title=$($mp.ProtectTitle) user=$($mp.ProtectUserName) pass=$($mp.ProtectPassword) url=$($mp.ProtectUrl) notes=$($mp.ProtectNotes)")
    foreach ($kv in $db.CustomData) { $lines.Add("META cd $($kv.Key)=$($kv.Value)") }
    $lines.Add("META public lab19.public=$($db.PublicCustomData.GetString('lab19.public'))")
    foreach ($ic in $db.CustomIcons) { $lines.Add("ICON $($ic.Uuid.ToHexString()) len=$($ic.ImageDataPng.Length) sha=$(Sha $ic.ImageDataPng)") }
    foreach ($d in $db.DeletedObjects) { $lines.Add("DELETED $($d.Uuid.ToHexString()) at=$(T $d.DeletionTime)") }
    Add-GroupLines $lines $db.RootGroup $db.RootGroup.Name
    $db.Close()
    return $lines
}

function Get-FormatVersion([string]$path) {
    $b = [IO.File]::ReadAllBytes($path)
    return "$([BitConverter]::ToUInt16($b, 10)).$([BitConverter]::ToUInt16($b, 8))"
}

function Invoke-Lab([string[]]$labArgs) {
    $out = & $LabExe @labArgs 2>&1 | ForEach-Object { "$_" }
    if ($LASTEXITCODE -ne 0) { throw "kdbx-lab $($labArgs[0]) fallo ($LASTEXITCODE): $out" }
    return $out
}

function Compare-Summaries([string]$name, $before, $after) {
    $diff = @(Compare-Object -ReferenceObject $before -DifferenceObject $after)
    if ($diff.Count -eq 0) { Write-Host "  [$name] IDENTICO ($($before.Count) lineas)"; return 0 }
    Write-Host "  [$name] DIFERENCIAS: $($diff.Count)"
    foreach ($d in $diff) { Write-Host "    $($d.SideIndicator) $($d.InputObject)" }
    return $diff.Count
}

Write-Host "KeePassLib $([KeePassLib.PwDefs]::VersionString)"
$keyFile = Join-Path $Work 'llave.keyx'
[KeePassLib.Keys.KcpKeyFile]::Create($keyFile, $null)

$cases = @(
    @{ Name = 'pass'; Password = $PwAscii; KeyFile = $null; Cipher = 'aes' },
    @{ Name = 'key'; Password = $null; KeyFile = $keyFile; Cipher = 'aes' },
    @{ Name = 'both'; Password = $PwAscii; KeyFile = $keyFile; Cipher = 'chacha20' },
    @{ Name = 'unicode'; Password = $PwUnicode; KeyFile = $null; Cipher = 'aes' }
)

Write-Host '== 1. Reescritura sin cambios: KeePass 2 -> Rust (open + save) -> KeePass 2 =='
$total = 0
foreach ($c in $cases) {
    $src = Join-Path $Work "f-$($c.Name).kdbx"
    $dst = Join-Path $Work "r-$($c.Name).kdbx"
    New-Fixture $src $c.Password $c.KeyFile $c.Cipher
    $pw = if ($c.Password) { $c.Password } else { '-' }
    $kf = if ($c.KeyFile) { $c.KeyFile } else { '-' }
    $labOut = Invoke-Lab @('rewrite', $src, $dst, $pw, $kf)
    Write-Host "  [$($c.Name)] formato $(Get-FormatVersion $src) -> $(Get-FormatVersion $dst) | $labOut"
    $before = Get-Summary $src $c.Password $c.KeyFile
    $after = Get-Summary $dst $c.Password $c.KeyFile
    $total += Compare-Summaries $c.Name $before $after
}

Write-Host '== 2. Edicion en Rust (cambia clave con historial + entrada nueva con adjunto) -> KeePass 2 =='
$src = Join-Path $Work 'f-both.kdbx'
$dst = Join-Path $Work 'e-both.kdbx'
Invoke-Lab @('edit', $src, $dst, $PwAscii, $keyFile) | Out-Null
$before = Get-Summary $src $PwAscii $keyFile
$after = Get-Summary $dst $PwAscii $keyFile
Compare-Summaries 'edit' $before $after | Out-Null

Write-Host '== 3. Boveda y archivo llave creados en Rust -> KeePass 2 =='
$newDb = Join-Path $Work 'nueva.kdbx'
$newKey = Join-Path $Work 'nueva.keyx'
Write-Host "  $(Invoke-Lab @('create', $newDb, $PwUnicode, $newKey))"
Write-Host "  formato $(Get-FormatVersion $newDb)"
Get-Summary $newDb $PwUnicode $newKey | ForEach-Object { Write-Host "  $_" }
try { Get-Summary $newDb $PwUnicode $null | Out-Null; Write-Host '  ERROR: abrio sin archivo llave' }
catch { Write-Host "  sin archivo llave: rechazada ($($_.Exception.InnerException.GetType().Name))" }

Write-Host "== Lineas distintas en la reescritura sin cambios: $total =="
