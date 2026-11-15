# Pase de interaccion (4): la boveda "Familia" creada por la interfaz en la ubicacion propuesta (Documentos > AlmarioKS).
# Solo lee: existencia, tamano, fecha y huella de los DOS archivos, y el nombre de la base con KeePassLib (KeePass 2
# instalado). Argumento "borrar": borra SOLO esos dos archivos por su nombre exacto y la carpeta AlmarioKS si quedo
# vacia y no existia antes de la corrida. No lista ni toca nada mas de Documentos.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$mode = if ($StepArgs.Count) { $StepArgs[0] } else { 'leer' }
function Describe-File([string]$p) { if (Test-Path -LiteralPath $p) { $f = Get-Item -LiteralPath $p; "$($f.Name): $($f.Length) bytes, $($f.LastWriteTime.ToString('HH:mm:ss.fff')), sha256 $(Sha $p)" } else { "$(Split-Path $p -Leaf): no existe" } }
Step 'PX40 Archivos de Familia' -Continue {
    Log "   carpeta AlmarioKS en Documentos: existia antes de la corrida = $(Get-State 'familia-carpeta-antes'); existe ahora = $(Test-Path $FamDir)"
    Log "   $(Describe-File $FamVault)"
    Log "   $(Describe-File $FamKey)"
    if (Test-Path $FamDir) { Log "   archivos en esa carpeta: $(@(Get-ChildItem -LiteralPath $FamDir -Force | ForEach-Object { $_.Name }) -join ', ')" }
}
if ($mode -eq 'leer') {
    Step 'PX41 KeePassLib: nombre de la base' -Continue {
        $exe = Join-Path $env:ProgramFiles 'KeePass Password Safe 2\KeePass.exe'
        if (-not (Test-Path $exe)) { Check 'KeePass 2 esta instalado' $false 'no esta: no se instala'; return }
        [Reflection.Assembly]::LoadFrom($exe) | Out-Null
        Log "   KeePassLib $([Diagnostics.FileVersionInfo]::GetVersionInfo($exe).ProductVersion)"
        $key = [KeePassLib.Keys.CompositeKey]::new()
        $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($FamMaster))
        $key.AddUserKey([KeePassLib.Keys.KcpKeyFile]::new($FamKey))
        $db = [KeePassLib.PwDatabase]::new()
        $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($FamVault), $key, $null)
        $name = $db.Name; $root = $db.RootGroup.Name
        $kdf = ([KeePassLib.Cryptography.KeyDerivation.KdfPool]::Get($db.KdfParameters.KdfUuid)).Name
        $db.Close()
        Check 'KeePassLib abre Familia.kdbx con la contrasena y Familia.keyx, y la base se llama "Familia"' ($name -ceq 'Familia') "nombre de la base = `"$name`"; grupo raiz = `"$root`"; KDF $kdf"
        $rejected = $false
        try { $k2 = [KeePassLib.Keys.CompositeKey]::new(); $k2.AddUserKey([KeePassLib.Keys.KcpPassword]::new($FamMaster)); $x = [KeePassLib.PwDatabase]::new(); $x.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($FamVault), $k2, $null); $x.Close() } catch { $rejected = $true }
        Check 'Sin el archivo llave, KeePassLib la rechaza' $rejected
        if (Test-Path $RfVault) {
            $k3 = [KeePassLib.Keys.CompositeKey]::new(); $k3.AddUserKey([KeePassLib.Keys.KcpPassword]::new($RfMaster))
            $d3 = [KeePassLib.PwDatabase]::new(); $d3.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($RfVault), $k3, $null)
            Check 'La boveda de muestra (archivo Cuentas.kdbx en otra carpeta) se llama "Cuentas"' ($d3.Name -ceq 'Cuentas') "nombre = `"$($d3.Name)`"; raiz = `"$($d3.RootGroup.Name)`""
            $d3.Close()
        }
    }
}
if ($mode -eq 'borrar') {
    Step 'PX42 Borrar solo Familia.kdbx y Familia.keyx' {
        if ((Any-AlmarioProc).Count -gt 0) { throw 'La app debe estar cerrada' }
        if ((Split-Path $FamVault -Leaf) -cne 'Familia.kdbx' -or (Split-Path $FamKey -Leaf) -cne 'Familia.keyx' -or (Split-Path $FamDir -Leaf) -cne 'AlmarioKS') { throw 'Rutas inesperadas: no se borra nada' }
        foreach ($p in @($FamVault, $FamKey)) { if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Force } }
        $left = @(Get-ChildItem -LiteralPath $FamDir -Force -ErrorAction SilentlyContinue)
        if ((Test-Path $FamDir) -and $left.Count -eq 0 -and (Get-State 'familia-carpeta-antes') -eq 'False') { Remove-Item -LiteralPath $FamDir -Force }
        Check 'Familia.kdbx y Familia.keyx borrados' (-not (Test-Path -LiteralPath $FamVault) -and -not (Test-Path -LiteralPath $FamKey))
        Check 'Documentos\AlmarioKS como estaba antes (no existia)' ((Test-Path $FamDir) -eq ((Get-State 'familia-carpeta-antes') -eq 'True')) "existe ahora = $(Test-Path $FamDir); quedaban dentro: $(($left | ForEach-Object { $_.Name }) -join ', ')"
    }
}
Summary
