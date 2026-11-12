# A (3/4): buscar por parte del titulo -> copiar la contrasena -> portapapeles real: contiene el valor, se limpia a los
# N s, y si otra app copia antes, se respeta (ejemplo 1 de la spec, casos a y b).
function Classes([string]$s) {
    $c = @()
    if ($s -cmatch '[a-z]') { $c += 'min' }; if ($s -cmatch '[A-Z]') { $c += 'may' }
    if ($s -match '\d') { $c += 'dig' }; if ($s -match '[^A-Za-z0-9]') { $c += 'sim' }
    return ($c -join '+')
}
Step 'A8 Buscar por parte del titulo' {
    $search = Find-El -Id 'vault-search'
    Fill $search 'titulo-corr'
    Check 'Aparece "Resultados de la búsqueda"' ((Wait-Text 'Resultados de la búsqueda' 5) -ge 0)
    Check 'La entrada sale en los resultados (plegado: minusculas)' (Has-El -Name $Canary.Title -Type Button -Timeout 3)
    Shot 'A11-busqueda-con-resultado'
    Click-Name -Name $Canary.Title
    Check 'Se abre el detalle con "Copiar contraseña"' (Has-El -Name 'Copiar contraseña' -Type Button -Timeout 4)
    Check 'La contrasena se ve oculta' (Has-El -Name '(oculto)' -Timeout 2)
}
Step 'A9 Copiar: el portapapeles tiene el valor y se limpia solo (ejemplo 1a)' {
    $seq0 = Get-ClipSeq
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    $script:Pw = Get-Clip
    $status = (Get-Names | Where-Object { $_ -like 'Copiado*' }) -join ' | '
    Log "   estado en la ventana: $status"
    $script:N = if ($status -match 'se borra en (\d+) s') { [int]$Matches[1] } else { 20 }
    Log "   portapapeles: $($Pw.Length) caracteres, tipos $(Classes $Pw); secuencia $seq0 -> $(Get-ClipSeq)"
    Log "   formatos en el portapapeles: $((Get-ClipFormats) -join ', ')"
    Check 'El portapapeles real contiene una contrasena de 20 caracteres con los 4 tipos' ($Pw.Length -eq 20 -and (Classes $Pw) -eq 'min+may+dig+sim') "$($Pw.Length) caracteres, $(Classes $Pw)"
    Check 'RF-74: la ventana dice en cuantos segundos se limpia' ([bool]$status) $status
    Check 'RF-72: lleva el formato ExcludeClipboardContentFromMonitorProcessing' ((Get-ClipFormats) -contains 'ExcludeClipboardContentFromMonitorProcessing')
    Shot 'A12-copiado-cuenta-atras'
    $cleared = -1
    while ($sw.Elapsed.TotalSeconds -lt ($N + 15)) {
        if ((Get-Clip) -eq '') { $cleared = $sw.Elapsed.TotalSeconds; break }
        Start-Sleep -Milliseconds 250
    }
    Log ("TIEMPO portapapeles vacio a los {0:n1} s de copiar (N = {1})" -f $cleared, $N)
    Check 'RF-73: el portapapeles queda vacio pasados N s' ($cleared -ge ($N - 1) -and $cleared -le ($N + 3)) ("{0:n1} s" -f $cleared)
    Log "   estado en la ventana despues: $((Get-Names | Where-Object { $_ -like '*cumplió*' -or $_ -like 'Copiado*' }) -join ' | ')"
    Shot 'A13-portapapeles-limpio'
}
Step 'A10 Mostrar la contrasena (RF-44) coincide con lo copiado' {
    Click-Name -Name 'Mostrar contraseña'
    Start-Sleep -Milliseconds 500
    $names = Get-Names
    Check 'El valor mostrado es el mismo que se copio' ($names -contains $Pw)
    Check 'El boton pasa a "Ocultar contraseña"' (Has-El -Name 'Ocultar contraseña' -Type Button -Timeout 2)
    Click-Name -Name 'Ocultar contraseña'
    Start-Sleep -Milliseconds 300
    Check 'Al ocultar, el valor sale del arbol UIA' (-not ((Get-Names) -contains $Pw))
}
Step 'A11 Otra app copia antes de la limpieza: se respeta (ejemplo 1b)' {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    Check 'Segunda copia: el portapapeles vuelve a tener la contrasena' ((Get-Clip) -ceq $Pw)
    Start-Sleep -Seconds 6
    Set-ClipExternal 'hola desde otra app (QA)'
    Log ("   a los {0:n1} s otra app (este script) copia un texto" -f $sw.Elapsed.TotalSeconds)
    while ($sw.Elapsed.TotalSeconds -lt ($N + 6)) { Start-Sleep -Milliseconds 500 }
    $now = Get-Clip
    Check 'RF-73: pasados N s el texto de la otra app sigue en el portapapeles' ($now -eq 'hola desde otra app (QA)') ("a los {0:n1} s" -f $sw.Elapsed.TotalSeconds)
    Log "   estado en la ventana: $((Get-Names | Where-Object { $_ -like '*cumplió*' -or $_ -like 'Copiado*' }) -join ' | ')"
}
Step 'A12 Limpiar la busqueda con Escape' {
    Click-El (Find-El -Id 'vault-search')
    Press 'ESC'
    Start-Sleep -Milliseconds 400
    Check 'Escape limpia la busqueda y vuelve el grupo' (Has-El -Id 'entries-title' -Like 'Entradas de qa-uno' -Timeout 3)
}
Summary
