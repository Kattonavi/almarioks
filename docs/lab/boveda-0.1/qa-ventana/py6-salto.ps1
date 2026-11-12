# Ronda 2 (6): ¿por que la primera entrada de muestra se guardo sin sitio? En "Nueva entrada" se teclea la contrasena
# y, SIN esperar, se hace clic en el campo Sitio y se teclea. Se mira cuanto baja el campo al aparecer el medidor de
# fortaleza, donde cae el foco y si el texto llega al campo. No guarda nada: cierra con Escape y "Descartar".
# Tambien sirve de comprobacion de que la boveda de muestra abre desde "recientes" con su contrasena maestra.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'py-pasos.ps1'), [Text.Encoding]::UTF8)))
Step 'PY60 Boveda de muestra abierta desde recientes' { Ensure-Cuentas; [void](Size-Window 1120 720); Clear-Search; Pick-Group 'Compras' }
foreach ($wait in @(0, 0, 0, 600)) {
    Step "PY61 Contrasena y, tras $wait ms, clic en Sitio" -Continue {
        Click-Name -Name 'Nueva entrada'
        [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
        Type-Text 'Prueba de salto'
        Fill-Id 'entry-dialog-username' 'ana.casa@example.com'
        $y0 = (LRect (Find-El -Id 'entry-dialog-url')).Y
        Fill-Id 'entry-dialog-password' 'FAROPYsalto123456' -Secret
        if ($wait) { Start-Sleep -Milliseconds $wait }
        $url = Find-El -Id 'entry-dialog-url'; $yClick = (LRect $url).Y
        Click-El $url; Press-Ctrl 'a'; Press 'DEL'; Type-Text 'https://salto.example'
        Start-Sleep -Milliseconds 500
        $y1 = (LRect (Find-El -Id 'entry-dialog-url')).Y
        $val = "$(Get-Value (Find-El -Id 'entry-dialog-url'))"
        $open = Has-El -Name 'Ocultar opciones del generador' -Type Button -Timeout 0.3
        Check "Con $wait ms de espera el sitio tecleado llega a su campo" ($val -ceq 'https://salto.example') "campo Sitio: y=$y0 antes de la contrasena, y=$yClick al leerlo para el clic, y=$y1 medio segundo despues (baja $([math]::Round($y1 - $y0, 1)) px; alto del campo 44,8); valor=`"$val`"; foco: $(Focused-Desc); opciones del generador abiertas por el clic = $open"
        Press 'ESC'; Start-Sleep -Milliseconds 500
        $ask = Find-El -Name 'Descartar' -Type Button -Timeout 1 -Quiet
        if ($null -ne $ask) { Click-El $ask; Start-Sleep -Milliseconds 500 }
        for ($k = 0; $k -lt 3 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) { $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.3 -Quiet; if ($null -ne $ask) { Click-El $ask } else { Press 'ESC' }; Start-Sleep -Milliseconds 500 }
    }
}
Step 'PY62 Nada guardado' -Continue {
    Check 'No quedo ninguna entrada de prueba' (-not (Has-Entry 'Prueba de salto')) "Compras: $((Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like 'Compras · *' } | ForEach-Object { $_.Current.Name }) -join '')"
    Pick-Group 'Trabajo'; Pick-Entry 'GitHub'
}
Summary
