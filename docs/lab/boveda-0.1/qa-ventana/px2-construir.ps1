# Pase de interaccion (2): llena la boveda de muestra "Cuentas" con las 13 entradas del prototipo
# (docs/diseno/prototipo-0.1/Main.dc.html, base()), por la interfaz: Nueva entrada -> campos -> Generar -> Guardar.
# Se puede repetir: salta los titulos que ya estan. Argumento opcional: cuantas crear como maximo en esta pasada.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
$max = if ($StepArgs.Count) { [int]$StepArgs[0] } else { 99 }
$o = [char]0x00F3; $a = [char]0x00E1; $i2 = [char]0x00ED
$entries = @(
    @('Trabajo', 'Correo de la oficina', 'ana@example.com', 'https://correo.example', ''),
    @('Trabajo', 'Figma', 'ana@example.com', 'https://figma.example', ''),
    @('Trabajo', 'GitHub', 'ana@example.com', 'https://github.com', "Cuenta de la organizaci${o}n. Renovar el token de acceso en enero."),
    @('Trabajo', 'Jira', 'ana.g', 'https://jira.example', ''),
    @('Trabajo', "N${o}mina", '1032456789', 'https://nomina.example', ''),
    @('Trabajo', 'Railway', 'ana@example.com', 'https://railway.example', ''),
    @('Trabajo', 'Slack', 'ana@example.com', 'https://slack.example', ''),
    @('Trabajo', 'VPN de la oficina', 'agomez', '', "Se conecta solo desde el port${a}til de la empresa."),
    @('Clientes', 'Portal de clientes', 'soporte@example.com', 'https://clientes.example', ''),
    @('Personal', 'Netflix', 'ana.casa@example.com', 'https://netflix.example', ''),
    @('Personal', 'Gimnasio', 'ana.casa@example.com', 'https://gimnasio.example', ''),
    @('Bancos', 'Banco de la esquina', '1032456789', 'https://banco.example', ''),
    @('Compras', "Tienda en l${i2}nea", 'ana.casa@example.com', 'https://tienda.example', '')
)
Step 'PX10 Boveda de muestra abierta' { Ensure-Cuentas; [void](Size-Window 1120 720) }
$made = 0
foreach ($e in $entries) {
    if ($made -ge $max) { break }
    Step "PX11 Entrada $($e[1])" -Continue {
        Ensure-Cuentas
        Pick-Group $e[0]
        if (Has-Entry $e[1]) { Log "   ya existe: $($e[1])"; return }
        $r = New-EntryPx $e[1] $e[2] $e[3] $e[4]
        $script:made++
        Check "Entrada creada: $($e[1]) en $($e[0])" ($r.Closed -and (Has-Entry $e[1])) "clics: Nueva entrada + Generar + Guardar; dialogo cerrado a los $($r.CloseMs) ms; disco a los $($r.DiskMs) ms; generada visible = $($r.GenVisible) ($($r.GenLen) caracteres)"
    }
}
Step 'PX12 Conteos' -Continue {
    Ensure-Cuentas
    $tree = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name })
    Log "   RIEL: $($tree -join '  |  ')"
}
Summary
