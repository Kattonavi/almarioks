# Decisiones del dueno (4b), RF-116: con 4 lineas el cuerpo de la entrada de 500 ya no desborda a 1120 x 720, asi que
# para desplazar el contenido de la ventana se alargan sus notas (40 renglones, pegados en "Editar") y se mide la
# cabecera antes y despues de la rueda sobre el cuerpo, con la contrasena oculta y mostrada. RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
# Las ayudas de pd4-cabecera.ps1 (todo lo que hay antes de su primer paso, sin volver a cargar rf-lib.ps1).
$pd4 = ([IO.File]::ReadAllText((Join-Path $QaDir 'pd4-cabecera.ps1'), [Text.Encoding]::UTF8) -split "`nStep 'PD40")[0]
$pd4 = $pd4 -replace '(?m)^\. .*rf-lib\.ps1.*$', ''
. ([scriptblock]::Create($pd4))
Step 'PD43 Notas largas en la entrada de 500' {
    Ensure-Pd; [void](Size-Window 1120 720); Clear-Search
    Pick-Entry $PdLong; Click-Name -Name "Abrir $PdLong"; [void](Entry-Window $PdLong 4); Start-Sleep -Milliseconds 600
    Click-El (Find-El -Name 'Editar' -Type Button -Root (Entry-Window $PdLong 2)); Start-Sleep -Milliseconds 800
    $notes = (1..40 | ForEach-Object { "Renglon $_ de las notas de prueba" }) -join "`r`n"
    $n = Find-El -Id 'entry-dialog-notes' -Timeout 4; Click-El $n; Start-Sleep -Milliseconds 200
    Set-ClipExternal $notes; Start-Sleep -Milliseconds 200
    Press-Ctrl 'a'; Press-Ctrl 'v'; Start-Sleep -Milliseconds 500
    [System.Windows.Forms.Clipboard]::Clear()
    $save = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -like 'Guardar*' }) | Select-Object -First 1
    Log "   boton de guardar: `"$($save.Current.Name)`""
    $stamp0 = (Get-Item -LiteralPath $Vault).LastWriteTimeUtc
    Click-El $save
    $sw = [Diagnostics.Stopwatch]::StartNew(); while ($sw.Elapsed.TotalSeconds -lt 20 -and (Get-Item -LiteralPath $Vault).LastWriteTimeUtc -eq $stamp0) { Start-Sleep -Milliseconds 100 }
    Start-Sleep -Milliseconds 900
    Check 'Notas de 40 renglones guardadas' ((Get-Item -LiteralPath $Vault).LastWriteTimeUtc -ne $stamp0) "ventana abierta ahora: $((@(Get-All | Where-Object { (Type-Of $_) -eq 'Window' }) | ForEach-Object { $_.Current.Name }) -join ' | ')"
}
Step 'PD44 Rueda sobre el cuerpo: la cabecera no se mueve' -Continue {
    if ($null -eq (Entry-Window $PdLong 1)) { Ensure-Pd; Clear-Search; Pick-Entry $PdLong; Click-Name -Name "Abrir $PdLong"; [void](Entry-Window $PdLong 4); Start-Sleep -Milliseconds 600 }
    foreach ($shown in @($false, $true)) {
        if ($shown) { Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Entry-Window $PdLong 2)); Start-Sleep -Milliseconds 700 }
        $h0 = Head-Y $PdLong; $b0 = Body-Ys $PdLong
        $w = (Entry-Window $PdLong 2).Current.BoundingRectangle
        $px = [int]($w.X + 60); $py = [int]($w.Y + $w.Height - 60)
        Wheel-At $px $py -6
        $h1 = Head-Y $PdLong; $b1 = Body-Ys $PdLong; $mv = Moved $b0 $b1
        $u0 = $b0['Usuario']; $u1 = $b1['Usuario']
        Wheel-At $px $py 10
        $h2 = Head-Y $PdLong
        Check "(3) Contrasena $(if ($shown) { 'mostrada' } else { 'oculta' }): la rueda desplaza el cuerpo y la cabecera queda quieta (+-1 px)" ($mv -gt 0 -and (Same-Head $h0 $h1) -and (Same-Head $h0 $h2)) "elementos del cuerpo que cambiaron de Y: $mv de $($b0.Count) (rotulo Usuario y=$u0 -> $u1); cabecera antes: $($h0.Txt); tras bajar: $($h1.Txt); tras volver: $($h2.Txt)"
    }
    Press 'ESC'; Start-Sleep -Milliseconds 500
}
Summary
