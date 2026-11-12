# B (2/2): ENTRADAS mover, borrar (papelera), borrar definitivo · GRUPOS mover (y el rechazo a su propio subgrupo),
# borrar con el aviso "N entradas y M subgrupos", vaciar la papelera · y releer todo tras bloquear y desbloquear.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
function Dialog-Text { (Get-Names | Where-Object { $_ -match 'contiene|Se borrarán|para siempre|No se puede deshacer|No hay otro|papelera con' }) -join ' | ' }

Step 'B7 Entrada: mover a otro grupo' -Continue {
    Open-Entry 'qa-uno' $Canary.Title
    Click-Name -Name 'Mover a otro grupo…'
    Start-Sleep -Milliseconds 400
    Shot 'B05-dialogo-mover-entrada'
    $picked = Select-Option 'move-entry-target' '*Personal QA*'
    Check 'El destino "Personal QA" se puede elegir con el raton' $picked
    Click-Name -Name 'Mover'
    Start-Sleep -Milliseconds 900
    Log "   estado: $(Status-Text)"
    Show-Tree 'tras mover la entrada'
    Check 'ENTRADA mover: queda en "Personal QA" y sale de la raiz (RF-46)' ((Has-El -Name 'Personal QA · 1 entrada' -Type Button -Timeout 3) -and (Has-El -Name 'qa-uno · 1 entrada' -Type Button)) (Status-Text)
}
Step 'B8 Grupo: mover a su propio subgrupo se rechaza (RF-55, ejemplo 3)' -Continue {
    Select-Group 'Trabajo'
    Click-Name -Name 'Mover grupo…'
    Start-Sleep -Milliseconds 400
    Shot 'B06-dialogo-mover-grupo'
    $offered = Select-Option 'move-group-target' '*Clientes*'
    if ($offered) {
        Click-Name -Name 'Mover'
        Start-Sleep -Milliseconds 900
        $msg = Alerts
        Log "   mensaje al mover Trabajo dentro de Clientes: $msg"
        Shot 'B07-mover-grupo-rechazado'
        Check 'GRUPO mover a su subgrupo: se rechaza con un mensaje entendible' ([bool]$msg -and (Has-El -Like 'Trabajo · *' -Type Button)) $msg
        if (Has-El -Name 'Cancelar' -Type Button) { Click-Name -Name 'Cancelar' }
    } else {
        Log "   el destino 'Clientes' no se ofrece en la lista; textos del dialogo: $(Alerts)"
        Shot 'B07-mover-grupo-sin-destino-hijo'
        Check 'GRUPO mover a su subgrupo: imposible desde la ventana (el subgrupo no se ofrece como destino)' $true 'sin mensaje: la opcion no existe'
        Click-Name -Name 'Cancelar'
    }
    Start-Sleep -Milliseconds 300
    Show-Tree 'tras el intento'
}
Step 'B9 Grupo: mover (valido)' -Continue {
    Check 'GRUPO crear: "Archivo" en la raiz' (New-Group 'qa-uno' 'Archivo')
    Select-Group 'Personal QA'
    Click-Name -Name 'Mover grupo…'
    Start-Sleep -Milliseconds 400
    $picked = Select-Option 'move-group-target' '*Archivo*'
    Click-Name -Name 'Mover'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Check 'GRUPO mover: "Personal QA" dentro de "Archivo" (RF-54)' ($picked -and $st -like '*se movió a*Archivo*') $st
    Show-Tree 'tras mover el grupo'
    Dump-Tree 'B08-arbol-tras-mover.uia.txt' | Out-Null
    Shot 'B08-grupo-movido'
}
Step 'B10 Grupo: borrar con el aviso de contenido (RF-56, ejemplo 4)' -Continue {
    Select-Group 'Trabajo'
    Click-Name -Name 'Borrar grupo…'
    Start-Sleep -Milliseconds 500
    $text = Dialog-Text
    Log "   aviso: $text"
    Shot 'B09-dialogo-borrar-grupo'
    Check 'GRUPO borrar: el aviso dice "2 entradas y 1 subgrupo"' ($text -like '*2 entradas y 1 subgrupo*') $text
    Log "   foco inicial del dialogo: $(Focused-Desc)"
    Click-Name -Name 'Mover a la papelera'
    Start-Sleep -Milliseconds 900
    Show-Tree 'tras borrar Trabajo'
    Check 'GRUPO borrar: va a la papelera con su contenido' (Has-El -Like 'Papelera*' -Type Button -Timeout 3) (Status-Text)
}
Step 'B11 Entrada: borrar (a la papelera)' -Continue {
    Open-Entry 'qa-uno' 'CANARIO-QA-Desechable'
    Click-Name -Name 'Borrar entrada'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Log "   estado: $st"
    Check 'ENTRADA borrar: se mueve a la papelera y lo dice (RF-47)' ($st -like '*se movió a la papelera*' -and (Has-El -Name 'qa-uno · 0 entradas' -Type Button -Timeout 3)) $st
    Shot 'B10-entrada-a-la-papelera'
    Show-Tree 'tras borrar la entrada'
}
Step 'B12 Entrada: borrar definitivamente desde la papelera (RF-48)' -Continue {
    Select-Group 'Papelera'
    Click-Name -Name 'CANARIO-QA-Desechable'
    Start-Sleep -Milliseconds 400
    Click-Name -Name 'Borrar definitivamente…'
    Start-Sleep -Milliseconds 500
    Log "   aviso: $(Dialog-Text)"
    Log "   foco inicial del dialogo: $(Focused-Desc)"
    Shot 'B11-dialogo-borrar-definitivo'
    Click-Name -Name 'Borrar definitivamente'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Check 'ENTRADA borrar definitivo: desaparece de la papelera y lo dice' ($st -like '*Se borró*de forma definitiva*' -and -not (Has-El -Name 'CANARIO-QA-Desechable' -Type Button)) $st
    Show-Tree 'tras borrar definitivo'
}
Step 'B13 Vaciar la papelera (RF-58)' -Continue {
    Select-Group 'Papelera'
    Click-Name -Name 'Vaciar papelera…'
    Start-Sleep -Milliseconds 500
    $text = Dialog-Text
    Log "   aviso: $text"
    Shot 'B12-dialogo-vaciar-papelera'
    Click-Name -Name 'Vaciar papelera'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Check 'PAPELERA vaciar: el aviso dice cuanto se borra y la papelera queda vacia' ($text -like '*2 entradas y 2 subgrupos*' -and $st -like '*quedó vacía*') "$text => $st"
    Show-Tree 'tras vaciar'
    Shot 'B13-papelera-vacia'
}
Step 'B14 Releer: bloquear, desbloquear y comparar' {
    Start-Sleep -Milliseconds 2500   # deja terminar el ultimo guardado
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10)
    Start-Sleep -Milliseconds 1200
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Start-Sleep -Milliseconds 600
    $tree = (Tree-Lines) -join '  |  '
    Log "   ARBOL releido del disco: $tree"
    Dump-Tree 'B14-releido.uia.txt' | Out-Null
    Check 'Releido: raiz 0, Archivo 0, Personal QA 1, sin Trabajo ni Clientes' ($tree -like '*qa-uno · 0 entradas*' -and $tree -like '*Archivo · 0 entradas*' -and $tree -like '*Personal QA · 1 entrada*' -and $tree -notlike '*Trabajo*' -and $tree -notlike '*Clientes*') $tree
    Fill (Find-El -Id 'vault-search') 'canario'
    Start-Sleep -Milliseconds 800
    $found = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like 'CANARIO-QA-*' } | ForEach-Object { $_.Current.Name })
    Log "   busqueda 'canario': $($found -join ' | ')"
    Check 'Releido: la busqueda solo encuentra la entrada que quedo' ($found.Count -eq 1 -and $found[0] -eq $Canary.Title) ($found -join ', ')
    Shot 'B14-releido-busqueda'
    Click-El (Find-El -Id 'vault-search'); Press 'ESC'
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length)" }
}
Summary
