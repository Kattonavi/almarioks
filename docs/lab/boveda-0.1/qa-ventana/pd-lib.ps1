# Decisiones del dueno (2026-11-15, RF-114 a RF-116) en la app instalada: variante de rf-lib.ps1 (RF_VARIANT=pd-lib).
# Carga pz-lib.ps1 (y con ella py-lib, px-lib, vn-lib: raton real, UIA, portapapeles e historial) y pone encima
# carpetas propias, canarios nuevos y las ayudas de esta corrida. Ninguna ruta del PC va escrita aqui: la carpeta base
# llega por la variable PD_DIR (fuera del repo) y se borra al terminar.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'pz-lib.ps1'), [Text.Encoding]::UTF8)))
if (-not $env:PD_DIR -or -not (Test-Path -LiteralPath $env:PD_DIR)) { throw 'Falta PD_DIR: carpeta base temporal de esta corrida' }
$script:PdBase = $env:PD_DIR
$script:Pz = Join-Path $PdBase 'boveda'
$script:FamDir = $Pz
$script:Work = Join-Path $PdBase 'work'
$script:Shots = Join-Path $StateDir 'capturas'
New-Item -ItemType Directory -Force $Shots, $Pz, $Work | Out-Null
$script:FinalShots = Join-Path $Repo '.impeccable\review\decisiones\app'
$script:PdName = 'Decisiones'
$script:Vault = Join-Path $Pz "$PdName.kdbx"
$script:RfVault = $Vault
$script:RfMaster = 'PD-Decisiones-Maestra-4471'
$script:Master = $RfMaster
$script:PdShort = 'FAROPD-corta'
$script:PdShortPass = 'FAROPDcorta4417'
$script:PdLong = 'FAROPD-quinientos'
function Purge-Mine { History-Purge (@('FAROPD', 'centinela-pd', 'centinela-vn', 'PD-Decisiones') + @(Gen-List)) }

# Deja abierta la boveda de esta corrida venga de donde venga (como Ensure-Cuentas, con su nombre).
function Ensure-Pd {
    if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'pd' }
    [void](Focus-App)
    for ($k = 0; $k -lt 4 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) {
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet
        if ($null -ne $ask) { Click-El $ask } else { Press 'ESC' }
        Start-Sleep -Milliseconds 500
    }
    if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.6) { return }
    if (Has-El -Id 'welcome-title' -Timeout 0.6) {
        Click-Name -Name "Abrir $PdName.kdbx"
        [void](Find-El -Id 'unlock-title' -Timeout 6); Start-Sleep -Milliseconds 400
    }
    Unlock-Pd
    if (-not (Has-El -Name 'Bloquear' -Type Button -Timeout 3)) { throw 'No se pudo dejar abierta la boveda de prueba' }
}
function Unlock-Pd {
    if (Has-El -Id 'unlock-password' -Timeout 3) {
        Fill-Id 'unlock-password' $RfMaster -Secret
        Click-Name -Name 'Desbloquear'
        [void](Find-El -Name 'Bloquear' -Type Button -Timeout 30); Start-Sleep -Milliseconds 700
    }
}
# ¿Esta la pantalla de desbloqueo? (y la boveda ya no esta a la vista)
function Is-Locked { (Has-El -Id 'unlock-password' -Timeout 0.4) -and -not (Has-El -Name 'Bloquear' -Type Button -Timeout 0.2) }
function Wait-Locked([double]$sec = 4) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $sec) { if (Is-Locked) { return [int]$sw.ElapsedMilliseconds }; Start-Sleep -Milliseconds 100 }
    return -1
}
# Rueda real en un punto fisico (notches > 0 = hacia arriba).
function Wheel-At([int]$x, [int]$y, [int]$notches) {
    [void](Focus-App)
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 150
    $step = if ($notches -gt 0) { 120 } else { -120 }
    for ($k = 0; $k -lt [math]::Abs($notches); $k++) {
        [QA.Native]::mouse_event(0x0800, 0, 0, [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]$step), 0), [UIntPtr]::Zero); Start-Sleep -Milliseconds 90
    }
    Start-Sleep -Milliseconds 450
}
# Y logica de un control (o '-' si no esta).
function Y-Of($el) { if ($null -eq $el) { return '-' }; $r = LRect $el; if ($null -eq $r) { return '-' }; return $r.Y }
# Ayuda del control: HelpText y, si viene vacia, FullDescription (UIA 30159).
function Help-Of($el) {
    $h = "$($el.Current.HelpText)"
    if (-not $h) { try { $p = [System.Windows.Automation.AutomationProperty]::LookupById(30159); if ($null -ne $p) { $h = "$($el.GetCurrentPropertyValue($p))" } } catch { } }
    return $h
}
