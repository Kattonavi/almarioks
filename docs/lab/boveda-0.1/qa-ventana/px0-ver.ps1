# Pase de interaccion (0): explorador. Ejecuta una lista de ordenes sobre la ventana abierta, para descubrir los
# nombres accesibles de la interfaz nueva sin suponerlos. Las ordenes llegan en la variable PX_CMDS, separadas por ";;":
#   start | close | size=1120x720 | names | rects | all | shot=nombre | click=Nombre | clicklike=Comodin | clickid=id
#   select=id:comodin | wheel=n | filedialog=archivo (en la carpeta de trabajo) | famkey | hover=Nombre
#   clicktype=Tipo:Nombre | fill=id:texto | type=texto | key=ESC | ctrl=k | sleep=ms | focus | wait=Comodin | rclick=Nombre
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
foreach ($cmd in ($env:PX_CMDS -split ';;')) {
    $cmd = $cmd.Trim(); if (-not $cmd) { continue }
    $op, $arg = $cmd -split '=', 2
    Log "> $op $(if ($op -eq 'fill' -or $op -eq 'type') { '<texto>' } else { $arg })"
    try {
        switch ($op) {
            'start' { if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'explorador' } else { [void](Focus-App) } }
            'close' { [void](Close-App 20) }
            'size' { $w, $h = $arg -split 'x'; Log "   tamano correcto = $(Size-Window ([int]$w) ([int]$h))" }
            'names' { Show-Names 300 }
            'rects' { Print-Rects }
            'all' { Print-Rects -All }
            'shot' { Shot-Client $arg }
            'shotk' { Shot-Client $arg -KeepPointer }
            'click' { Click-El (Find-El -Name $arg -Timeout 5) }
            'clicklike' { Click-El (Find-El -Like $arg -Timeout 5) }
            'clickid' { Click-El (Find-El -Id $arg -Timeout 5) }
            'clicktype' { $t, $n = $arg -split ':', 2; Click-El (Find-El -Name $n -Type $t -Timeout 5) }
            'rclick' { $c = Center-Of (Find-El -Name $arg -Timeout 5); [void](Focus-App); [void][QA.Native]::SetCursorPos($c[0], $c[1]); Start-Sleep -Milliseconds 120; [QA.Native]::mouse_event(0x0008, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 40; [QA.Native]::mouse_event(0x0010, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 600 }
            'hover' { $c = Center-Of (Find-El -Name $arg -Timeout 5); [void](Focus-App); [void][QA.Native]::SetCursorPos($c[0], $c[1]); Start-Sleep -Milliseconds 1200 }
            'fill' { $id, $text = $arg -split ':', 2; Fill-Id $id $text }
            'type' { [void](Focus-App); Type-Text $arg }
            'key' { [void](Focus-App); Press $arg }
            'ctrl' { [void](Focus-App); Press-Ctrl ([char]$arg) }
            'wheel' { [void](Focus-App); $i = Get-WinInfo; [void][QA.Native]::SetCursorPos($i.X + [int]($i.OuterW * 0.6), $i.Y + [int]($i.OuterH / 2)); Start-Sleep -Milliseconds 100; [QA.Native]::mouse_event(0x0800, 0, 0, [BitConverter]::ToUInt32([BitConverter]::GetBytes([int]([int]$arg * 120)), 0), [UIntPtr]::Zero); Start-Sleep -Milliseconds 500 }
            'filedialog' { New-Item -ItemType Directory -Force $Work | Out-Null; Answer-FileDialog (Join-Path $Work $arg) }
            'select' { $id, $like = $arg -split ':', 2; Log "   elegido = $(Select-Option $id $like); opciones: $($LastOptions -join ' | ')" }
            'famkey' { Answer-FileDialog $FamKey }
            'hoverid' { $c = Center-Of (Find-El -Id $arg -Timeout 5); [void](Focus-App); [void][QA.Native]::SetCursorPos($c[0], $c[1]); Start-Sleep -Milliseconds 80; [void][QA.Native]::SetCursorPos($c[0] + 3, $c[1] + 1); Start-Sleep -Milliseconds 1800; Log "   ayuda (HelpText) presente = $([bool](Find-El -Id $arg).Current.HelpText)" }
            'sleep' {Start-Sleep -Milliseconds ([int]$arg) }
            'focus' { Log "   foco: $(Focused-Desc)" }
            'wait' { Log "   aparecio a los $(Wait-Text $arg 15) ms" }
            default { Log "   orden desconocida: $op" }
        }
    } catch { Log "   FALLO: $($_.Exception.Message)" }
}
