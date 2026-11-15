# Ronda 2 (2): llena la boveda de muestra "Cuentas" con las 13 entradas del prototipo, por la interfaz, con
# contrasenas canario tecleadas (longitudes distintas). Se puede repetir: salta los titulos que ya estan. Al final
# completa el sitio de GitHub por "Editar" si falta. RF_VARIANT=py-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
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
Step 'PY20 Boveda de muestra abierta' { Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search }
foreach ($e in $entries) {
    Step "PY21 Entrada $($e[1])" -Continue {
        Ensure-Cuentas
        Pick-Group $e[0]
        if (Has-Entry $e[1]) { Log "   ya existe: $($e[1])"; return }
        $r = New-EntryPy $e[1] $e[2] $e[3] $e[4] $PyPass[$e[1]]
        $site = Cell-Of $e[1] 'SITIO'; $siteName = if ($null -ne $site) { $site.Current.Name } else { '(sin celda)' }
        $want = ($e[3] -replace '^https://', ''); if (-not $want) { $want = [string][char]0x2014 }
        Check "Entrada creada: $($e[1]) en $($e[0])" ($r.Closed -and $r.Disk -and (Has-Entry $e[1]) -and $siteName -eq $want) "sitio tecleado=`"$($r.UrlTyped)`" (reintento=$($r.UrlRetry)); celda SITIO=`"$siteName`"; foco: $($r.FocusAfter)"
    }
}
Step 'PY22 Sitio de GitHub por Editar (si falta)' -Continue {
    Ensure-Cuentas; Pick-Group 'Trabajo'
    $site = Cell-Of 'GitHub' 'SITIO'
    if ($site.Current.Name -eq 'github.com') { Log '   GitHub ya tiene su sitio'; return }
    Pick-Entry 'GitHub'
    Click-Name -Name 'Abrir GitHub'; [void](Find-El -Name 'GitHub' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    Click-Name -Name 'Editar'; [void](Find-El -Name 'Editar entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 500
    Fill-Id 'entry-dialog-url' 'https://github.com'
    Log "   sitio tecleado en Editar: `"$(Get-Value (Find-El -Id 'entry-dialog-url'))`""
    Click-Name -Name 'Guardar entrada'; Start-Sleep -Milliseconds 1500
    Log "   tras guardar: ventana de la entrada abierta = $($null -ne (Entry-Window 'GitHub' 2)); nombres con github: $((Get-Names | Where-Object { $_ -like '*github.com*' }) -join ' | ')"
    Press 'ESC'; Start-Sleep -Milliseconds 600
    Check 'GitHub con su sitio en la lista' ((Cell-Of 'GitHub' 'SITIO').Current.Name -eq 'github.com') "celda: $((Cell-Of 'GitHub' 'SITIO').Current.Name)"
}
Step 'PY23 Conteos' -Continue {
    Ensure-Cuentas
    $tree = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name })
    Check 'Conteos del riel: 13 en total; Personal 2, Trabajo 8, Clientes 1, Bancos 1, Compras 1' (($tree -join '|') -eq 'Todas las entradas · 13 entradas|Personal · 2 entradas|Trabajo · 8 entradas|Clientes · 1 entrada|Bancos · 1 entrada|Compras · 1 entrada') ($tree -join ' | ')
}
Summary
