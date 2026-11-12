# Reverificacion final (1): con la app cerrada, borrar SOLO EBWebView e instalar en silencio el instalador de RELEASE.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$setup = Join-Path $Repo 'target\release\bundle\nsis\AlmarioKS_0.1.0_x64-setup.exe'
$built = Join-Path $Repo 'target\release\almarioks.exe'
Step 'RF1 Estado previo y borrado de EBWebView' {
    if ((Get-AppProc).Count -gt 0 -or @(Get-Process almarioks -ErrorAction SilentlyContinue).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    New-Item -ItemType Directory -Force $Work | Out-Null
    Set-State 'inicio' ((Get-Date).ToString('o'))
    Log "Instalado antes: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe)"
    if (Test-Path $EbDir) {
        $files = @(Get-ChildItem $EbDir -Recurse -File -Force)
        Log ("EBWebView antes de borrar: {0} archivos, {1:n1} MB" -f $files.Count, (($files | Measure-Object Length -Sum).Sum / 1MB))
        if ($EbDir -notlike '*\io.github.kattonavi.almarioks\EBWebView') { throw 'Ruta inesperada: no se borra nada' }
        Remove-Item -LiteralPath $EbDir -Recurse -Force
    }
    Check 'EBWebView borrada (solo esa carpeta)' (-not (Test-Path $EbDir)) "queda en la carpeta de datos: $((Get-ChildItem $DataDir -Force | ForEach-Object { $_.Name }) -join ', ')"
}
Step 'RF2 Instalar el instalador de release' {
    $conf = Get-Content -Raw (Join-Path $Repo 'src-tauri\tauri.conf.json')
    Check 'tauri.conf.json: incognito y autocompletado apagado' ($conf -match '"incognito"\s*:\s*true' -and $conf -match '"generalAutofillEnabled"\s*:\s*false')
    $lastCommit = (& git -C $Repo log -1 --format='%cI %h' -- src src-tauri crates Cargo.toml package.json)
    $dirty = @(& git -C $Repo status --short -- src src-tauri crates).Count
    Log "Instalador: $((Get-Item $setup).Length) bytes, $((Get-Item $setup).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss zzz')), sha256 $(Sha $setup)"
    Log "Exe de target\release: $((Get-Item $built).Length) bytes, $((Get-Item $built).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss zzz')), sha256 $(Sha $built)"
    Log "Ultimo commit que toca el producto: $lastCommit; cambios de producto sin commit: $dirty"
    $commitTime = [DateTimeOffset]::Parse(($lastCommit -split ' ')[0])
    Check 'El instalador es posterior al ultimo commit de producto y no hay cambios sin commit' (([DateTimeOffset](Get-Item $setup).LastWriteTime) -gt $commitTime -and $dirty -eq 0) "commit $($commitTime.ToString('HH:mm:ss zzz'))"
    $p = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
    Check 'Instalador /S termina con codigo 0' ($p.ExitCode -eq 0)
    Start-Sleep -Seconds 2
    $latin = [Text.Encoding]::GetEncoding(28591)
    $a = $latin.GetString([IO.File]::ReadAllBytes($AppExe))
    $b = $latin.GetString([IO.File]::ReadAllBytes($built)).Replace('__TAURI_BUNDLE_TYPE_VAR_UNK', '__TAURI_BUNDLE_TYPE_VAR_NSS')
    Log "Instalado ahora: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe); version del archivo: $([Diagnostics.FileVersionInfo]::GetVersionInfo($AppExe).ProductVersion)"
    Check 'El exe instalado es el de target\release (igual salvo la marca del empaquetado NSIS)' ([string]::Equals($a, $b, [StringComparison]::Ordinal)) "tamanos $($a.Length) / $($b.Length)"
    $a = $null; $b = $null
    $un = Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*' -ErrorAction SilentlyContinue | Where-Object { $_.DisplayName -like 'AlmarioKS*' }
    Check 'Clave de desinstalacion en HKCU con version 0.1.0' (@($un | Where-Object { $_.DisplayVersion -eq '0.1.0' }).Count -eq 1) "$(($un | ForEach-Object { "$($_.DisplayName) $($_.DisplayVersion)" }) -join ' ; ')"
    $pe = [IO.File]::ReadAllBytes($AppExe)
    $off = [BitConverter]::ToInt32($pe, 0x3C)
    $subsystem = [BitConverter]::ToUInt16($pe, $off + 24 + 68)
    Check 'El exe de release es de subsistema de ventanas (2), no de consola (3)' ($subsystem -eq 2) "subsistema = $subsystem"
}
Summary
