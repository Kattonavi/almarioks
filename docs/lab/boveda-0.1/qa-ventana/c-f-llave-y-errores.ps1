# C y F: archivo llave (crear, abrir, llave mala, archivo que no es boveda) y estados de error visibles.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
$uno = Join-Path $Work 'qa-uno.kdbx'; $dos = Join-Path $Work 'qa-dos.kdbx'; $keyx = Join-Path $Work 'qa-dos.keyx'
function Countdown { (Get-Names | Where-Object { $_ -match '^\d+:\d\d$' }) -join '' }
function Unlock-Msg { @(Get-Names (Find-El -Id 'unlock-error' -Timeout 2)) -join ' ' }
function Try-Unlock([string]$password) {
    Fill-Id 'unlock-password' $password -Secret
    Click-Name -Name 'Desbloquear'
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt 20) {
        Start-Sleep -Milliseconds 150
        if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.1) { return 'ABIERTA' }
        if ((Has-El -Name 'Desbloquear' -Type Button -Timeout 0.1) -and -not (Has-El -Like 'Abriendo la bóveda*' -Timeout 0.1)) { $m = Unlock-Msg; if ($m) { return $m } }
    }
    return '<sin respuesta en 20 s>'
}
Step 'F0 Cuenta de bloqueo y volver a 5 minutos' -Continue {
    if (Has-El -Name 'Desbloquear' -Type Button -Timeout 1) {
        Log '   la boveda se habia bloqueado sola (ajuste en 1 minuto): se desbloquea'
        $r = Try-Unlock $Master
        if ($r -ne 'ABIERTA') { throw "No se pudo desbloquear: $r" }
    }
    $i = Get-WinInfo
    $c0 = Countdown
    [void][QA.Native]::SetCursorPos($i.X + $i.OuterW + 300, $i.Y + $i.OuterH + 100)
    Start-Sleep -Seconds 6
    $c1 = Countdown
    Log "   cuenta con el ajuste en 1 minuto: $c0 -> (6 s sin tocar) -> $c1"
    Check 'La cuenta "Se bloquea en" baja mientras no se toca la ventana' ($c0 -ne $c1) "$c0 -> $c1"
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    [void](Select-Option 'settings-auto-lock' '5 minutos')
    Click-Name -Name 'Guardar ajustes'
    [void](Wait-Text 'Ajustes guardados.' 5)
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    $c2 = Countdown
    [void][QA.Native]::SetCursorPos($i.X + $i.OuterW + 300, $i.Y + $i.OuterH + 100)
    Start-Sleep -Seconds 3
    $c3 = Countdown
    Log "   cuenta justo despues de guardar 5 minutos: $c2; 3 s despues: $c3"
    Check 'Tras cambiar el ajuste, la cuenta visible usa el valor nuevo (5 minutos)' ($c2 -match '^[45]:' -and $c3 -match '^[45]:') "$c2 / $c3"
}
Step 'F1 Titulo vacio al guardar (RF-39)' -Continue {
    Select-Group 'qa-uno'
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text 'Nueva entrada en *' 5)
    Fill-Id 'entry-title' '   '
    Click-Name -Name 'Guardar entrada'
    Start-Sleep -Milliseconds 800
    $msg = @(Get-Names (Find-El -Id 'entry-title-error' -Timeout 2)) -join ' '
    Log "   mensaje: $msg  ·  foco: $(Focused-Desc)"
    Check 'Titulo vacio: mensaje en español que dice que hacer y la entrada no se crea' ([bool]$msg -and (Has-El -Name 'qa-uno · 0 entradas' -Type Button)) $msg
    Shot 'F01-titulo-vacio'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
}
Step 'F2 Busqueda sin resultados (RF-64)' -Continue {
    Fill (Find-El -Id 'vault-search') 'zzz-no-existe'
    Start-Sleep -Milliseconds 800
    $has = Has-El -Name 'Sin resultados.' -Timeout 3
    $clear = Has-El -Name 'Limpiar búsqueda' -Type Button -Timeout 1
    Shot 'F02-busqueda-sin-resultados'
    Check 'Sin resultados: lo dice y ofrece "Limpiar búsqueda"' ($has -and $clear) ((Get-Names | Where-Object { $_ -like '*resultado*' -or $_ -like '*coincid*' }) -join ' | ')
    if ($clear) { Click-Name -Name 'Limpiar búsqueda'; Start-Sleep -Milliseconds 400 }
    Check 'Limpiar vuelve al grupo' (Has-El -Id 'entries-title' -Like 'Entradas de *' -Timeout 3)
}
Step 'F3 Cerrar la boveda vuelve a la bienvenida (RF-36)' {
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Click-Name -Name 'Cerrar bóveda'
    Check 'Bienvenida tras cerrar la boveda' (Has-El -Id 'welcome-title' -Timeout 5)
}
Step 'F4 Crear donde ya existe un archivo (RF-4) y contrasenas que no coinciden (RF-8)' -Continue {
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    $hash = Sha $uno
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $uno -Replace
    Fill-Id 'create-password' $Master -Secret
    Fill-Id 'create-confirm' 'otra-cosa-distinta-QA' -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 800
    $msg = @(Get-Names (Find-El -Id 'create-error' -Timeout 2)) -join ' '
    Log "   no coinciden: $msg"
    Check 'Contrasenas que no coinciden: mensaje y no crea nada' ($msg -like '*no coinciden*') $msg
    Shot 'F04-contrasenas-no-coinciden'
    Fill-Id 'create-confirm' $Master -Secret
    Click-Name -Name 'Crear bóveda'
    $sw = [Diagnostics.Stopwatch]::StartNew(); $msg = ''
    while ($sw.Elapsed.TotalSeconds -lt 30) {
        Start-Sleep -Milliseconds 300
        if (Has-El -Name 'Bloquear' -Type Button -Timeout 0.1) { $msg = '<SE CREO Y SE ABRIO>'; break }
        $m = @(Get-Names (Find-El -Id 'create-error' -Timeout 1 -Quiet)) -join ' '
        if ($m -and -not (Has-El -Like 'Creando la bóveda*' -Timeout 0.1)) { $msg = $m; break }
    }
    Log "   archivo existente ($([int]$sw.ElapsedMilliseconds) ms): $msg"
    Shot 'F05-archivo-ya-existe'
    Check 'Archivo existente: se rechaza con un mensaje que dice que hacer' ($msg -and $msg -ne '<SE CREO Y SE ABRIO>') $msg
    Check 'Archivo existente: qa-uno.kdbx queda identico (SHA-256)' ((Sha $uno) -eq $hash) "$hash -> $(Sha $uno)"
}
Step 'C1 Segunda boveda con archivo llave generado (RF-5, RF-9)' {
    if (-not (Has-El -Id 'create-title' -Timeout 1)) { throw 'No estoy en la pantalla de crear' }
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $dos
    Fill-Id 'create-password' $Master -Secret
    Fill-Id 'create-confirm' $Master -Secret
    Click-El (Find-El -Id 'create-key-generate')
    Start-Sleep -Milliseconds 300
    Click-Name -Name 'Elegir dónde guardar el archivo llave…'
    Answer-FileDialog -path $keyx -shot 'C01-dialogo-nativo-archivo-llave'
    $warn = (Get-Names | Where-Object { $_ -like 'Sin una copia del archivo llave*' }) -join ''
    Log "   aviso antes de crear: $warn"
    Shot 'C02-crear-con-archivo-llave'
    $btn = Find-El -Name 'Crear bóveda' -Type Button
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-El $btn
    $ms = Wait-Text 'Bóveda creada' 40
    Log "TIEMPO crear con archivo llave (clic -> 'Bóveda creada'): $([int]$sw.ElapsedMilliseconds) ms"
    Check 'Se crea y muestra "Bóveda creada"' ($ms -ge 0)
    $notice = (Get-Names | Where-Object { $_ -like '*no hay forma de recuperarla*' -or $_ -like '*copia*archivo*' }) -join ' '
    Log "   aviso tras crear: $notice"
    Check 'RF-9: avisa que sin copia del archivo llave y sin la contrasena no se recupera' ($notice -like '*no hay forma de recuperarla*') $notice
    Shot 'C03-boveda-creada-aviso'
    $xml = if (Test-Path $keyx) { Get-Content -Raw $keyx } else { '' }
    Check 'El .keyx existe y es un archivo llave XML version 2.0 con Hash' ($xml -like '*<KeyFile>*' -and $xml -like '*<Version>2.0</Version>*' -and $xml -like '*Hash="*') "$((Get-Item $keyx -ErrorAction SilentlyContinue).Length) bytes"
    Click-Name -Name 'Ir a la bóveda'
    Check 'La boveda nueva queda abierta' (Has-El -Name 'Bloquear' -Type Button -Timeout 8)
    $script:Vault = $dos
    $e = New-Entry 'qa-dos' 'CANARIO-QA-ConLlave' 'x-QA-con-llave-1' 'ana'
    Check 'Entrada creada en la boveda con archivo llave' ($e.Msg -ge 0)
    Start-Sleep -Milliseconds 1500
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Click-Name -Name 'Cerrar bóveda'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
}
Step 'C2 Abrir con contrasena + archivo llave; llaves malas dan un solo mensaje igual (RF-11, ejemplo 7)' {
    Click-Name -Name 'Abrir qa-dos.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    $m1 = Try-Unlock $Master
    Log "   contrasena buena SIN archivo llave: $m1"
    Click-Name -Name 'Elegir archivo llave…'
    Answer-FileDialog -path $keyx -shot 'C04-dialogo-nativo-elegir-llave'
    Check 'El archivo llave elegido se muestra en la pantalla' ((Wait-Text '*qa-dos.keyx' 4) -ge 0)
    $m2 = Try-Unlock 'no-es-la-llave-QA'
    Log "   contrasena MALA con archivo llave: $m2"
    Shot 'C05-llave-incorrecta-con-archivo'
    $alerts = @(Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -like '*no abren esta bóveda*' })
    Check 'Llave mala: el mismo mensaje en los dos casos, sin decir que parte fallo' ($m1 -eq $m2 -and $m1 -like '*no abren esta bóveda*') $m2
    Check 'Llave mala: un solo mensaje en pantalla' ($alerts.Count -eq 1) "mensajes: $($alerts.Count)"
    $keyStill = Has-El -Like '*qa-dos.keyx' -Timeout 1
    Log "   tras el fallo, el archivo llave elegido sigue puesto = $keyStill"
    if (-not $keyStill) { Click-Name -Name 'Elegir archivo llave…'; Answer-FileDialog -path $keyx }
    $m3 = Try-Unlock $Master
    Check 'Contrasena + archivo llave correctos: abre' ($m3 -eq 'ABIERTA') $m3
    Check 'La entrada de la boveda con llave esta ahi' (Has-El -Name 'CANARIO-QA-ConLlave' -Type Button -Timeout 4)
    Shot 'C06-abierta-con-archivo-llave'
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Click-Name -Name 'Cerrar bóveda'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
}
Step 'C3 Un archivo de texto como "boveda" (RF-13)' -Continue {
    $fake = Join-Path $Work 'no-es-boveda.kdbx'
    [IO.File]::WriteAllText($fake, "esto es un archivo de texto cualquiera, no una boveda`r`n")
    Click-Name -Name 'Abrir una bóveda…'
    Answer-FileDialog -path $fake -shot 'C07-dialogo-nativo-abrir'
    Check 'Va a "Desbloquear no-es-boveda.kdbx"' (Has-El -Id 'unlock-title' -Like 'Desbloquear no-es-boveda.kdbx' -Timeout 5)
    $m = Try-Unlock 'cualquier-cosa-QA'
    Log "   archivo de texto: $m"
    Shot 'C08-archivo-danado'
    Check 'Archivo de texto: mensaje de archivo danado, en español y sin codigo' ($m -like '*dañado*' -or $m -like '*no es una bóveda*') $m
    Check 'La app sigue viva' ((Get-AppProc).Count -eq 1 -and (Has-El -Name 'Desbloquear' -Type Button))
    $empty = Join-Path $Work 'vacio.kdbx'
    [IO.File]::WriteAllBytes($empty, (New-Object byte[] 0))
    Click-Name -Name 'Volver a la bienvenida'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
    Click-Name -Name 'Abrir una bóveda…'
    Answer-FileDialog -path $empty
    $m0 = Try-Unlock 'cualquier-cosa-QA'
    Log "   archivo de 0 bytes: $m0"
    Check 'Archivo de 0 bytes: mensaje de archivo danado y la app sigue' (($m0 -like '*dañado*' -or $m0 -like '*no es una bóveda*') -and (Get-AppProc).Count -eq 1) $m0
    Click-Name -Name 'Volver a la bienvenida'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
}
Step 'F5 Boveda reciente que ya no existe (RF-20)' -Continue {
    Rename-Item $dos 'qa-dos.movida'
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 5)
    Click-Name -Name 'Volver a la bienvenida'; [void](Find-El -Id 'welcome-title' -Timeout 5)
    Start-Sleep -Milliseconds 600
    $text = (Get-Names | Where-Object { $_ -like '*ya no está*' -or $_ -like '*movido o*' }) -join ' '
    $find = Has-El -Name 'Buscar qa-dos.kdbx…' -Type Button -Timeout 2
    $forget = Has-El -Name 'Quitar qa-dos.kdbx de la lista' -Type Button -Timeout 1
    Log "   texto: $text"
    Shot 'F06-reciente-que-no-existe'
    Dump-Tree 'F06-bienvenida-reciente-perdida.uia.txt' | Out-Null
    Check 'Reciente perdida: lo dice y ofrece buscarla o quitarla' ($find -and $forget) "buscar=$find quitar=$forget"
    Rename-Item (Join-Path $Work 'qa-dos.movida') 'qa-dos.kdbx'
    Log "   recientes ahora: $((Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match '^(Abrir|Buscar|Quitar) ' } | ForEach-Object { $_.Current.Name }) -join ' | ')"
}
Show-Net 'fin de C y F'
Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length)" }
Summary
