# Genera una boveda KDBX 3.1 (AES-KDF) con KeePassLib 2.62, para la P4 del lab (RF-13, RF-14).
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File generar-kdbx3.ps1
# Escribe en %TEMP%\almarioks-boveda-lab\v3-aes.kdbx. Datos falsos. Archivo en ASCII (PS 5.1).
param(
    [string]$KeePassExe = 'C:\Program Files\KeePass Password Safe 2\KeePass.exe'
)
$ErrorActionPreference = 'Stop'
[Reflection.Assembly]::LoadFrom($KeePassExe) | Out-Null

$dir = Join-Path $env:TEMP 'almarioks-boveda-lab'
New-Item -ItemType Directory -Force $dir | Out-Null
$path = Join-Path $dir 'v3-aes.kdbx'
if (Test-Path $path) { Remove-Item -Force $path }

$key = [KeePassLib.Keys.CompositeKey]::new()
$key.AddUserKey([KeePassLib.Keys.KcpPassword]::new('lab-Maestra-2026'))

$db = [KeePassLib.PwDatabase]::new()
$db.New([KeePassLib.Serialization.IOConnectionInfo]::FromPath($path), $key)
# AES-KDF and no KDBX 4 only features: KeePassLib picks the 3.1 file format.
$aes = [KeePassLib.Cryptography.KeyDerivation.AesKdf]::new()
$params = $aes.GetDefaultParameters()
$params.SetUInt64([KeePassLib.Cryptography.KeyDerivation.AesKdf]::ParamRounds, [uint64]60000)
$db.KdfParameters = $params

$entry = [KeePassLib.PwEntry]::new($true, $true)
$entry.Strings.Set('Title', [KeePassLib.Security.ProtectedString]::new($false, 'Cuenta v3'))
$entry.Strings.Set('UserName', [KeePassLib.Security.ProtectedString]::new($false, 'ana@example.com'))
$entry.Strings.Set('Password', [KeePassLib.Security.ProtectedString]::new($true, 'clave-v3'))
$db.RootGroup.AddEntry($entry, $true)

# PwDatabase.Save picks KDBX 4.0 even with AES-KDF in 2.62 (first run of this script): force 3.1 with KdbxFile.
$kdbx = [KeePassLib.Serialization.KdbxFile]::new($db)
$force = [KeePassLib.Serialization.KdbxFile].GetProperty('ForceVersion', [Reflection.BindingFlags]'Public,NonPublic,Instance')
$force.GetSetMethod($true).Invoke($kdbx, @([uint32]0x00030001)) | Out-Null  # internal setter
$stream = [IO.File]::Create($path)
try { $kdbx.Save($stream, $null, [KeePassLib.Serialization.KdbxFormat]::Default, $null) } finally { $stream.Dispose() }
$db.Close()

$bytes = [IO.File]::ReadAllBytes($path)
$minor = [BitConverter]::ToUInt16($bytes, 8)
$major = [BitConverter]::ToUInt16($bytes, 10)
Write-Output "KeePassLib escribio %TEMP%\almarioks-boveda-lab\v3-aes.kdbx: KDBX $major.$minor ($($bytes.Length) bytes)"
