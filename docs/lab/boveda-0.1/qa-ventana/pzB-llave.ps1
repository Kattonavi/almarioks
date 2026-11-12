# Ronda 2, punto 1: el archivo llave en la app instalada. Argumento: crear (1e, 1a, 1b; b6 = solo 1b), mover (1d, 1c; b8 = solo 1c) o u1 (una
# boveda nueva en la misma ruta no hereda la llave recordada). Todo en la carpeta temporal propia ($Pz); en Documentos
# no se crea nada (la ubicacion propuesta solo se mira). RF_VARIANT=pz-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$only = if ($StepArgs.Count) { $StepArgs[0] } else { 'crear' }
function Group-Texts([string]$like) {
    $g = Find-El -Like $like -Type Group -Timeout 6
    @($g.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -and $_.Current.Name.Trim() } | ForEach-Object { "[$(Type-Of $_)] $($_.Current.Name)" })
}
function Key-State { (@(Group-Texts 'Desbloquear *') | Where-Object { $_ -notmatch 'Contraseña maestra|^\[Group\]|^\[Text\] Desbloquear|Usa la contraseña maestra|Llave de la bóveda' }) -join ' | ' }
function Open-Recent([string]$file) { Click-Name -Name "Abrir $file"; [void](Find-El -Id 'unlock-title' -Timeout 8); Start-Sleep -Milliseconds 1300 }
function Do-Unlock([string]$master, [double]$wait = 30) {
    Fill-Id 'unlock-password' $master -Secret
    Click-Name -Name 'Desbloquear'
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout $wait -Quiet)
    if ($ok) { Start-Sleep -Milliseconds 700 }
    return $ok
}
function Do-Lock { Click-Name -Name 'Bloquear'; [void](Find-El -Id 'unlock-password' -Timeout 10); Start-Sleep -Milliseconds 1300 }
function Open-Create { Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500 }
function Create-Lines { (@(Group-Texts 'Bóveda nueva') | Where-Object { $_ -match '^\[Text\]' -and $_ -notmatch 'Contraseña maestra|Nombre de la bóveda|Bóveda nueva|Si pierdes|Archivo llave \(opcional\)' } | ForEach-Object { $_ -replace '^\[Text\] ', '' }) -join '' }
function Flat([string]$t) { $t -replace '\s', '' }
# El estado de la llave en una sola tira sin espacios ni separadores (los textos llegan partidos en varios nodos).
function KF([string]$ks) { Flat (($ks -replace '\[\w+\] ', '') -replace ' \| ', '') }
function Finish-Create([string]$master) {
    Fill-Id 'create-password' $master -Secret
    Fill-Id 'create-confirm' $master -Secret
    Click-Name -Name 'Crear bóveda' -Type Button
    return (After-Create)
}
# Tras "Crear bóveda": con llave nueva sale antes la pantalla "Bóveda creada" con el boton "Ir a la bóveda".
function After-Create {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt 45) {
        if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.2) { Start-Sleep -Milliseconds 800; return $true }
        if (Has-El -Name 'Ir a la bóveda' -Type Button -Timeout 0.2) {
            Log "   PANTALLA intermedia: $((Group-Texts 'Bóveda creada') -join ' | ')"
            Pointer-Away; Shot-Client 'pz-b6-boveda-creada'
            Click-Name -Name 'Ir a la bóveda'
        }
        Start-Sleep -Milliseconds 200
    }
    return $false
}
function Pz-Files { (@(Get-ChildItem -LiteralPath $Pz -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object { $_.FullName.Substring($Pz.Length + 1) }) -join ', ') }
function Key-Memory {
    $s = Settings-Json
    $kf = $s.key_files
    if ($null -eq $kf) { return 'sin key_files' }
    return (($kf | ConvertTo-Json -Depth 5 -Compress) -replace [regex]::Escape(($Pz -replace '\\', '\\')), '<carpeta-pz>')
}

if ($only -eq 'b6') { $only = 'crear'; $skip = $true } else { $skip = $false }
if ($only -eq 'crear') {
if (-not $skip) {
Step 'PZB4 (1e) Crear sin tocar las opciones de llave' {
    New-Item -ItemType Directory -Force $Pz, $PzMoved | Out-Null
    Log "   Documentos\AlmarioKS existe antes = $(Test-Path $DocsFam)"
    [void](To-Welcome); Check 'Ventana a 1120 x 720' (Size-Window 1120 720)
    if (Test-Path -LiteralPath (Join-Path $Pz 'SinLlave.kdbx')) { Log '   (1e ya se comprobo en una corrida anterior: no se repite)'; return }
    Open-Create
    $r = Radios
    Check '(4) "Crear bóveda" recien abierta: marcada "Sin archivo llave"' (@($r | Where-Object { $_ -eq 'Sin archivo llave = True' }).Count -eq 1 -and @($r | Where-Object { $_ -like '* = True' }).Count -eq 1) ($r -join ' | ')
    Fill-Id 'create-name' 'SinLlave'
    Pick-Dialog 'Cambiar ubicación…' (Join-Path $Pz 'SinLlave.kdbx')
    $ok = Finish-Create $PzMaster2
    Check '(4) Crear sin tocar esas opciones NO crea ningun .keyx' ($ok -and (Test-Path (Join-Path $Pz 'SinLlave.kdbx')) -and @(Get-ChildItem -LiteralPath $Pz -Recurse -Filter '*.keyx').Count -eq 0) "boveda abierta = $ok; archivos en la carpeta: $(Pz-Files)"
    Do-Lock
    Log "   desbloqueo de SinLlave: $(Key-State)"
    [void](To-Welcome)
}
Step 'PZB5 (1a) Las dos rutas antes de crear' -Continue {
    [void](To-Welcome); Open-Create
    Fill-Id 'create-name' 'Familia'; Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 900
    Click-El (Find-El -Name 'Crear uno nuevo' -Type RadioButton); Start-Sleep -Milliseconds 700
    $l1 = Create-Lines
    Check '(5) Con "Crear uno nuevo" y el nombre Familia se ven las dos rutas' ((Flat $l1).Contains((Flat 'Se guarda en Documentos › AlmarioKS › Familia.kdbx')) -and (Flat $l1).Contains((Flat 'Se guarda junto a la bóveda: Documentos › AlmarioKS › Familia.keyx. Guarda una copia en otro lugar.'))) $l1
    $root = Get-Root; $maxB = 0; $off = @(); $worst = ''
    foreach ($el in @(Get-All $root)) { $r = LRect $el; if ($null -eq $r -or $r.W -le 0) { continue }; if ($r.B -gt $maxB) { $maxB = $r.B; $worst = "[$(Type-Of $el)] `"$($el.Current.Name)`"" }; if ($el.Current.IsOffscreen -and $el.Current.Name) { $off += "[$(Type-Of $el)] $($el.Current.Name)" } }
    $warn = Find-El -Like 'Si pierdes la contraseña maestra*' -Type Text; $wr = LRect $warn
    $scroll = @(Scroll-Report | Where-Object { $_.V -or $_.H })
    Check '(5) A 1120 x 720 no hay barra de desplazamiento ni aviso cortado' ($maxB -le 720 -and $off.Count -eq 0 -and $scroll.Count -eq 0 -and $wr.B -le 720) "borde inferior maximo del contenido = $maxB de 720 ($worst); elementos fuera de la vista: $($off.Count) $($off -join ', '); desplazables por UIA: $($scroll.Count); aviso `"Si pierdes…`": $(LRect-Text $warn); botones: Cancelar $(LRect-Text (Find-El -Name 'Cancelar' -Type Button)), Crear $(LRect-Text (Find-El -Name 'Crear bóveda' -Type Button))"
    Pointer-Away; Shot-Client 'app-crear-llave-1120x720'
    Fill-Id 'create-name' 'Hogar'; Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 900
    $l2 = Create-Lines
    Check '(5) Las dos rutas cambian al cambiar el nombre' ($l2.Contains('Hogar.kdbx') -and $l2.Contains('Hogar.keyx') -and -not $l2.Contains('Familia')) $l2
    Fill-Id 'create-name' 'Familia'; Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 600
    Pick-Dialog 'Cambiar ubicación…' $PzVault
    $l3 = Create-Lines
    Check '(5) Las dos rutas cambian con "Cambiar ubicación…"' ($l3.Contains('almarioks-pz-20261112') -and $l3.Contains('Familia.keyx') -and -not $l3.Contains('Documentos › AlmarioKS')) "$l3; nombre = `"$(Get-Value (Find-El -Id 'create-name'))`"; opciones: $((Radios) -join ' | ')"
    Check '(5) Mirar la ubicacion propuesta no creo nada en Documentos' (-not (Test-Path $DocsFam)) "Documentos\AlmarioKS existe = $(Test-Path $DocsFam)"
}
}
Step 'PZB6 (1b) Crear Familia con llave nueva; desbloquear la trae elegida' -Continue {
    if (Has-El -Id 'create-title' -Timeout 1) { $ok = Finish-Create $PzMaster } else { $ok = After-Create }
    Check '(6) Crear "Familia" con llave nueva deja Familia.kdbx y Familia.keyx juntos en la carpeta elegida' ($ok -and (Test-Path -LiteralPath $PzVault) -and (Test-Path -LiteralPath $PzKey) -and -not (Test-Path $DocsFam)) "boveda abierta = $ok; archivos: $(Pz-Files); llave de $((Get-Item -LiteralPath $PzKey -ErrorAction SilentlyContinue).Length) bytes; Documentos\AlmarioKS existe = $(Test-Path $DocsFam)"
    Do-Lock
    $ks = Key-State
    Check '(6) Bloquear y desbloquear: la llave viene ya elegida y se dice de donde salio' ($ks -like '*Familia.keyx*' -and $ks -like '*Se eligió sola la llave que está junto a la bóveda*' -and $ks -notlike '*Ninguno*') $ks
    Dump-Here 'desbloqueo con llave propuesta'
    Pointer-Away; Shot-Client 'app-desbloquear-llave-1120x720'
    $open = Do-Unlock $PzMaster
    Check '(6) Solo se escribe la contrasena: la boveda abre' $open "memoria de llaves en settings.json: $(Key-Memory)"
}
}

if ($only -eq 'b8') { $only = 'mover'; $skip7 = $true } else { $skip7 = $false }
if ($only -eq 'mover') {
if (-not $skip7) {
Step 'PZB7 (1d) Solo la contrasena en una boveda que usa llave' -Continue {
    [void](To-Welcome); Open-Recent 'Familia.kdbx'
    Log "   al abrir desde recientes: $(Key-State)"
    $q = Find-El -Like 'Quitar*' -Type Button -Timeout 2 -Quiet
    if ($null -eq $q) { Dump-Here 'no hay boton Quitar'; throw 'No se encontro como quitar la llave propuesta' }
    Log "   boton para quitarla: `"$($q.Current.Name)`""
    Click-El $q; Start-Sleep -Milliseconds 600
    $ks = Key-State
    $open = Do-Unlock $PzMaster 6
    $msg = @(Names-Like 'no coinciden|No se pudo|incorrect')
    Check '(7) Sin la llave, el mensaje es el de 1d' (-not $open -and ($msg -contains 'La contraseña o el archivo llave no coinciden. Si esta bóveda usa archivo llave, elígelo.')) "tras quitarla: $ks; abrio = $open; mensajes: $($msg -join ' | '); despues del fallo: $(Key-State)"
    Pointer-Away; Shot-Client 'pz-b7-mensaje'
    [void](To-Welcome); Open-Recent 'Familia.kdbx'
    $open2 = Do-Unlock $PzMaster
    Check '(7) CONTROL: la misma contrasena con la llave propuesta si abre' $open2
}
}
Step 'PZB8 (1c) La llave movida se recuerda' -Continue {
    [void](To-Welcome)
    if (Test-Path -LiteralPath $PzKey) { Move-Item -LiteralPath $PzKey -Destination (Join-Path $PzMoved 'Familia.keyx') }
    Log "   llave movida: $(Pz-Files); memoria antes: $(Key-Memory)"
    Open-Recent 'Familia.kdbx'
    $k0 = Key-State
    Check '(8) Con la llave movida ya no hay llave junto a la boveda: "Ninguno"' ($k0 -like '*Ninguno*' -and $k0 -notlike '*Se eligió sola*') $k0
    Pick-Dialog 'Elegir archivo llave…' (Join-Path $PzMoved 'Familia.keyx')
    Log "   tras elegirla con el dialogo: $(Key-State)"
    $open = Do-Unlock $PzMaster
    Check '(8) Eligiendola una vez con el dialogo, abre' $open "memoria: $(Key-Memory)"
    [void](To-Welcome); Open-Recent 'Familia.kdbx'
    $k1 = Key-State
    Check '(8) Cerrar y volver a abrir desde recientes: viene YA elegida, con su ruta completa y el aviso' ((KF $k1) -like '*almarioks-pz-20261112\otra\Familia.keyx*' -and (KF $k1) -like (Flat '*Se eligió sola la última llave con la que abriste esta bóveda*')) $k1
    Pointer-Away; Shot-Client 'pz-b8-recordada'
    $raw = Get-Content -Raw -Encoding UTF8 $SettingsFile
    $keyText = [IO.File]::ReadAllText((Join-Path $PzMoved 'Familia.keyx'))
    $data = ''; if ($keyText -match '(?s)<Data[^>]*>(.*?)</Data>') { $data = ($Matches[1] -replace '\s', '') }
    $hash = ''; if ($keyText -match 'Hash="([0-9A-Fa-f]+)"') { $hash = $Matches[1] }
    $flat = $raw -replace '\s', ''
    $s = Settings-Json
    Check '(8) settings.json guarda SOLO la ruta de la llave, nunca su contenido' ($data.Length -ge 32 -and -not $flat.ToUpper().Contains($data.ToUpper()) -and -not ($hash -and $flat.ToUpper().Contains($hash.ToUpper())) -and $raw.Contains('Familia.keyx')) "claves de settings.json: $(($s.PSObject.Properties.Name) -join ', '); key_files = $(Key-Memory); datos de la llave ($($data.Length) caracteres hex) en el archivo = $($flat.ToUpper().Contains($data.ToUpper())); su huella ($($hash.Length) hex) = $([bool]($hash -and $flat.ToUpper().Contains($hash.ToUpper()))); tamano de settings.json $($raw.Length)"
    $open2 = Do-Unlock $PzMaster
    Check '(8) Con la llave recordada solo se escribe la contrasena' $open2
    [void](To-Welcome)
    Rename-Item -LiteralPath (Join-Path $PzMoved 'Familia.keyx') -NewName 'Familia-renombrada.keyx'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Abrir Familia.kdbx'; [void](Find-El -Id 'unlock-title' -Timeout 10); $ms = [int]$sw.ElapsedMilliseconds; Start-Sleep -Milliseconds 1300
    $k2 = Key-State
    Check '(8) Renombrada la llave recordada: lo dice y pide elegir otra' ((KF $k2) -match 'Noencuentroelarchivollavequeusastelaúltimavez\(.*Familia\.keyx\)\.Conéctalooeligeotro\.') "$k2; pantalla de desbloqueo a los $ms ms"
    Pointer-Away; Shot-Client 'pz-b8-no-encuentro'
    $bad = Do-Unlock $PzMaster 5
    $msg = @(Names-Like 'no coinciden|No encuentro|No se pudo')
    $sw2 = [Diagnostics.Stopwatch]::StartNew(); $back = To-Welcome; $ms2 = [int]$sw2.ElapsedMilliseconds
    Check '(8) La pantalla no se congela: responde al intento de abrir y a "Cerrar bóveda"' ($back -and -not $bad) "abrio sin la llave = $bad; mensajes: $($msg -join ' | '); de vuelta en la bienvenida en $ms2 ms"
    Rename-Item -LiteralPath (Join-Path $PzMoved 'Familia-renombrada.keyx') -NewName 'Familia.keyx'
}
}

if ($only -eq 'u1') {
Step 'PZB9 (U-1) Una boveda nueva en la misma ruta no hereda la llave recordada' -Continue {
    [void](To-Welcome)
    $mem0 = Key-Memory
    $pre = (Test-Path -LiteralPath (Join-Path $PzMoved 'Familia.keyx')) -and -not (Test-Path -LiteralPath $PzKey) -and $mem0 -like '*otra*Familia.keyx*'
    Check '(9) PRECONDICION: la llave recordada existe en su ruta, esta en settings.json y no hay llave junto a la boveda' $pre "archivos: $(Pz-Files); memoria: $mem0"
    Open-Recent 'Familia.kdbx'; $kPre = Key-State
    Check '(9) CONTROL: antes de borrar, la boveda vieja SI trae propuesta la llave recordada' ((KF $kPre) -like '*otra\Familia.keyx*' -and (KF $kPre) -like (Flat '*Se eligió sola la última llave*')) $kPre
    [void](To-Welcome)
    foreach ($f in @($PzVault, "$PzVault.bak")) { if (Test-Path -LiteralPath $f) { Remove-Item -LiteralPath $f -Force } }
    Open-Create
    $r = Radios
    Fill-Id 'create-name' 'Familia'
    Pick-Dialog 'Cambiar ubicación…' $PzVault
    $ok = Finish-Create $PzMaster2
    $mem1 = Key-Memory
    Check '(9) Creada de nuevo en la MISMA ruta, con contrasena y "Sin archivo llave" (sin tocar las opciones)' ($ok -and (Test-Path -LiteralPath $PzVault) -and -not (Test-Path -LiteralPath $PzKey)) "opciones al abrir: $($r -join ' | '); archivos: $(Pz-Files); memoria despues de crear: $mem1"
    Do-Lock
    $k1 = Key-State
    Check '(9) Al desbloquearla NO viene propuesta la llave recordada de la anterior' ($k1 -like '*Ninguno*' -and $k1 -notlike '*Familia.keyx*' -and $k1 -notlike '*Se eligió sola*') $k1
    $open = Do-Unlock $PzMaster2
    Check '(9) Abre con solo su contrasena' $open
    [void](To-Welcome); Open-Recent 'Familia.kdbx'
    $k2 = Key-State
    Check '(9) Tampoco desde recientes, y la entrada ya no esta en settings.json' ($k2 -like '*Ninguno*' -and $k2 -notlike '*Familia.keyx*' -and (Key-Memory) -notlike '*Familia.keyx*') "$k2; memoria: $(Key-Memory)"
    [void](To-Welcome)
}
}
Summary
