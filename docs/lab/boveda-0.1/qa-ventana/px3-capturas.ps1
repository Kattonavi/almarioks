# Pase de interaccion (3): capturas de la boveda de muestra y del dialogo "Nueva entrada" en la app instalada, a
# 1120 x 720 y a 1280 x 800, mas los extras (menus, aviso de copiado). Van a la carpeta de estado; a
# .impeccable\review\pase\app se copian despues de mirarlas. Antes bloquea y desbloquea: asi el riel no arrastra el
# aviso de la ultima accion de grupo.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
function Close-Dialogs { for ($k = 0; $k -lt 3 -and -not (Has-El -Name 'Nueva entrada' -Type Button -Timeout 0.3); $k++) { Press 'ESC'; Start-Sleep -Milliseconds 350 } }
Step 'PX20 Bloquear y desbloquear (riel limpio)' {
    Ensure-Cuentas
    Close-Dialogs
    Start-Sleep -Milliseconds 2500
    Click-Name -Name 'Bloquear'
    [void](Find-El -Name 'Desbloquear' -Type Button -Timeout 10); Start-Sleep -Milliseconds 1200
    Log "   pantalla de desbloqueo: $((Get-Names | Where-Object { $_ -notlike '*\*' } | Select-Object -Unique -First 12) -join ' | ')"
    Ensure-Cuentas
}
foreach ($size in @(@(1120, 720), @(1280, 800))) {
    $tag = "$($size[0])x$($size[1])"
    Step "PX21 Boveda a $tag" -Continue {
        Ensure-Cuentas; Close-Dialogs
        Check "Ventana a $tag logicos y dentro del area util" (Size-Window $size[0] $size[1])
        Pick-Group 'Trabajo'; Pick-Entry 'GitHub'; Start-Sleep -Milliseconds 700
        $tree = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name })
        Check 'Conteos del riel: 13 en total; Personal 2, Trabajo 8, Clientes 1, Bancos 1, Compras 1' (($tree -join '|') -eq 'Todas las entradas · 13 entradas|Personal · 2 entradas|Trabajo · 8 entradas|Clientes · 1 entrada|Bancos · 1 entrada|Compras · 1 entrada') ($tree -join ' | ')
        Shot-Client "app-boveda-$tag"
        [void](Dump-Rects -All -file "rects-boveda-$tag.txt")
    }
    Step "PX22 Dialogo Nueva entrada a $tag" -Continue {
        Click-Name -Name 'Nueva entrada'
        [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 700
        Check 'El foco esta en Titulo y el campo esta vacio' ((Focused-Desc) -like '*entry-dialog-title*' -and -not (Get-Value (Find-El -Id 'entry-dialog-title'))) (Focused-Desc)
        Shot-Client "app-nueva-entrada-$tag"
        [void](Dump-Rects -All -file "rects-dialogo-$tag.txt")
        Press 'ESC'; Start-Sleep -Milliseconds 500
        Check 'Esc sin cambios cierra el dialogo sin preguntar' (-not (Dialog-Open 'Nueva entrada') -and (Has-El -Name 'Bloquear' -Type Button -Timeout 1)) "foco tras cerrar: $(Focused-Desc)"
    }
    if ($size[0] -ne 1120) { continue }
    Step 'PX23 Extras: menu del grupo, menu del ultimo grupo, menu de la entrada, aviso de copiado' -Continue {
        foreach ($g in @('Trabajo', 'Compras')) {
            $btn = Find-El -Name "Más acciones de $g" -Type Button
            Click-El $btn; Start-Sleep -Milliseconds 500
            $menu = Find-El -Name "Acciones de $g" -Type Menu -Timeout 3
            $items = @($menu.FindAll($TreeScope::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, [System.Windows.Automation.ControlType]::MenuItem))) | ForEach-Object { $_.Current.Name })
            Log "   MENU de $g : boton $(LRect-Text $btn); menu $(LRect-Text $menu); elementos: $($items -join ' | '); foco: $(Focused-Desc)"
            $b = LRect $btn; $m = LRect $menu
            Check "El menu de $g abre pegado a su boton y dentro de la ventana" (([math]::Abs($m.Y - $b.B) -le 8 -or [math]::Abs($m.B - $b.Y) -le 8) -and $m.X -ge 0 -and $m.B -le 720 -and [math]::Abs($m.R - $b.R) -le 12) "boton abajo=$($b.B) derecha=$($b.R); menu arriba=$($m.Y) abajo=$($m.B) derecha=$($m.R)"
            Shot-Client "extra-menu-grupo-$g-$tag" -KeepPointer
            Press 'ESC'; Start-Sleep -Milliseconds 400
            Log "   foco tras Escape: $(Focused-Desc)"
        }
        Pick-Group 'Trabajo'; Pick-Entry 'GitHub'
        $btn = Find-El -Name 'Más acciones de la entrada' -Type Button
        Click-El $btn; Start-Sleep -Milliseconds 500
        $menu = @(Get-All | Where-Object { (Type-Of $_) -eq 'Menu' }) | Select-Object -First 1
        $items = @(Get-All | Where-Object { (Type-Of $_) -eq 'MenuItem' } | ForEach-Object { $_.Current.Name })
        Log "   MENU de la entrada: boton $(LRect-Text $btn); menu `"$($menu.Current.Name)`" $(LRect-Text $menu); elementos: $($items -join ' | ')"
        Shot-Client "extra-menu-entrada-$tag" -KeepPointer
        Press 'ESC'; Start-Sleep -Milliseconds 400
        Set-ClipExternal 'centinela-px-captura'; Start-Sleep -Milliseconds 200
        Click-Name -Name 'Copiar usuario'; Start-Sleep -Milliseconds 500
        Shot-Client "extra-copiado-$tag"
        Log "   aviso tras copiar el usuario: $((Get-Names | Where-Object { $_ -like '*opiad*' -or $_ -like '*portapapeles*' -or $_ -like '*borra*' }) -join ' | ')"
    }
}
Step 'PX24 Volver a 1120 x 720' -Continue { Close-Dialogs; Check 'Ventana a 1120 x 720' (Size-Window 1120 720) }
Summary
