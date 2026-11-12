# Arranque medido y captura de la bienvenida vacia.
if ((Get-AppProc).Count -eq 0) { Start-App }
$ms = Wait-Text 'Bienvenida' 20
Log "Bienvenida legible por UIA a los $ms ms tras tener ventana (ventana a los $LastStartMs ms)"
[QA.Native]::SetCursorPos(5, 5) | Out-Null
Shot 'A01-bienvenida-primer-arranque'
$i = Get-WinInfo
Log "Ventana: exterior $($i.OuterW)x$($i.OuterH), interior logico $($i.LogicalW)x$($i.LogicalH), escala $($i.Scale), primer plano = $([QA.Native]::GetForegroundWindow() -eq $i.Hwnd)"
Get-ChildItem $Work | ForEach-Object { Log "  $($_.Name) $($_.Length)" }
