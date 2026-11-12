# R-4, paso 1 (control positivo): sobre la version INSTALADA antes del arreglo, seleccionar el valor de una
# contrasena mostrada y Ctrl+C. Ejecutar con RF_VARIANT=vn-lib (sin VN_EXE). Deja creada la boveda de prueba.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
Step 'VN1 Control positivo del historial del portapapeles (WinRT)' -Continue {
    $enabled = try { [Windows.ApplicationModel.DataTransfer.Clipboard]::IsHistoryEnabled() } catch { "error: $($_.Exception.Message)" }
    Set-ClipExternal 'centinela-vn-control'
    $seen = 'no'
    for ($k = 0; $k -lt 25 -and $seen -eq 'no'; $k++) { Start-Sleep -Milliseconds 200; $seen = History-Has 'centinela-vn-control' }
    Set-State 'historial' $seen
    Check 'CONTROL POSITIVO: una copia sin marcar hecha por otro proceso aparece en el historial (Win+V)' ($seen -eq 'si') "historial encendido = $enabled; visto = $seen"
}
Step 'VN2 Version instalada, boveda y entrada de prueba' {
    New-Item -ItemType Directory -Force $Work | Out-Null
    Set-State 'inicio' ((Get-Date).ToString('o'))
    Log "BINARIOS: instalado sha256 $(Sha $InstalledExe) ($((Get-Item $InstalledExe).Length) bytes); de prueba (solo capa nativa) sha256 $(Sha $IsolatedExe) ($((Get-Item $IsolatedExe).Length) bytes); instalador nuevo sha256 $(Sha (Join-Path $Repo 'target\release\bundle\nsis\AlmarioKS_0.1.0_x64-setup.exe'))"
    Check 'La version instalada es la de la corrida anterior (dos capas, valor todavia seleccionable)' ((Sha $InstalledExe) -eq '2658300083BA543D')
    Start-Checked 'instalada, antes del arreglo de R-4'
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $RfVault
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-Name -Name 'Crear bóveda'
    Check 'Boveda creada y abierta' (($null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 90 -Quiet)) -and (Test-Path $RfVault))
    Select-Group $RootName
    Click-Name -Name 'Nueva entrada'
    [void](Wait-Text 'Nueva entrada en *' 5)
    Fill-Id 'entry-title' $Rf.Title
    Fill-Id 'entry-username' $Rf.User
    Fill-Id 'entry-password' $Rf.Pass -Secret
    Fill-Id 'entry-url' $Rf.Url
    Fill-Id 'entry-notes' $Rf.Notes
    Fill-Id 'entry-tags' $Rf.Tag
    $s = Save-EntryTimed
    Check 'Entrada con la contrasena canario guardada' ($s.Msg -ge 0 -and $s.Disk -ge 0)
    Start-Sleep -Milliseconds 800
    Check 'Grupo con nombre canario' (New-Group $RootName $Rf.Group)
    Start-Sleep -Milliseconds 2500
    Open-Entry $RootName $Rf.Title
}
Step 'VN3 R-4: seleccionar el valor mostrado y Ctrl+C' -Continue {
    $st = 'R-4, version sin el arreglo'
    $hits = Copy-Gestures $st 'VN10'
    Set-State 'r4-control' "$hits"
    Check 'R-4 SE REPRODUCE: algun gesto deja la contrasena mostrada en el portapapeles por la via del navegador' ($hits -gt 0) "$hits de 5 gestos"
    $r = Try-Copy $st 'doble clic sobre el valor (para la espera de 25 s)' $SelDouble $Rf.Pass
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Pointer-Away
    while ($sw.Elapsed.TotalSeconds -lt 23) { Start-Sleep -Milliseconds 500 }
    $still = "$(Get-Clip)".Contains($Rf.Pass)
    Row $st 'el mismo portapapeles 25 s despues de Ctrl+C' $(if ($still) { 'SENAL: el valor sigue en el portapapeles (sin limpieza)' } else { 'el portapapeles ya no tiene el valor' })
    Start-Sleep -Milliseconds 500
    [void](Lock-Timed)
    $afterLock = "$(Get-Clip)".Contains($Rf.Pass)
    Row $st 'el mismo portapapeles tras bloquear la boveda' $(if ($afterLock) { 'SENAL: el valor sigue en el portapapeles tras bloquear' } else { 'el bloqueo lo limpio' })
    Row $st 'historial al final' "contiene el valor: $(History-Has $Rf.Pass)"
    Fill-Id 'unlock-password' $RfMaster -Secret
    Click-Name -Name 'Desbloquear'
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 20)
}
Step 'VN4 Linea base: el boton "Copiar" de la contrasena' -Continue {
    Open-Entry $RootName $Rf.Title
    Set-ClipExternal 'centinela-vn-base'; Start-Sleep -Milliseconds 300
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Click-Name -Name 'Copiar contraseña'
    Start-Sleep -Milliseconds 600
    $clip = "$(Get-Clip)"; $formats = @(Get-ClipFormats)
    $excl = @($ExclFormats | Where-Object { $formats -contains $_ }).Count
    Start-Sleep -Milliseconds 2000
    $hist = History-Has $Rf.Pass
    Pointer-Away
    $cleared = -1
    while ($sw.Elapsed.TotalSeconds -lt 35) { if ((Get-Clip) -eq '') { $cleared = $sw.Elapsed.TotalSeconds; break }; Start-Sleep -Milliseconds 200 }
    Check 'Boton "Copiar": valor exacto, 3 formatos de exclusion, fuera del historial y vacio a los ~20 s' ($clip -ceq $Rf.Pass -and $excl -eq 3 -and $hist -ne 'si' -and $cleared -ge 19 -and $cleared -le 23) ("formatos $excl de 3; historial: $hist; vacio a los {0:n1} s" -f $cleared)
}
Step 'VN5 Arrastrar hacia fuera, a una ventana propia (control)' -Continue {
    $box = New-DropBox
    try {
        Open-Entry $RootName $Rf.Title
        $pt = Notes-Point
        $d1 = Drag-Out $pt[0] $pt[1] $box
        $okNotes = ($d1 -and $Rf.Notes.Contains("$d1".Trim()) -and "$d1".Trim().Length -ge 3)
        $r = Pass-Rect
        $d2 = Drag-Out ([int]($r.X + $r.Width / 2)) ([int]($r.Y + $r.Height / 2)) $box
        $okPass = ("$d2".Contains($Rf.Pass))
        Set-State 'arrastre-control' "notas=$okNotes;valor=$okPass"
        Row 'R-4, version sin el arreglo' 'arrastrar una palabra de las notas a una ventana propia' $(if ($okNotes) { 'llega (el metodo de arrastre funciona)' } else { "NO PROBADO: no llego nada (recibido de $("$d1".Length) caracteres)" })
        Row 'R-4, version sin el arreglo' 'arrastrar el valor mostrado a una ventana propia' $(if ($okPass) { 'SENAL: el valor llega a la otra ventana' } else { "no llega (recibido de $("$d2".Length) caracteres)" })
    } finally { $box.Close(); $box.Dispose() }
}
Step 'VN6 Otro sitio donde un secreto es texto: el generador' -Continue {
    [void](Focus-App)
    Generator-Copy 'R-4, version sin el arreglo'
}
Step 'VN7 Limpiar portapapeles e historial y cerrar' {
    [System.Windows.Forms.Clipboard]::Clear()
    $n = Purge-Mine
    Log "LIMPIEZA: elementos de esta prueba borrados del historial: $n; queda el canario en el historial: $(History-Has 'CENTELLA'); portapapeles de $("$(Get-Clip)".Length) caracteres"
    Start-Sleep -Milliseconds 2000
    Check 'Cierre normal de la version instalada' (Close-App 20)
    Start-Sleep -Seconds 2
}
Rows-Summary
Summary
