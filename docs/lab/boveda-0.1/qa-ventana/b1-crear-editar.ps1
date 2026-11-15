# B (1/2): GRUPOS crear, ver, renombrar · ENTRADAS crear, ver, editar solo el titulo (ejemplo 8).
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'B1 Grupos: crear y ver en el arbol' {
    Show-Tree 'inicio'
    Check 'GRUPO crear: "Trabajo" en la raiz' (New-Group 'qa-uno' 'Trabajo') (Status-Text)
    Shot 'B01-grupo-creado'
    Check 'GRUPO crear: "Clientes" dentro de Trabajo' (New-Group 'Trabajo' 'Clientes')
    Check 'GRUPO crear: "Personal" en la raiz' (New-Group 'qa-uno' 'Personal')
    Show-Tree 'tras crear 3 grupos'
    Dump-Tree 'B01-arbol.uia.txt' | Out-Null
}
Step 'B2 Grupo: nombre vacio se rechaza' -Continue {
    Select-Group 'qa-uno'
    Click-Name -Name 'Nuevo grupo…'
    Fill-Id 'group-name' '   '
    Click-Name -Name 'Crear grupo'
    Start-Sleep -Milliseconds 600
    $msg = @(Get-Names (Find-El -Id 'group-name-error' -Timeout 2)) -join ' '
    Log "   mensaje: $msg"
    Check 'GRUPO nombre vacio: mensaje y no se crea (RF-52)' ([bool]$msg) $msg
    Shot 'F03-grupo-nombre-vacio'
    Click-Name -Name 'Cancelar'
    Start-Sleep -Milliseconds 300
}
Step 'B3 Grupo: renombrar' {
    Select-Group 'Personal'
    Click-Name -Name 'Renombrar grupo…'
    Shot 'B02-dialogo-renombrar'
    Fill-Id 'group-name' 'Personal QA'
    Click-Name -Name 'Guardar nombre'
    Start-Sleep -Milliseconds 700
    Check 'GRUPO renombrar: "Personal" -> "Personal QA"' ((Has-El -Like 'Personal QA · *' -Type Button -Timeout 4) -and -not (Has-El -Name 'Personal · 0 entradas' -Type Button)) (Status-Text)
    Show-Tree 'tras renombrar'
}
Step 'B4 Entradas: crear en grupos (cronometro de guardado)' {
    $a = New-Entry 'Trabajo' 'CANARIO-QA-Banco' $Canary.Pass2 'ana'
    Check 'ENTRADA crear en "Trabajo" con contrasena escrita' ($a.Msg -ge 0 -and $a.Heading -eq 'Nueva entrada en Trabajo') "mensaje $($a.Msg) ms, disco $($a.Disk) ms, Guardado $($a.Saved) ms"
    $b = New-Entry 'Clientes' 'CANARIO-QA-Cliente' 'x-QA-cliente-1'
    Check 'ENTRADA crear en "Clientes"' ($b.Msg -ge 0) "mensaje $($b.Msg) ms, disco $($b.Disk) ms, Guardado $($b.Saved) ms"
    $c = New-Entry 'qa-uno' 'CANARIO-QA-Desechable' 'x-QA-desechable-1'
    Check 'ENTRADA crear en la raiz' ($c.Msg -ge 0) "mensaje $($c.Msg) ms, disco $($c.Disk) ms, Guardado $($c.Saved) ms"
    Check 'D-1: "Entrada guardada." no sale antes de que el archivo este reemplazado' ($a.Msg -ge $a.Disk -and $b.Msg -ge $b.Disk -and $c.Msg -ge $c.Disk) "mensaje/disco: $($a.Msg)/$($a.Disk), $($b.Msg)/$($b.Disk), $($c.Msg)/$($c.Disk) ms"
    Show-Tree 'tras crear 3 entradas'
}
Step 'B5 Entrada: ver' {
    Open-Entry 'Trabajo' 'CANARIO-QA-Banco'
    $names = Get-Names
    Check 'ENTRADA ver: detalle con usuario y contrasena oculta' (($names -contains 'ana') -and ($names -contains '(oculto)') -and ($names -contains 'Copiar contraseña'))
    Check 'RF-43: la contrasena escrita no esta en el arbol UIA' (-not (($names -join ' ').Contains($Canary.Pass2)))
    Shot 'B03-entrada-detalle'
}
Step 'B6 Entrada: editar solo el titulo; la contrasena sigue igual (ejemplo 8)' {
    Click-Name -Name 'Editar'
    Check 'Formulario "Editar CANARIO-QA-Banco"' ((Wait-Text 'Editar CANARIO-QA-Banco' 4) -ge 0)
    $hint = (Get-Names | Where-Object { $_ -like 'Déjala vacía*' }) -join ''
    Log "   ayuda de la contrasena al editar: $hint"
    Shot 'B04-formulario-editar'
    Fill-Id 'entry-title' 'CANARIO-QA-Banco personal'
    Click-Name -Name 'Guardar entrada'
    Check 'ENTRADA editar: "Entrada guardada."' ((Wait-Text 'Entrada guardada.' 8) -ge 0)
    Check 'ENTRADA editar: el titulo nuevo esta en la lista' (Has-El -Name 'CANARIO-QA-Banco personal' -Type Button -Timeout 4)
    Start-Sleep -Milliseconds 1500
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    $clip = Get-Clip
    Check 'Ejemplo 8: tras editar solo el titulo, la contrasena es la misma (portapapeles)' ($clip -ceq $Canary.Pass2) "longitud $($clip.Length) (esperada $($Canary.Pass2.Length))"
    # Confirmar otra vez sin cambiar nada
    Click-Name -Name 'Editar'
    [void](Wait-Text 'Editar CANARIO-QA-Banco personal' 4)
    Start-Sleep -Milliseconds 1500
    $stamp = (Get-Item $Vault).LastWriteTimeUtc
    Click-Name -Name 'Guardar entrada'
    $ms = Wait-Text 'No había cambios que guardar.' 6
    Start-Sleep -Milliseconds 2500
    Check 'Ejemplo 8: confirmar sin cambios dice "No había cambios que guardar." y no reescribe el archivo' ($ms -ge 0 -and (Get-Item $Vault).LastWriteTimeUtc -eq $stamp) (Status-Text)
}
Summary
