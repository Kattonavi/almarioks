# Pase de interaccion (1): con la app cerrada, instalar en silencio el instalador de release sobre lo que haya y
# comprobar que el exe instalado es el de target\release. No borra nada (ni EBWebView ni ajustes).
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$setup = Join-Path $Repo 'target\release\bundle\nsis\AlmarioKS_0.1.0_x64-setup.exe'
$built = Join-Path $Repo 'target\release\almarioks.exe'
Step 'PX1 Estado previo' {
    if ((Any-AlmarioProc).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    Set-State 'inicio' ((Get-Date).ToString('o'))
    Set-State 'familia-carpeta-antes' ([string](Test-Path $FamDir))
    Log "Instalado antes: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe)"
    Log "Carpeta AlmarioKS en Documentos existia antes = $(Test-Path $FamDir)"
    $s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
    Set-State 'recientes-antes' ([string]@($s.recent_vaults).Count)
    Log "settings.json antes: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
}
Step 'PX2 Instalar el instalador de release' {
    $lastCommit = (& git -C $Repo log -1 --format='%cI %h' -- src src-tauri crates Cargo.toml package.json)
    $dirty = @(& git -C $Repo status --short -- src src-tauri crates).Count
    Log "Rama: $(& git -C $Repo branch --show-current); HEAD $(& git -C $Repo log -1 --format='%h %s')"
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
}
Summary
