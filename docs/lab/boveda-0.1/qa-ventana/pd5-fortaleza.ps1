# Decisiones del dueno (5), RF-115: en "Nueva entrada" la linea de fortaleza ocupa siempre su alto. Se mide por UIA la Y
# del campo "Sitio" con la contrasena vacia, justo tras teclearla, con "Fortaleza: …" a la vista y tras vaciarla (control:
# abrir "Opciones del generador" si baja "Sitio"), y se repite py6-salto.ps1 (clic inmediato en "Sitio" tras teclear la
# contrasena), que antes fallaba 3 de 3. No guarda nada. RF_VARIANT=pd-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
function Url-Y { (LRect (Find-El -Id 'entry-dialog-url')).Y }
function Strength { $n = @(Names-Like '^Fortaleza'); if ($n.Count) { $n[0] } else { '(sin texto)' } }
function Discard-New {
    for ($k = 0; $k -lt 3 -and $null -ne (Find-El -Type Window -Timeout 0.3 -Quiet); $k++) { $ask = Find-El -Name 'Descartar' -Type Button -Timeout 0.4 -Quiet; if ($null -ne $ask) { Click-El $ask } else { Press 'ESC' }; Start-Sleep -Milliseconds 500 }
}
Step 'PD50 Sitio en la misma Y en los cuatro momentos' -Continue {
    Ensure-Pd; [void](Size-Window 1120 720); Clear-Search
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 500
    Pointer-Away; Shot-Client 'app-nueva-entrada-vacia-1120x720'
    Type-Text 'FAROPD-fortaleza'
    $y0 = Url-Y; $s0 = Strength
    Fill-Id 'entry-dialog-password' 'FAROPDmedidor5520' -Secret
    $y1 = Url-Y; $s1 = Strength
    $sw = [Diagnostics.Stopwatch]::StartNew(); while ($sw.Elapsed.TotalSeconds -lt 4 -and (Strength) -eq '(sin texto)') { Start-Sleep -Milliseconds 50 }
    Start-Sleep -Milliseconds 300
    $y2 = Url-Y; $s2 = Strength
    Click-El (Find-El -Id 'entry-dialog-password'); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 800
    $y3 = Url-Y; $s3 = Strength
    $all = @($y0, $y1, $y2, $y3); $spread = ($all | Measure-Object -Maximum).Maximum - ($all | Measure-Object -Minimum).Minimum
    Check '(2) "Sitio" en la misma Y (+-1 px): vacia, tras teclear, con "Fortaleza" y tras vaciar' ($spread -le 1 -and $s2 -ne '(sin texto)' -and $s0 -eq '(sin texto)' -and $s3 -eq '(sin texto)') "vacia y=$y0 [$s0]; justo tras teclear y=$y1 [$s1]; con el texto y=$y2 [$s2]; vaciada y=$y3 [$s3]; diferencia maxima $spread px"
    Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 600
    $y4 = Url-Y
    Check '(control) Abrir "Opciones del generador" si baja "Sitio" (la medida ve el movimiento)' ($y4 -gt $y3 + 5) "y=$y3 -> $y4"
    Discard-New
}
foreach ($wait in @(0, 0, 0, 600)) {
    Step "PD51 Contrasena y, tras $wait ms, clic en Sitio (py6-salto)" -Continue {
        Click-Name -Name 'Nueva entrada'
        [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
        Type-Text 'Prueba de salto'
        Fill-Id 'entry-dialog-username' 'ana.pd'
        $y0 = Url-Y
        Fill-Id 'entry-dialog-password' 'FAROPDsalto123456' -Secret
        if ($wait) { Start-Sleep -Milliseconds $wait }
        $url = Find-El -Id 'entry-dialog-url'; $yClick = (LRect $url).Y
        Click-El $url; Press-Ctrl 'a'; Press 'DEL'; Type-Text 'https://salto.example'
        Start-Sleep -Milliseconds 500
        $y1 = Url-Y
        $val = "$(Get-Value (Find-El -Id 'entry-dialog-url'))"
        Check "(2) Con $wait ms de espera el sitio tecleado llega a su campo y no salta" ($val -ceq 'https://salto.example' -and [math]::Abs($y1 - $y0) -le 1) "Sitio y=$y0 antes, y=$yClick al clic, y=$y1 despues; valor=`"$val`"; fortaleza: $(Strength); foco: $(Focused-Desc)"
        Discard-New
    }
}
Step 'PD52 Nada guardado' -Continue { Check 'No quedo ninguna entrada de prueba' (-not (Has-Entry 'Prueba de salto') -and -not (Has-Entry 'FAROPD-fortaleza')) }
Summary
