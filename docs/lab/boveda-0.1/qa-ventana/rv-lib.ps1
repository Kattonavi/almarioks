# Reverificacion de D-1 y D-2 (modo privado de WebView2): datos y ayudas comunes de los pasos rv*.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
# Canarios NUEVOS, distintos de los de la primera corrida.
$script:RvMaster = "RV-Maestra-Nueva-83-$([char]0x00F1)$([char]0x20AC)"
$script:RvOther = 'RV-Repeticion-Distinta-51'
$script:Rv = @{
    Title = 'CENTINELA-RV-Titulo'; User = 'CENTINELA-RV-usuario@example.org'; Url = 'https://example.org/CENTINELA-RV-url'
    Notes = 'CENTINELA-RV-notas privadas'; Tag = 'CENTINELA-RV-etiqueta'; Group = 'CENTINELA-RV-Grupo'
    Search = 'CENTINELA-RV-busqueda'; Pass = 'CENTINELA-RV-Clave-64'
}
$script:RvVault = Join-Path $Work 'rv-uno.kdbx'
$script:OldCopy = Join-Path $Work 'anterior'
$script:EbDir = Join-Path $DataDir 'EBWebView'
function New-Secrets([hashtable]$plain, [hashtable]$hashed) {
    $json = @{ plain = $plain; hashed = $hashed } | ConvertTo-Json -Depth 4 -Compress
    return [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
}
function Rv-NewSecrets {
    New-Secrets @{ 'titulo' = $Rv.Title; 'usuario' = $Rv.User; 'URL' = $Rv.Url; 'notas' = $Rv.Notes; 'etiqueta' = $Rv.Tag; 'grupo' = $Rv.Group
        'busqueda' = $Rv.Search; 'contrasena de la entrada' = $Rv.Pass; 'prefijo CENTINELA-RV' = 'CENTINELA-RV' } @{ 'maestra nueva' = $RvMaster; 'repeticion distinta nueva' = $RvOther }
}
function Rv-OldSecrets {
    New-Secrets @{ 'titulo (corrida anterior)' = $Canary.Title; 'usuario (corrida anterior)' = $Canary.User; 'notas (corrida anterior)' = $Canary.Notes } @{ 'maestra de la corrida anterior' = $Master; 'repeticion distinta de la corrida anterior' = 'otra-cosa-distinta-QA' }
}
# Corre el inspector y devuelve sus lineas (ya registradas). $roots: "nombre=carpeta".
function Run-Inspector([string]$secrets, [string[]]$roots, [string]$since = '', [string[]]$exclude = @(), [switch]$Tree) {
    $env:RV_SECRETS = $secrets; $env:RV_SINCE = $since; $env:RV_EXCLUDE = ($exclude -join '|'); $env:RV_TREE = $(if ($Tree) { '1' } else { '' })
    $lines = @(& py -3 -I (Join-Path $QaDir 'rv-inspeccionar.py') @roots 2>&1 | ForEach-Object { "$_" })
    $env:RV_SECRETS = ''
    foreach ($l in $lines) { Log "   $l" }
    return $lines
}
function Hits-Of($lines, [string]$root) {
    $m = $lines | Where-Object { $_ -like "RESUMEN|$root|*apariciones=*" } | Select-Object -First 1
    if ($m -and $m -match 'apariciones=(\d+)') { return [int]$Matches[1] }
    return -1
}
