# R-4, paso 3: instalar el release nuevo (dos capas + el valor mostrado no se puede seleccionar, copiar ni arrastrar
# por el navegador) y repetir LOS MISMOS gestos del paso 1, mas lo que debe seguir funcionando y un humo corto.
# Ejecutar con RF_VARIANT=vn-lib (sin VN_EXE). Usa la boveda que dejo vn1.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$setup = Join-Path $Repo 'target\release\bundle\nsis\AlmarioKS_0.1.0_x64-setup.exe'
$built = Join-Path $Repo 'target\release\almarioks.exe'
Step 'VN20 Instalar el release nuevo' {
    if ((Any-AlmarioProc).Count -gt 0) { throw 'Hay procesos almarioks* vivos: se esperaba todo cerrado' }
    $old = Sha $AppExe
    $head = (& git -C $Repo log -1 --format='%cI %h' -- src src-tauri crates Cargo.toml package.json)
    $dirty = @(& git -C $Repo status --short -- src src-tauri crates Cargo.toml package.json).Count
    Log "Instalado antes: sha256 $old; instalador: $((Get-Item $setup).Length) bytes, $((Get-Item $setup).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss zzz')), sha256 $(Sha $setup); ultimo commit de producto: $head; cambios de producto sin commit: $dirty"
    Check 'El instalador es posterior al ultimo commit de producto y no hay cambios de producto sin commit' (([DateTimeOffset](Get-Item $setup).LastWriteTime) -gt [DateTimeOffset]::Parse(($head -split ' ')[0]) -and $dirty -eq 0)
    $p = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
    Check 'Instalador /S termina con codigo 0' ($p.ExitCode -eq 0)
    Start-Sleep -Seconds 2
    $latin = [Text.Encoding]::GetEncoding(28591)
    $a = $latin.GetString([IO.File]::ReadAllBytes($AppExe))
    $b = $latin.GetString([IO.File]::ReadAllBytes($built)).Replace('__TAURI_BUNDLE_TYPE_VAR_UNK', '__TAURI_BUNDLE_TYPE_VAR_NSS')
    Log "Instalado ahora: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($AppExe).ProductVersion)"
    Check 'El exe instalado es el de target\release y ya no es el de antes' ([string]::Equals($a, $b, [StringComparison]::Ordinal) -and (Sha $AppExe) -ne $old)
    $a = $null; $b = $null
}
Step 'VN21 Arranca, abre centrada y abre la boveda' {
    Start-Checked 'instalada, release nuevo'
    $w = Where-Window 'release nuevo, sin tocarla'
    Check 'Abre centrada y dentro del area util' ($w.Inside -and [math]::Abs($w.Dx) -le 12 -and [math]::Abs($w.Dy) -le 12)
    Click-Name -Name 'Abrir rf-uno.kdbx'
    [void](Find-El -Id 'unlock-title' -Timeout 5)
    Start-Sleep -Milliseconds 400
    [void](Unlock-Timed 'release nuevo')
    Open-Entry $RootName $Rf.Title
}
Step 'VN22 R-4 en la version arreglada: los mismos gestos' -Continue {
    $st = 'R-4, version arreglada'
    $shown = Show-Pass
    Check 'El valor mostrado sigue en el arbol de la ventana (lo lee un lector de pantalla)' $shown
    $hits = Copy-Gestures $st 'VN50'
    Check 'R-4 CORREGIDO: ninguno de los cinco gestos deja el valor (ni un trozo) en el portapapeles' ($hits -eq 0) "$hits de 5 gestos lo dejan; en el paso 1 fueron $(Get-State 'r4-control') de 5 (mas el arrastre, que copio 19 de 20 caracteres)"
    # Ctrl+X por si acaso, con el valor seleccionado por doble clic (cortar tambien se cancela).
    Set-ClipExternal 'centinela-vn-cortar'; Start-Sleep -Milliseconds 300
    & $SelDouble; Start-Sleep -Milliseconds 300
    Press-Ctrl 'x'; Start-Sleep -Milliseconds 600
    $cut = "$(Get-Clip)"
    Row $st 'doble clic sobre el valor + Ctrl+X' $(if ($cut.Contains($Rf.Pass.Substring(4, 8))) { 'SENAL: el portapapeles lleva el valor' } elseif ($cut -ceq 'centinela-vn-cortar') { 'sigue el centinela: no se copio nada' } else { "otro texto de $($cut.Length) caracteres, sin el valor" })
    Check 'Tras los gestos el valor sigue en la boveda y en pantalla (nada se corto)' (Show-Pass)
    Generator-Copy $st
}
Step 'VN23 Lo que debe seguir funcionando' -Continue {
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
    Check 'Boton "Copiar": valor exacto, 3 formatos de exclusion, fuera del historial y vacio a los ~20 s' ($clip -ceq $Rf.Pass -and $excl -eq 3 -and $hist -eq 'no' -and $cleared -ge 19 -and $cleared -le 23) ("formatos $excl de 3; historial: $hist; vacio a los {0:n1} s" -f $cleared)
    Click-Name -Name 'Copiar usuario'; Start-Sleep -Milliseconds 600
    Check '"Copiar usuario" deja el usuario en el portapapeles' ("$(Get-Clip)" -ceq $Rf.User)
    # Otro texto de la pagina: una palabra de las notas (doble clic) y el titulo (triple clic), con Ctrl+C.
    Set-ClipExternal 'centinela-vn-notas'; Start-Sleep -Milliseconds 300
    $pt = Notes-Point
    [QA.Native]::ClickAt($pt[0], $pt[1]); Start-Sleep -Milliseconds 60; [QA.Native]::ClickAt($pt[0], $pt[1]); Start-Sleep -Milliseconds 350
    Shot-Area 'VN60-notas-seleccionadas'
    Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    $word = "$(Get-Clip)".Trim(); $f2 = @(Get-ClipFormats)
    Log "   copia por el navegador de un texto NO protegido: formatos del portapapeles: $($f2 -join ', ')"
    Check 'Seleccionar y copiar con Ctrl+C una palabra de las notas sigue funcionando' ($word.Length -ge 3 -and $Rf.Notes.Contains($word)) "copiado: '$word'; formatos de exclusion presentes: $((@($ExclFormats | Where-Object { $f2 -contains $_ })) -join ', ')"
    Set-ClipExternal 'centinela-vn-titulo'; Start-Sleep -Milliseconds 300
    $t = Find-El -Name $Rf.Title -Type Text -Timeout 2
    $c = Center-Of $t
    [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 50; [QA.Native]::ClickAt($c[0], $c[1]); Start-Sleep -Milliseconds 350
    Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    Check 'Seleccionar y copiar con Ctrl+C el titulo de la entrada sigue funcionando' ("$(Get-Clip)".Trim() -ceq $Rf.Title) "copiado de $("$(Get-Clip)".Trim().Length) caracteres"
    # Texto dentro de un campo del formulario de edicion.
    Click-Name -Name 'Editar'; [void](Wait-Text 'Editar *' 4)
    Set-ClipExternal 'centinela-vn-campo'; Start-Sleep -Milliseconds 300
    Click-El (Find-El -Id 'entry-notes'); Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    Check 'Ctrl+A y Ctrl+C dentro del campo de notas del formulario de edicion' ("$(Get-Clip)" -ceq $Rf.Notes)
    Click-El (Find-El -Id 'entry-title'); Press-Ctrl 'a'; Press-Ctrl 'c'; Start-Sleep -Milliseconds 600
    Check 'Ctrl+A y Ctrl+C dentro del campo del titulo del formulario de edicion' ("$(Get-Clip)" -ceq $Rf.Title)
    for ($k = 0; $k -lt 3 -and (Has-El -Name 'Guardar entrada' -Type Button -Timeout 0.4); $k++) { Click-Name -Name 'Cancelar'; Start-Sleep -Milliseconds 400 }
    Open-Entry $RootName $Rf.Title
    $on = Show-Pass
    Shot 'VN61-valor-mostrado'
    Click-Name -Name 'Ocultar contraseña'; Start-Sleep -Milliseconds 400
    Check 'Mostrar y ocultar: el valor entra y sale del arbol de la ventana' ($on -and -not ((Get-Names) -contains $Rf.Pass))
}
Step 'VN24 Humo corto' -Continue {
    Start-Sleep -Milliseconds 1500
    $l = Lock-Timed
    Check 'Bloquear deja el portapapeles sin canarios' (-not "$(Get-Clip)".Contains('CENTELLA'))
    $u = Unlock-Timed 'humo'
    Press-Ctrl 'k'; Start-Sleep -Milliseconds 300
    Type-Text $Rf.Partial; Start-Sleep -Milliseconds 900
    Press 'ENTER'; Start-Sleep -Milliseconds 700
    $f = Focused-Desc
    Press 'ENTER'; Start-Sleep -Milliseconds 500
    Check 'Ctrl+K, buscar, Enter abre con el foco en "Copiar contraseña" y Enter copia' ($f -like '*Copiar contraseña*' -and "$(Get-Clip)" -ceq $Rf.Pass) $f
    [void](Lock-Timed)
    Check 'Bloquear al final: pantalla de desbloqueo y portapapeles vacio' ((Screen-Ext) -eq 'desbloqueo' -and (Get-Clip) -eq '')
}
Step 'VN25 Limpiar portapapeles e historial y cerrar' {
    [System.Windows.Forms.Clipboard]::Clear()
    $n = Purge-Mine
    Log "LIMPIEZA: elementos de esta prueba borrados del historial: $n; queda el canario en el historial: $(History-Has 'CENTELLA')"
    Start-Sleep -Milliseconds 1500
    Check 'Cierre normal' (Close-App 20)
    Start-Sleep -Seconds 3
}
Rows-Summary
Summary
