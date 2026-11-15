# B (2/2, segunda corrida): la primera (b2) eligio mal el destino porque el <select> no expone sus opciones por UIA
# hasta desplegarse; borro en cascada la entrada principal. Aqui se rehace el escenario y se prueba de nuevo:
# ENTRADAS mover (eligiendo destino) · GRUPOS mover, rechazo a su propio subgrupo, borrar con aviso, vaciar papelera · releer.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
function Dialog-Text {
    (Get-All | Where-Object { (Type-Of $_) -eq 'Text' -and $_.Current.Name -match 'contiene|Se borrarán|para siempre|No se puede deshacer|No hay otro|papelera con' } |
        ForEach-Object { $_.Current.Name } | Select-Object -Unique) -join ' | '
}
Step 'B15 Rehacer el escenario' {
    Show-Tree 'inicio'
    $e = New-Entry 'qa-uno' $Canary.Title $Canary.Pass2 $Canary.User $Canary.Url $Canary.Notes $Canary.Tag
    Check 'ENTRADA crear (todos los campos, contrasena con ñ y €)' ($e.Msg -ge 0)
    Check 'GRUPO crear: "Trabajo"' (New-Group 'qa-uno' 'Trabajo')
    Check 'GRUPO crear: "Clientes" dentro de Trabajo' (New-Group 'Trabajo' 'Clientes')
    [void](New-Entry 'Trabajo' 'CANARIO-QA-Banco' 'x-QA-banco-1' 'ana')
    [void](New-Entry 'Clientes' 'CANARIO-QA-Cliente' 'x-QA-cliente-1')
    Show-Tree 'escenario'
}
Step 'B16 Entrada: mover a otro grupo eligiendo el destino' -Continue {
    Open-Entry 'qa-uno' $Canary.Title
    Click-Name -Name 'Mover a otro grupo…'
    Start-Sleep -Milliseconds 400
    $picked = Select-Option 'move-entry-target' '*Personal QA*'
    Shot 'B05-dialogo-mover-entrada'
    Check 'El destino "Personal QA" se puede elegir' $picked
    Click-Name -Name 'Mover'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Show-Tree 'tras mover la entrada'
    Check 'ENTRADA mover: queda solo en "Personal QA" (RF-46)' ((Has-El -Name 'Personal QA · 1 entrada' -Type Button -Timeout 3) -and (Has-El -Name 'qa-uno · 0 entradas' -Type Button) -and $st -like '*se movió a Personal QA*') $st
}
Step 'B17 Grupo: mover a su propio subgrupo (RF-55, ejemplo 3)' -Continue {
    Select-Group 'Trabajo'
    Click-Name -Name 'Mover grupo…'
    Start-Sleep -Milliseconds 400
    $opts = List-Options 'move-group-target'
    Log "   destinos que ofrece el dialogo 'Mover «Trabajo»': $($opts -join ' | ')"
    Shot 'B06-dialogo-mover-grupo'
    $offersChild = [bool]($opts | Where-Object { $_ -like '*Clientes*' })
    $offersSelf = [bool]($opts | Where-Object { $_ -match '(^|› )Trabajo$' })
    if ($offersChild) {
        [void](Select-Option 'move-group-target' '*Clientes*')
        Click-Name -Name 'Mover'
        Start-Sleep -Milliseconds 900
        $msg = Alert-Text
        Log "   mensaje al mover Trabajo dentro de Clientes: $msg"
        Shot 'B07-mover-grupo-rechazado'
        Check 'GRUPO mover a su subgrupo: se rechaza con un mensaje entendible' ([bool]$msg) $msg
    } else {
        Check 'GRUPO mover a su subgrupo: la ventana no lo permite (ni el grupo ni su subgrupo salen como destino)' (-not $offersSelf) "destinos: $($opts -join ' | ')"
        Log '   OJO: no hay mensaje que explique por que faltan esos destinos; el rechazo del nucleo (MovimientoInvalido) no se puede provocar desde la ventana'
    }
    if (Has-El -Name 'Cancelar' -Type Button) { Click-Name -Name 'Cancelar' }
    Start-Sleep -Milliseconds 300
}
Step 'B18 Grupo: mover (valido)' -Continue {
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
Step 'B19 Grupo: borrar con el aviso de contenido (RF-56, ejemplo 4)' -Continue {
    Select-Group 'Trabajo'
    Click-Name -Name 'Borrar grupo…'
    Start-Sleep -Milliseconds 500
    $text = Dialog-Text
    Log "   aviso: $text"
    Log "   foco inicial del dialogo: $(Focused-Desc)"
    Shot 'B09-dialogo-borrar-grupo'
    Check 'GRUPO borrar: el aviso dice "2 entradas y 1 subgrupo"' ($text -like '*2 entradas y 1 subgrupo*') $text
    Click-Name -Name 'Mover a la papelera'
    Start-Sleep -Milliseconds 900
    Show-Tree 'tras borrar Trabajo'
    Check 'GRUPO borrar: va a la papelera con su contenido' ((Has-El -Like 'Papelera*' -Type Button -Timeout 3) -and (Status-Text) -like '*«Trabajo» se movió a la papelera*') (Status-Text)
}
Step 'B20 Vaciar la papelera (RF-58)' -Continue {
    Select-Group 'Papelera'
    Click-Name -Name 'Vaciar papelera…'
    Start-Sleep -Milliseconds 500
    $text = Dialog-Text
    Log "   aviso: $text"
    Shot 'B12-dialogo-vaciar-papelera'
    Click-Name -Name 'Vaciar papelera'
    Start-Sleep -Milliseconds 900
    $st = Status-Text
    Check 'PAPELERA vaciar: el aviso dice "2 entradas y 2 subgrupos" y queda vacia' ($text -like '*2 entradas y 2 subgrupos*' -and $st -like '*quedó vacía*') "$text => $st"
    Show-Tree 'tras vaciar'
}
Step 'B21 Releer: bloquear, desbloquear y comparar' {
    Relock-Unlock
    $tree = (Tree-Lines) -join '  |  '
    Log "   ARBOL releido del disco: $tree"
    Dump-Tree 'B14-releido.uia.txt' | Out-Null
    Check 'Releido: raiz 0, Archivo 0, Personal QA 1, papelera 0, sin Trabajo ni Clientes' ($tree -like '*qa-uno · 0 entradas*' -and $tree -like '*Archivo · 0 entradas*' -and $tree -like '*Personal QA · 1 entrada*' -and $tree -notlike '*Trabajo*' -and $tree -notlike '*Clientes*') $tree
    Fill (Find-El -Id 'vault-search') 'canario'
    Start-Sleep -Milliseconds 800
    $found = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like 'CANARIO-QA-*' } | ForEach-Object { $_.Current.Name })
    Log "   busqueda 'canario': $($found -join ' | ')"
    Check 'Releido: la busqueda solo encuentra la entrada que quedo' ($found.Count -eq 1 -and $found[0] -eq $Canary.Title) ($found -join ', ')
    Shot 'B14-releido-busqueda'
    Click-Name -Name $Canary.Title
    Start-Sleep -Milliseconds 500
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 400
    Check 'Releido: la contrasena con ñ y € sale igual del disco (RF-41)' ((Get-Clip) -ceq $Canary.Pass2)
    Click-El (Find-El -Id 'vault-search'); Press 'ESC'
    Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length)" }
}
Summary
