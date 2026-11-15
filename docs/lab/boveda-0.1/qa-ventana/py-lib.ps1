# Ronda 2 del dueno (2026-11-12), puntos 5 y 6: variante de rf-lib.ps1 (RF_VARIANT=py-lib) para la lista en tabla y
# la ventana de la entrada en la app instalada. Carga px-lib.ps1 y px-pasos.ps1 y pone encima carpeta, canarios y
# ayudas nuevas: doble clic real, recorrido completo del arbol de accesibilidad y filas de la tabla.
# La carpeta de trabajo NO se borra al terminar: la usa el agente siguiente de la ronda.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
$script:Shots = Join-Path $StateDir 'capturas'
New-Item -ItemType Directory -Force $Shots | Out-Null
$script:FinalShots = Join-Path $Repo '.impeccable\review\ronda2\app'
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-py-20261112'
$script:RfMaster = 'PY-Maestra-Ronda2-8364-llave'
$script:RfVault = Join-Path $Work 'Cuentas.kdbx'
$script:Vault = $RfVault
$script:Master = $RfMaster
# Contrasenas canario de las entradas de muestra: mismo prefijo, longitudes distintas (8 a 40), sin separadores.
$script:PyPass = [ordered]@{
    'Correo de la oficina' = 'FAROPYcorreo41xz'; 'Figma' = 'FAROPYfigma7720qwer'; 'GitHub' = 'FAROPYgithub5306wq'
    'Jira' = 'FAROPYj8'; 'Nómina' = 'FAROPYnomina99'; 'Railway' = 'FAROPYrailway3141592'
    'Slack' = 'FAROPYslack0123456789abcdefghij012345678'; 'VPN de la oficina' = 'FAROPYvpn55aa'
    'Portal de clientes' = 'FAROPYportal8080'; 'Netflix' = 'FAROPYnetflix2718'; 'Gimnasio' = 'FAROPYgim12'
    'Banco de la esquina' = 'FAROPYbanco777777'; 'Tienda en línea' = 'FAROPYtienda4242'
}
function Purge-Mine { History-Purge @('FAROPY', 'centinela-py', 'almarioks-py', 'PY-Maestra') }
function Set-Location-Py([string]$file) {
    Start-Sleep -Milliseconds 900
    Click-Name -Name 'Cambiar ubicación…'
    if ((Find-FileDialog 4) -eq [IntPtr]::Zero) { Log '   (el dialogo nativo no aparecio con el primer clic: se repite)'; Start-Sleep -Milliseconds 500; Click-Name -Name 'Cambiar ubicación…' }
    Answer-FileDialog (Join-Path $Work $file); Start-Sleep -Milliseconds 600; [void](Focus-App)
}

# ---------- Raton real ----------
# Doble clic de verdad: dos pulsaciones en la misma posicion fisica, sin ninguna consulta UIA entre las dos.
function Dbl-At([int]$x, [int]$y, [int]$gapMs = 110) {
    [void](Focus-App)
    [void][QA.Native]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 180
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 25
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds $gapMs
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 25
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 200
}
function Click-At([int]$x, [int]$y) { [void](Focus-App); [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 150 }
# Centro fisico de un elemento (para el raton).
function Mid($el) { $r = $el.Current.BoundingRectangle; return @([int]($r.X + $r.Width / 2), [int]($r.Y + $r.Height / 2)) }

# ---------- Tabla ----------
# Todos los elementos del arbol cuyo nombre es exactamente $name y que caen a la derecha del riel.
function Els-Named([string]$name, [string]$type = '') {
    @(Get-All | Where-Object { $_.Current.Name -eq $name -and (-not $type -or (Type-Of $_) -eq $type) } | Where-Object { $r = LRect $_; $null -ne $r -and $r.X -ge 232 })
}
# Padre en la vista cruda hasta dar con un tipo.
function Up-To($el, [string]$type) {
    $cur = $el
    for ($k = 0; $k -lt 8 -and $null -ne $cur; $k++) { if ((Type-Of $cur) -eq $type) { return $cur }; $cur = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($cur) }
    return $null
}
function Kids($el) { @($el.FindAll($TreeScope::Children, [System.Windows.Automation.Condition]::TrueCondition)) }

# ---------- Recorrido completo del arbol de accesibilidad ----------
# Cada elemento de la ventana (vista cruda): nombre, valor (ValuePattern directo, sin saltarse los campos de
# contrasena), HelpText, ItemStatus, AutomationId, mas el texto del documento por TextPattern.
function Tree-Strings {
    $out = New-Object System.Collections.Generic.List[object]
    $root = Get-Root
    foreach ($el in @($root) + @(Get-All $root)) {
        $c = $el.Current; $t = Type-Of $el
        foreach ($pair in @(@('Name', $c.Name), @('HelpText', $c.HelpText), @('ItemStatus', $c.ItemStatus), @('AutomationId', $c.AutomationId), @('ItemType', $c.ItemType), @('AccessKey', $c.AccessKey))) {
            if ($pair[1]) { $out.Add([pscustomobject]@{ Where = "$($pair[0]) de [$t]"; Text = [string]$pair[1] }) }
        }
        $p = $null
        if ($el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$p)) { try { $v = $p.Current.Value; if ($v) { $out.Add([pscustomobject]@{ Where = "Value de [$t]"; Text = [string]$v }) } } catch { } }
        $tp = $null
        if ($t -eq 'Document' -and $el.TryGetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern, [ref]$tp)) { try { $txt = $tp.DocumentRange.GetText(200000); if ($txt) { $out.Add([pscustomobject]@{ Where = 'TextPattern del documento'; Text = [string]$txt }) } } catch { } }
    }
    return $out
}
# Busca agujas (etiqueta -> texto) en el recorrido. Devuelve "etiqueta @ donde" por cada hallazgo.
function Tree-Find([hashtable]$needles) {
    $all = Tree-Strings
    $hits = New-Object System.Collections.Generic.List[string]
    foreach ($k in $needles.Keys) {
        foreach ($s in $all) { if ($s.Text.Contains([string]$needles[$k])) { $hits.Add("$k @ $($s.Where)") } }
    }
    return [pscustomobject]@{ Count = $all.Count; Hits = @($hits | Select-Object -Unique) }
}
function Pass-Needles { $h = @{}; foreach ($k in $PyPass.Keys) { $h["contrasena de $k"] = $PyPass[$k] }; $h['prefijo FAROPY'] = 'FAROPY'; return $h }

# ---------- Portapapeles ----------
function Clip-State([string]$expect) {
    $t = "$(Get-Clip)"; $f = @(Get-ClipFormats)
    $excl = @('ExcludeClipboardContentFromMonitorProcessing', 'CanIncludeInClipboardHistory', 'CanUploadToCloudClipboard')
    $has = @($excl | Where-Object { $f -contains $_ })
    [pscustomobject]@{ Exact = ($t -ceq $expect); Len = $t.Length; Excl = $has.Count; Formats = ($f -join ', '); Text = $t }
}
