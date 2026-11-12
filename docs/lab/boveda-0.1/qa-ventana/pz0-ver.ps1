# Ronda 2, puntos 1 a 4 (0): descubrir por UIA los nombres de lo que no se conoce. Argumento: generador | crear (incluye el
# desbloqueo y la bienvenida) | ajustes. No crea ni guarda nada. RF_VARIANT=pz-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$what = if ($StepArgs.Count) { $StepArgs[0] } else { 'generador' }
if ($what -eq 'generador') {
    Step 'PZ00 Dialogo Nueva entrada y opciones del generador' {
        Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search; Pick-Group 'Trabajo'
        Dump-Here 'boveda abierta'
        Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
        $b = Find-El -Like '*pciones del generador*' -Type Button -Timeout 3
        Click-El $b; Start-Sleep -Milliseconds 500
        Dump-Here 'nueva entrada con opciones'
        Press 'ESC'; Start-Sleep -Milliseconds 500
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.5 -Quiet; if ($null -ne $ask) { Click-El $ask }
    }
}
if ($what -eq 'crear') {
    Step 'PZ01 Crear boveda' {
        Ensure-Cuentas; [void](Size-Window 1120 720)
        Click-Name -Name 'Más acciones de Cuentas'; Start-Sleep -Milliseconds 500
        Log "   MENU de la boveda: $((@(Get-All | Where-Object { (Type-Of $_) -eq 'MenuItem' } | ForEach-Object { $_.Current.Name })) -join ' | ')"
        Press 'ESC'; Start-Sleep -Milliseconds 400
        Click-Name -Name 'Bloquear'; [void](Find-El -Id 'unlock-password' -Timeout 8); Start-Sleep -Milliseconds 1200
        Dump-Here 'desbloqueo de Cuentas (sin llave)'
        Log "   a la bienvenida = $(To-Welcome)"
        Dump-Here 'bienvenida'
        Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
        Fill-Id 'create-name' 'Familia'; Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 900
        Log "   RADIOS: $((Radios) -join ' | ')"
        Click-El (Find-El -Name 'Crear uno nuevo' -Type RadioButton); Start-Sleep -Milliseconds 700
        Dump-Here 'crear con llave nueva'
        Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 500
    }
}
if ($what -eq 'ajustes') {
    Step 'PZ02 Ajustes y Acerca de' {
        Ensure-Cuentas
        Click-Name -Name 'Ajustes y Acerca de'; [void](Find-El -Id 'settings-title' -Timeout 5); Start-Sleep -Milliseconds 500
        Dump-Here 'ajustes'
        Click-Name -Name 'Cerrar'; Start-Sleep -Milliseconds 300
    }
}
Summary
