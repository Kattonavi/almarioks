# G (y lo que quedo de F): archivo existente y contrasenas distintas; teclado (desbloquear y copiar sin raton);
# cuenta de bloqueo sin tocar; capturas a 800x600, al minimo y maximizada.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
$uno = Join-Path $Work 'qa-uno.kdbx'
function Countdown { (Get-Names | Where-Object { $_ -match '^\d+:\d\d$' }) -join '' }
# Pulsa Tab hasta que el foco este en un control cuyo nombre case; devuelve el recorrido.
function Tab-Until([string]$like, [int]$max = 30) {
    $path = New-Object System.Collections.Generic.List[string]
    for ($k = 0; $k -lt $max; $k++) {
        Press 'TAB'; Start-Sleep -Milliseconds 120
        $d = Focused-Desc; $path.Add($d)
        if ($d -like $like) { return [pscustomobject]@{ Found = $true; Path = $path } }
    }
    return [pscustomobject]@{ Found = $false; Path = $path }
}
function Size-Note([string]$tag) {
    $i = Get-WinInfo
    $off = @(Get-All | Where-Object { $_.Current.IsOffscreen -and (Type-Of $_) -in @('Button', 'Edit', 'ComboBox') -and $_.Current.Name } | ForEach-Object { $_.Current.Name } | Select-Object -Unique)
    Log "   [$tag] interior logico $($i.LogicalW)x$($i.LogicalH); controles que piden desplazarse ($($off.Count)): $($off -join ' | ')"
}

Step 'F4 Crear donde ya existe un archivo (RF-4) y contrasenas que no coinciden (RF-8)' -Continue {
    if (-not (Has-El -Id 'welcome-title' -Timeout 2)) { throw 'No estoy en la bienvenida' }
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    $hash = Sha $uno
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $uno -Replace
    Check 'Tras aceptar el aviso de Windows, la ruta existente queda elegida y el archivo sigue igual' (((Wait-Text '*qa-uno.kdbx' 4) -ge 0) -and (Sha $uno) -eq $hash)
    Fill-Id 'create-password' $Master -Secret
    Fill-Id 'create-confirm' 'otra-cosa-distinta-QA' -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 800
    $msg = @(Get-Names (Find-El -Id 'create-error' -Timeout 2)) -join ' '
    Log "   no coinciden: $msg  ·  foco: $(Focused-Desc)"
    Check 'Contrasenas que no coinciden: mensaje y no crea nada (RF-8)' ($msg -like '*no coinciden*') $msg
    Shot 'F04-contrasenas-no-coinciden'
    Fill-Id 'create-password' $Master -Secret
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
    Check 'Archivo existente: se rechaza con un mensaje que dice que hacer (RF-4)' ($msg -and $msg -ne '<SE CREO Y SE ABRIO>') $msg
    Check 'Archivo existente: qa-uno.kdbx queda identico (SHA-256)' ((Sha $uno) -eq $hash) "$hash -> $(Sha $uno)"
    Click-Name -Name 'Volver a la bienvenida'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
}
Step 'G1 Teclado: de la bienvenida a la boveda abierta sin raton' -Continue {
    if (-not (Has-El -Id 'welcome-title' -Timeout 2)) { throw 'No estoy en la bienvenida' }
    Click-El (Find-El -Id 'welcome-title')     # un clic en el titulo para dejar el foco en la pagina; desde aqui, solo teclado
    $t = Tab-Until '*Abrir qa-uno.kdbx*' 15
    Log "   orden de Tab en la bienvenida: $($t.Path -join '  ->  ')"
    Check 'Bienvenida: "Abrir qa-uno.kdbx" se alcanza con Tab' $t.Found
    Shot 'G08-foco-bienvenida'
    Press 'ENTER'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Start-Sleep -Milliseconds 500
    $f = Focused-Desc
    Log "   foco al llegar a Desbloquear: $f"
    Check 'Desbloquear: el foco cae solo en la contrasena' ($f -like '*unlock-password*')
    $order = New-Object System.Collections.Generic.List[string]
    for ($k = 0; $k -lt 5; $k++) { Press 'TAB'; Start-Sleep -Milliseconds 120; $order.Add((Focused-Desc)); if ($k -eq 1) { Shot 'G09-foco-desbloquear' } }
    Log "   orden de Tab en Desbloquear: $($order -join '  ->  ')"
    Click-El (Find-El -Id 'unlock-password')   # vuelve al campo (un clic) para escribir
    Type-Text $Master -Secret
    Press 'ENTER'
    Check 'Desbloquear con Enter desde el campo' ($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 20 -Quiet))
}
Step 'G2 Teclado: buscar y copiar sin raton (tarea principal)' -Continue {
    Start-Sleep -Milliseconds 600
    Press-Ctrl 'k'
    Start-Sleep -Milliseconds 300
    $f = Focused-Desc
    Check 'Ctrl+K lleva el foco a la busqueda' ($f -like '*vault-search*') $f
    Type-Text 'correo'
    Start-Sleep -Milliseconds 800
    $t = Tab-Until "*$($Canary.Title)*" 12
    Log "   Tab desde la busqueda hasta el resultado: $($t.Path -join '  ->  ')"
    Check 'El resultado se alcanza con Tab' $t.Found
    Press 'ENTER'
    Start-Sleep -Milliseconds 700
    Log "   foco tras abrir la entrada con Enter: $(Focused-Desc)"
    $t2 = Tab-Until '*Copiar contraseña*' 20
    Log "   Tab hasta 'Copiar contraseña' ($($t2.Path.Count) pulsaciones): $($t2.Path -join '  ->  ')"
    Check '"Copiar contraseña" se alcanza con Tab' $t2.Found
    Shot 'G10-foco-copiar'
    Press 'ENTER'
    Start-Sleep -Milliseconds 500
    Check 'Copiar con Enter: el portapapeles tiene la contrasena' ((Get-Clip) -ceq $Canary.Pass2)
    Shot 'G04-boveda-detalle-800x600'
    Dump-Tree 'G04-boveda-detalle.uia.txt' | Out-Null
    Size-Note '800x600 boveda con detalle'
}
Step 'G3 La cuenta "Se bloquea en" sin tocar la ventana' -Continue {
    $i = Get-WinInfo
    [void][QA.Native]::SetCursorPos($i.X + $i.OuterW + 300, $i.Y + $i.OuterH + 100)
    $c0 = Countdown; Start-Sleep -Seconds 12; $c1 = Countdown; Start-Sleep -Seconds 12; $c2 = Countdown
    Log "   cuenta: $c0 -> (12 s) $c1 -> (24 s) $c2   (ajuste real: 5 minutos)"
    Check 'La cuenta visible baja con el tiempo y refleja el ajuste vigente' ($c0 -ne $c2 -and $c0 -match '^[45]:') "$c0 / $c1 / $c2"
}
Step 'G4 Tamano minimo (480x360) y maximizada' -Continue {
    $i = Set-WinClient 300 200
    Log "   pedido 300x200 -> interior logico $($i.LogicalW)x$($i.LogicalH)"
    Check 'El minimo de la ventana es 480x360' ($i.LogicalW -eq 480 -and $i.LogicalH -eq 360) "$($i.LogicalW)x$($i.LogicalH)"
    Shot 'G04-boveda-detalle-480x360'
    Size-Note '480x360 boveda con detalle'
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    Check 'Al minimo: copiar la contrasena sigue siendo posible con el raton' ((Get-Clip) -ceq $Canary.Pass2)
    Shot 'G04b-boveda-copiado-480x360'
    Click-Name -Name 'Editar'
    [void](Wait-Text 'Editar *' 4)
    Click-Name -Name 'Generar contraseña'
    Start-Sleep -Milliseconds 400
    Click-Name -Name 'Generar'
    Start-Sleep -Milliseconds 600
    Shot 'G05-formulario-generador-480x360'
    Size-Note '480x360 formulario con generador'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 400
    Select-Group 'Archivo'
    Click-Name -Name 'Borrar grupo…'
    Start-Sleep -Milliseconds 500
    Shot 'G07-dialogo-borrar-480x360'
    Size-Note '480x360 dialogo de borrado'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    Shot 'G06-ajustes-480x360'
    Size-Note '480x360 ajustes'
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    [void][QA.Native]::ShowWindow((Get-AppHwnd), 3)
    Start-Sleep -Milliseconds 800
    Open-Entry 'Personal QA' $Canary.Title
    Shot 'G04-boveda-detalle-maximizada'
    Size-Note 'maximizada'
    [void](Set-WinClient 800 600)
    Select-Group 'Archivo'
    Click-Name -Name 'Borrar grupo…'
    Start-Sleep -Milliseconds 500
    Shot 'G07-dialogo-borrar-800x600'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
    Open-Entry 'Personal QA' $Canary.Title
    Click-Name -Name 'Editar'
    [void](Wait-Text 'Editar *' 4)
    Click-Name -Name 'Generar contraseña'
    Start-Sleep -Milliseconds 400
    Shot 'G05-formulario-generador-800x600'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
}
Step 'G5 Desbloquear y bienvenida al minimo' -Continue {
    [void](Set-WinClient 480 360)
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Shot 'G02-desbloquear-480x360'
    Size-Note '480x360 desbloquear'
    Click-Name -Name 'Cerrar bóveda'
    [void](Find-El -Id 'welcome-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    Shot 'G01-bienvenida-480x360'
    Size-Note '480x360 bienvenida'
}
$i = Set-WinClient 800 600
Log "Ventana restaurada: interior logico $($i.LogicalW)x$($i.LogicalH)"
Show-Net 'fin de G'
Summary
