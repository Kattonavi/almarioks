# Ronda 2 del dueno (2026-11-12), puntos 1 a 4 en la app instalada: variante de rf-lib.ps1 (RF_VARIANT=pz-lib).
# Carga py-lib.ps1 y py-pasos.ps1 (boveda de muestra "Cuentas", raton real, recorrido del arbol) y pone encima la
# carpeta temporal propia de la boveda "Familia", sus maestras de prueba y ayudas del generador y de la llave.
# La salida de los pasos va a la carpeta de estado (RF_STATE): las contrasenas generadas se guardan ahi, no se imprimen.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
# Carpeta temporal propia en la carpeta publica del equipo: asi las rutas que la app pinta en la pantalla de
# desbloqueo (y que salen en la captura) no llevan el nombre del usuario de Windows. Se borra al terminar.
$script:Pz = Join-Path $env:PUBLIC 'almarioks-pz-20261112'
$script:DocsFam = Join-Path ([Environment]::GetFolderPath('MyDocuments')) 'AlmarioKS'
$script:FamDir = $Pz        # px8-rastro.ps1 la inspecciona como "familia"
$script:PzVault = Join-Path $Pz 'Familia.kdbx'
$script:PzKey = Join-Path $Pz 'Familia.keyx'
$script:PzMoved = Join-Path $Pz 'otra'
$script:PzMaster = 'PZ-Familia-Ronda2-5917-llave'
$script:PzMaster2 = 'PZ-Familia-Otra-3302-sinllave'
$script:PzTitle = 'FAROPZ-quinientos'
function Gen-List { @((Get-State 'generadas') -split "`n" | Where-Object { $_ }) }
function Purge-Mine { History-Purge (@('FAROPY', 'FAROPZ', 'centinela-py', 'centinela-pz', 'centinela-vn', 'almarioks-py', 'almarioks-pz', 'PY-Maestra', 'PZ-Familia') + @(Gen-List)) }
# Agujas del rastro en disco (px8-rastro.ps1 se reutiliza tal cual): la primera generada es la de 500 caracteres.
$script:Rf = @{ Title = $PzTitle; User = 'FAROPZ-usuario'; Notes = 'FAROPZ-notas reservadas'; Group = 'FAROPZ-Grupo'; Pass = $PyPass['Slack'] }
function Rf-Secrets {
    $plain = @{ 'titulo' = $Rf.Title; 'usuario' = $Rf.User; 'notas' = $Rf.Notes; 'grupo' = $Rf.Group; 'prefijo FAROPZ' = 'FAROPZ'; 'prefijo FAROPY' = 'FAROPY' }
    $k = 0
    foreach ($g in @(Gen-List)) { $k++; $plain["contrasena generada $k"] = $g }
    New-Secrets $plain @{ 'maestra' = $RfMaster; 'maestra de Familia' = $PzMaster; 'maestra de Familia sin llave' = $PzMaster2 }
}

# ---------- Arbol ----------
function Radios {
    @(Get-All | Where-Object { (Type-Of $_) -eq 'RadioButton' } | ForEach-Object {
            $p = $null; $sel = '?'
            if ($_.TryGetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern, [ref]$p)) { $sel = $p.Current.IsSelected }
            "$($_.Current.Name) = $sel" })
}
# Todos los nombres de la ventana (sin repetir), para leer avisos y ayudas.
function Names-Like([string]$rx) { @(Get-Names | Where-Object { $_ -match $rx } | Select-Object -Unique) }
function Dump-Here([string]$label) { Log "--- VOLCADO [$label]"; Print-Rects }
# Valor de un campo aunque sea de contrasena (solo para medir y comparar; nunca se registra).
function Raw-Value($el) {
    $p = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$p)) { return [string]$p.Current.Value }
    return $null
}
function Classes([string]$s) {
    $l = 0; $u = 0; $d = 0; $o = 0
    foreach ($ch in $s.ToCharArray()) { if ([char]::IsLower($ch)) { $l++ } elseif ([char]::IsUpper($ch)) { $u++ } elseif ([char]::IsDigit($ch)) { $d++ } else { $o++ } }
    return @($l, $u, $d, $o)
}
function Sha-Text([string]$s) { $h = [Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($s)); return (($h[0..5] | ForEach-Object { $_.ToString('x2') }) -join '') }

# ---------- Pantallas ----------
# Lleva la app a la bienvenida venga de donde venga (cierra dialogos; "Cerrar boveda" si hay una abierta o bloqueada).
function To-Welcome {
    if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'pz' }
    Close-Native
    [void](Focus-App)
    for ($k = 0; $k -lt 4 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) {
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet
        if ($null -ne $ask) { Click-El $ask } else { Press 'ESC' }
        Start-Sleep -Milliseconds 500
    }
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.5) { Click-Name -Name 'Bloquear'; [void](Find-El -Id 'unlock-password' -Timeout 10); Start-Sleep -Milliseconds 1300 }
    for ($k = 0; $k -lt 3 -and -not (Has-El -Id 'welcome-title' -Timeout 0.6); $k++) {
        $b = $null
        foreach ($n in @('Cerrar bóveda', 'Volver a la bienvenida', 'Cancelar')) { $b = Find-El -Name $n -Type Button -Timeout 0.3 -Quiet; if ($null -ne $b) { break } }
        if ($null -eq $b) { $b = Find-El -Like 'Cerrar*veda*' -Timeout 0.3 -Quiet }
        if ($null -eq $b) { $b = Find-El -Like '*otra b*veda*' -Timeout 0.3 -Quiet }
        if ($null -ne $b) { Click-El $b; Start-Sleep -Milliseconds 900 } else { break }
    }
    return (Has-El -Id 'welcome-title' -Timeout 2)
}
# Cierra con Escape lo nativo que haya quedado abierto (aviso de Windows y dialogo de archivo).
function Close-Native {
    for ($k = 0; $k -lt 4; $k++) {
        $h = Find-FileDialog 0.3
        if ($h -eq [IntPtr]::Zero) { break }
        [void][QA.Native]::SetForegroundWindow($h); Start-Sleep -Milliseconds 250; Press 'ESC'; Start-Sleep -Milliseconds 600
    }
}
# Responde el dialogo nativo PEGANDO la ruta (tecleada, Windows se comio una letra una vez) y comprueba lo escrito
# en el campo antes de aceptar.
function Pick-Dialog([string]$buttonName, [string]$path) {
    Start-Sleep -Milliseconds 600
    Click-Name -Name $buttonName
    if ((Find-FileDialog 4) -eq [IntPtr]::Zero) { Log '   (el dialogo nativo no aparecio con el primer clic: se repite)'; Start-Sleep -Milliseconds 500; Click-Name -Name $buttonName }
    $h = Find-FileDialog 6
    if ($h -eq [IntPtr]::Zero) { throw 'No aparecio el dialogo nativo de archivo' }
    Start-Sleep -Milliseconds 1200
    $dlg = $AE::FromHandle($h); $ok = $false
    for ($try = 0; $try -lt 3 -and -not $ok; $try++) {
        [void][QA.Native]::SetForegroundWindow($h); Start-Sleep -Milliseconds 300
        Set-ClipExternal $path; Start-Sleep -Milliseconds 200
        [QA.Native]::Chord(0x12, [byte][char]'N'); Start-Sleep -Milliseconds 250
        Press-Ctrl 'a'; Press-Ctrl 'v'; Start-Sleep -Milliseconds 500
        $ok = @($dlg.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -eq $path -or "$(Get-Value $_)" -eq $path }).Count -gt 0
    }
    [System.Windows.Forms.Clipboard]::Clear()
    Log "   dialogo nativo `"$($dlg.Current.Name)`": ruta pegada y leida de vuelta = $ok"
    if (-not $ok) { Close-Native; throw 'No se pudo escribir la ruta en el dialogo nativo' }
    Press 'ENTER'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ((Find-FileDialog 0.2) -ne [IntPtr]::Zero -and $sw.Elapsed.TotalSeconds -lt 8) { Start-Sleep -Milliseconds 150 }
    if ((Find-FileDialog 0.2) -ne [IntPtr]::Zero) { Close-Native; throw 'El dialogo nativo no se cerro' }
    Start-Sleep -Milliseconds 700; [void](Focus-App)
}
function Settings-Json { Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json }
