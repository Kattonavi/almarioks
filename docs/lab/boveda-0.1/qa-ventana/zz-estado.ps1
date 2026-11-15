# Estado actual: disco y textos de estado de la ventana.
Get-ChildItem $Work | ForEach-Object { Log "  disco: $($_.Name) $($_.Length) $($_.LastWriteTime.ToString('HH:mm:ss'))" }
$names = Get-Names
Log ("estado: " + (($names | Where-Object { $_ -match 'Guard|guard|Copiado|cumpli|bloquea|^\d+:\d\d$' }) -join ' | '))
Get-ChildItem $Work -Filter 'app-*.log' | Where-Object Length -gt 0 | ForEach-Object { Log "--- $($_.Name)"; Get-Content $_.FullName -TotalCount 40 | ForEach-Object { Log "   $_" } }
