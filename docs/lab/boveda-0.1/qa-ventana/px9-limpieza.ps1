# Pase de interaccion (9): limpieza. Con la app cerrada borra la carpeta de trabajo; abre la app y quita de "recientes"
# SOLO las dos bovedas de esta prueba (por su nombre exacto; las demas no se tocan ni se nombran); comprueba los
# ajustes (5 minutos, 20 s), deja el portapapeles y el historial sin canarios y la app instalada cerrada.
# Los archivos de Familia se borran antes con: px4-familia.ps1 borrar
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'PX90 Limpieza' {
    if ((Any-AlmarioProc).Count -gt 0) { [void](Close-App 20); Start-Sleep -Seconds 3 }
    if ((Any-AlmarioProc).Count -gt 0) { throw "Quedan procesos almarioks* vivos" }
    if ($Work -ne (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-px-20261112')) { throw 'La carpeta de trabajo no es la esperada' }
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    Check 'Archivos de Familia ya borrados' (-not (Test-Path -LiteralPath $FamVault) -and -not (Test-Path -LiteralPath $FamKey))
    Start-Checked 'limpieza'
    Start-Sleep -Milliseconds 900
    foreach ($name in @('Familia.kdbx', 'Cuentas.kdbx')) {
        $missing = (Get-Names | Where-Object { $_ -like "*$name*ya no*" -or $_ -like '*Puede que se haya movido*' } | Select-Object -First 1)
        $btn = Find-El -Like "Quitar $name*" -Type Button -Timeout 3 -Quiet
        if ($null -ne $btn) { Log "   reciente $name : boton `"$($btn.Current.Name)`""; Click-El $btn; Start-Sleep -Milliseconds 700 }
        else { Log "   reciente $name : no hay boton para quitarla" }
    }
    $mine = @(Get-Names | Where-Object { $_ -like '*Familia.kdbx*' -or $_ -like '*Cuentas.kdbx*' }).Count
    Check 'Recientes de prueba quitadas desde la app' ($mine -eq 0) "nombres de prueba que quedan en la bienvenida: $mine"
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes en 5 minutos y 20 s' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 300
    $closed = Close-App 20
    Start-Sleep -Milliseconds 1500
    if (Test-Path $Work) { Remove-Item -LiteralPath $Work -Recurse -Force }
    $clipNow = "$(Get-Clip)"
    if ($clipNow -match 'FAROPX|centinela-px|centinela-vn|PX-Maestra|PX-Familia|zz-marca') { [System.Windows.Forms.Clipboard]::Clear() }
    $n = Purge-Mine
    Check 'App cerrada, carpeta de trabajo borrada, ningun almarioks* vivo' ($closed -and -not (Test-Path $Work) -and (Any-AlmarioProc).Count -eq 0)
    Check 'Portapapeles e historial sin canarios ni centinelas de esta prueba' (-not ("$(Get-Clip)" -match 'FAROPX|centinela-px|PX-Maestra|PX-Familia') -and (History-Has 'FAROPX') -eq 'no' -and (History-Has 'centinela-px') -eq 'no' -and (History-Has 'PX-Maestra') -eq 'no') "borrados ahora del historial: $n"
}
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Check 'settings.json: 5 min, 20 s y las mismas recientes que antes de empezar (en numero)' ($s.auto_lock_minutes -eq 5 -and $s.clipboard_seconds -eq 20 -and "$(@($s.recent_vaults).Count)" -eq (Get-State 'recientes-antes')) "auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count) (antes: $(Get-State 'recientes-antes')); alguna de prueba = $(@($s.recent_vaults | Where-Object { $_ -like '*almarioks-px*' -or $_ -like '*\AlmarioKS\Familia.kdbx' }).Count -gt 0)"
Log "Instalado: $((Get-Item $InstalledExe).Length) bytes, sha256 $(Sha $InstalledExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($InstalledExe).ProductVersion); procesos almarioks* = $((Any-AlmarioProc).Count)"
Log "Documentos\AlmarioKS existe = $(Test-Path $FamDir) (antes de la corrida: $(Get-State 'familia-carpeta-antes'))"
$left = @(Get-ChildItem (Join-Path $env:LOCALAPPDATA 'Temp') -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -like 'almarioks-px*' })
Log "Restos de esta QA en %TEMP%: $($left.Count)"
Summary
