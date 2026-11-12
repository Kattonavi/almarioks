# Ronda 2 (9): cierre SIN limpieza. Deja la app instalada cerrada y el portapapeles y el historial sin canarios.
# NO borra la carpeta de trabajo ni quita la boveda de "recientes": las usa el agente siguiente de la ronda.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'PY90 Cierre' {
    if ((Any-AlmarioProc).Count -gt 0) {
        try { [void](Focus-App); for ($k = 0; $k -lt 3 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) { Press 'ESC'; Start-Sleep -Milliseconds 400 }; [void](Size-Window 1120 720) } catch { }
        $closed = Close-App 20; Start-Sleep -Milliseconds 1500
    }
    if ("$(Get-Clip)" -match 'FAROPY|centinela-py|PY-Maestra|almarioks-py') { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'App cerrada: ningun almarioks* vivo' ((Any-AlmarioProc).Count -eq 0)
    Check 'Portapapeles e historial sin canarios ni centinelas' (-not ("$(Get-Clip)" -match 'FAROPY|centinela-py|PY-Maestra') -and (History-Has 'FAROPY') -eq 'no' -and (History-Has 'centinela-py') -eq 'no' -and (History-Has 'PY-Maestra') -eq 'no') "borrados ahora del historial: $n"
    Check 'La boveda de muestra sigue en la carpeta de trabajo' (Test-Path $RfVault) "archivos: $((Get-ChildItem $Work | ForEach-Object { "$($_.Name) ($($_.Length) bytes)" }) -join ', ')"
    $s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
    Log "settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count) (antes de empezar: $(Get-State 'recientes-antes')); la de muestra esta en recientes = $(@($s.recent_vaults | Where-Object { $_ -like '*almarioks-py*' }).Count -gt 0)"
    Log "Instalado: $((Get-Item $InstalledExe).Length) bytes, version $([Diagnostics.FileVersionInfo]::GetVersionInfo($InstalledExe).ProductVersion)"
}
Summary
