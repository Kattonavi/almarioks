# Cierre: historial tras editar el titulo (ejemplo 8, visto con KeePassLib), minimo de la ventana arrastrando,
# y limpieza: borrar la carpeta de trabajo, quitar las recientes de prueba desde la propia app y dejarla cerrada.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'b-lib.ps1'), [Text.Encoding]::UTF8)))
$uno = Join-Path $Work 'qa-uno.kdbx'
Step 'Z1 Editar solo el titulo y mirar el historial con KeePass 2 (ejemplo 8)' -Continue {
    if ((Get-AppProc).Count -eq 0) { Start-App }
    [void](Find-El -Id 'welcome-title' -Timeout 20)
    Click-Name -Name 'Abrir qa-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Fill-Id 'unlock-password' $Master -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
    Open-Entry 'Personal QA' $Canary.Title
    Click-Name -Name 'Editar'
    [void](Wait-Text 'Editar *' 4)
    Fill-Id 'entry-title' "$($Canary.Title) 2"
    Click-Name -Name 'Guardar entrada'
    [void](Wait-Text 'Entrada guardada.' 8)
    Start-Sleep -Seconds 3
}
Step 'Z2 Minimo de la ventana arrastrando el borde invisible de la esquina' -Continue {
    [void](Focus-App)
    $before = Get-WinInfo
    $r = New-Object QA.RECT; [void][QA.Native]::GetWindowRect((Get-AppHwnd), [ref]$r)
    $x0 = $r.Right - 3; $y0 = $r.Bottom - 3
    [void][QA.Native]::SetCursorPos($x0, $y0); Start-Sleep -Milliseconds 300
    [QA.Native]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 200
    for ($k = 1; $k -le 30; $k++) {
        [void][QA.Native]::SetCursorPos([int]($x0 - ($x0 - $r.Left - 80) * $k / 30), [int]($y0 - ($y0 - $r.Top - 80) * $k / 30))
        [QA.Native]::mouse_event(0x0001, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 30
    }
    [QA.Native]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600
    $i = Get-WinInfo
    Log "   antes $($before.LogicalW)x$($before.LogicalH); tras arrastrar la esquina casi hasta el origen: $($i.LogicalW)x$($i.LogicalH)"
    if ($i.LogicalW -eq $before.LogicalW -and $i.LogicalH -eq $before.LogicalH) { Log '   el arrastre sintetico no engancho el borde: sin conclusion' }
    else { Check 'Arrastrando con el raton, la ventana no baja de 480x360' ($i.LogicalW -ge 478 -and $i.LogicalH -ge 358) "$($i.LogicalW)x$($i.LogicalH)" }
    Shot 'G11-tras-arrastrar-al-minimo'
    [void](Set-WinClient 800 600)
}
Step 'Z3 Cerrar y leer el historial con KeePassLib' -Continue {
    [void](Close-App 20)
    $exe = Join-Path $env:ProgramFiles 'KeePass Password Safe 2\KeePass.exe'
    [Reflection.Assembly]::LoadFrom($exe) | Out-Null
    $key = [KeePassLib.Keys.CompositeKey]::new(); $key.AddUserKey([KeePassLib.Keys.KcpPassword]::new($Master))
    $db = [KeePassLib.PwDatabase]::new()
    $db.Open([KeePassLib.Serialization.IOConnectionInfo]::FromPath($uno), $key, $null)
    $found = $null
    foreach ($e in $db.RootGroup.GetEntries($true)) { if ($e.Strings.ReadSafe('Title') -like 'CANARIO-QA-Titulo-Correo*') { $found = $e } }
    $title = $found.Strings.ReadSafe('Title'); $hist = $found.History.UCount
    $oldTitle = if ($hist -gt 0) { $found.History.GetAt(0).Strings.ReadSafe('Title') } else { '' }
    $samePw = $found.Strings.ReadSafe('Password') -ceq $Canary.Pass2
    $histPw = if ($hist -gt 0) { $found.History.GetAt(0).Strings.ReadSafe('Password') -ceq $Canary.Pass2 } else { $false }
    $db.Close()
    Log "   titulo = $title; historial = $hist; titulo en el historial = $oldTitle; contrasena igual = $samePw; contrasena del historial igual = $histPw"
    Check 'Ejemplo 8: titulo nuevo, contrasena intacta y el historial gana una version con el titulo viejo (visto con KeePass 2)' ($title -eq "$($Canary.Title) 2" -and $hist -eq 1 -and $oldTitle -eq $Canary.Title -and $samePw -and $histPw)
}
Step 'Z4 Borrar la carpeta de trabajo y quitar las recientes desde la app' {
    if ((Get-AppProc).Count -gt 0) { [void](Close-App 20) }
    if ($Work -notlike (Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa-*')) { throw 'La carpeta de trabajo no es la esperada: no se borra nada' }
    Remove-Item -LiteralPath $Work -Recurse -Force
    Check 'Carpeta de trabajo borrada' (-not (Test-Path $Work))
    Start-App
    [void](Find-El -Id 'welcome-title' -Timeout 20)
    Start-Sleep -Milliseconds 600
    Shot 'Z01-recientes-perdidas'
    foreach ($n in @('qa-uno.kdbx', 'qa-dos.kdbx')) {
        if (Has-El -Name "Quitar $n de la lista" -Type Button -Timeout 2) { Click-Name -Name "Quitar $n de la lista"; Start-Sleep -Milliseconds 500 }
        else { Log "   no aparece 'Quitar $n de la lista'" }
    }
    $empty = Has-El -Like 'Todavía no has abierto ninguna bóveda*' -Timeout 3
    Check 'RF-20: "Quitar … de la lista" deja la lista de recientes vacia' $empty
    Shot 'Z02-recientes-quitadas'
    Click-Name -Name 'Ajustes y Acerca de'
    [void](Find-El -Id 'settings-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    $lock = Get-Value (Find-El -Id 'settings-auto-lock'); $clip = Get-Value (Find-El -Id 'settings-clipboard')
    Check 'Ajustes como estaban antes de la prueba (5 minutos, 20 s)' ($lock -eq '5 minutos' -and $clip -eq '20') "$lock / $clip"
    Click-Name -Name 'Cerrar'
    Start-Sleep -Milliseconds 300
    Show-Net 'antes de cerrar'
    $closed = Close-App 20
    Check 'App cerrada' $closed
    Start-Sleep -Milliseconds 500
    Remove-Item -LiteralPath $Work -Recurse -Force
}
Log "Estado final: carpeta de trabajo existe = $(Test-Path $Work); procesos almarioks = $((Get-AppProc).Count)"
Log "Instalado: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe)"
$s = Get-Content -Raw -Encoding UTF8 $SettingsFile | ConvertFrom-Json
Log "settings.json: auto_lock_minutes=$($s.auto_lock_minutes) clipboard_seconds=$($s.clipboard_seconds) recientes=$(@($s.recent_vaults).Count)"
Log "Portapapeles: texto de $((Get-Clip).Length) caracteres"
Log "Datos de WebView2 siguen en %LOCALAPPDATA%\io.github.kattonavi.almarioks: $(Test-Path $DataDir)"
Summary
