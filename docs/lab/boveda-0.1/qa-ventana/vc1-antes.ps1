# Cierre de la superficie de navegador (1/4): instalar el release nuevo SIN borrar el perfil, comprobar que arranca
# y abre centrada, controles positivos del detector, y la tanda ANTES de cualquier bloqueo: bienvenida, Crear y
# boveda abierta con una contrasena mostrada. Ejecutar con RF_VARIANT=vc-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$setup = Join-Path $Repo 'target\release\bundle\nsis\AlmarioKS_0.1.0_x64-setup.exe'
$built = Join-Path $Repo 'target\release\almarioks.exe'
Step 'VC1 Instalar el release nuevo (el perfil de WebView2 se conserva)' {
    if (@(Get-Process almarioks -ErrorAction SilentlyContinue).Count -gt 0) { throw 'La app esta abierta: se esperaba cerrada' }
    New-Item -ItemType Directory -Force $Work | Out-Null
    Set-State 'inicio' ((Get-Date).ToString('o'))
    $old = Sha $AppExe
    $head = (& git -C $Repo log -1 --format='%cI %h')
    $dirty = @(& git -C $Repo status --short -- src src-tauri crates Cargo.toml package.json).Count
    Log "Instalado antes: $((Get-Item $AppExe).Length) bytes, sha256 $old"
    Log "Instalador: $((Get-Item $setup).Length) bytes, $((Get-Item $setup).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss zzz')), sha256 $(Sha $setup); HEAD: $head; cambios de producto sin commit: $dirty"
    Check 'El instalador es posterior a HEAD y no hay cambios de producto sin commit' (([DateTimeOffset](Get-Item $setup).LastWriteTime) -gt [DateTimeOffset]::Parse(($head -split ' ')[0]) -and $dirty -eq 0)
    $p = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
    Check 'Instalador /S termina con codigo 0' ($p.ExitCode -eq 0)
    Start-Sleep -Seconds 2
    $latin = [Text.Encoding]::GetEncoding(28591)
    $a = $latin.GetString([IO.File]::ReadAllBytes($AppExe))
    $b = $latin.GetString([IO.File]::ReadAllBytes($built)).Replace('__TAURI_BUNDLE_TYPE_VAR_UNK', '__TAURI_BUNDLE_TYPE_VAR_NSS')
    Log "Instalado ahora: $((Get-Item $AppExe).Length) bytes, sha256 $(Sha $AppExe), version $([Diagnostics.FileVersionInfo]::GetVersionInfo($AppExe).ProductVersion); EBWebView conservada = $(Test-Path $EbDir)"
    Check 'El exe instalado es el de target\release y ya no es el de la corrida anterior' ([string]::Equals($a, $b, [StringComparison]::Ordinal) -and (Sha $AppExe) -ne $old -and (Sha $AppExe) -ne '77FE2A351016AE57')
    $a = $null; $b = $null
}
Step 'VC2 La app arranca y abre centrada (D-12)' {
    [void]$AE::RootElement.FindAll($TreeScope::Children, [System.Windows.Automation.Condition]::TrueCondition).Count
    $sw = [Diagnostics.Stopwatch]::StartNew()
    Start-App 40
    $ms = Wait-Text 'Bienvenida' 40
    Log "TIEMPO arranque [tras instalar]: ventana a los $LastStartMs ms; 'Bienvenida' legible a los $([int]$sw.ElapsedMilliseconds) ms"
    if ((Get-AppHwnd) -eq [IntPtr]::Zero -or $ms -lt 0) {
        Check 'La app ARRANCA con las dos capas de arreglo' $false "procesos almarioks = $(@(Get-Process almarioks -ErrorAction SilentlyContinue).Count); msedgewebview2 = $(@(Get-Process msedgewebview2 -ErrorAction SilentlyContinue).Count)"
        Get-ChildItem $Work -Filter 'app-*.log' | ForEach-Object { Log "   $($_.Name) ($($_.Length) bytes): $(Redact ((Get-Content -Raw $_.FullName) -replace '\s+', ' '))" }
        try { Shot 'VC00-no-arranca' } catch { }
        Summary; exit 3
    }
    Check 'La app ARRANCA con las dos capas de arreglo' $true "bienvenida a los $([int]$sw.ElapsedMilliseconds) ms"
    $w = Where-Window 'primer arranque, sin tocarla'
    Check 'D-12: abre a 1120 x 720, dentro del area util y centrada' ($w.Inside -and [math]::Abs($w.Dx) -le 12 -and [math]::Abs($w.Dy) -le 12 -and [math]::Abs($w.W - 1120) -le 1 -and [math]::Abs($w.H - 720) -le 1) "desvio $($w.Dx), $($w.Dy) px; $($w.W) x $($w.H)"
    Shot 'VC01-bienvenida-centrada'
}
Step 'VC3 Controles positivos del detector' -Continue {
    [void](Focus-App)
    $f0 = Focused-Desc; Press 'TAB'; Start-Sleep -Milliseconds 250; $f1 = Focused-Desc; Press 'TAB'; Start-Sleep -Milliseconds 250; $f2 = Focused-Desc
    Check 'CONTROL: las teclas llegan a la pagina (Tab mueve el foco)' ($f1 -ne $f0 -or $f2 -ne $f1) "$f0 -> $f1 -> $f2"
    Check 'CONTROL: en reposo el detector no da senal' (@(Surface-Signal).Count -eq 0) (@(Surface-Signal) -join '; ')
    Click-Name -Like 'Abrir una bóveda*'
    Start-Sleep -Milliseconds 1800
    $sig = @(Surface-Signal)
    Log "   con el dialogo de elegir archivo abierto: $($sig -join '; ')"
    Shot-Area 'VC02-control-dialogo-nativo'
    Check 'CONTROL POSITIVO: el detector ve un dialogo nativo (#32770 y primer plano fuera de la app)' (@($sig | Where-Object { $_ -like 'dialogo nativo #32770*' }).Count -eq 1 -and @($sig | Where-Object { $_ -like 'primer plano fuera*' }).Count -eq 1) ($sig -join '; ')
    Press 'ESC'; Start-Sleep -Milliseconds 900
    Check 'CONTROL: cerrado el dialogo, la senal desaparece' (@(Surface-Signal).Count -eq 0) (@(Surface-Signal) -join '; ')
    $r = [QA.Native]::VisibleRect((Get-AppHwnd))
    $m = Probe-RightClick 'CONTROL: barra de titulo' ($r.Left + 260) ($r.Top + 16) 'VC03-control-menu-del-sistema'
    Check 'CONTROL POSITIVO: Probe-RightClick lee un menu cuando existe (el del sistema, en la barra de titulo)' ($m.Items.Count -ge 3) ($m.Items -join ' | ')
    Start-Sleep -Milliseconds 400
    $d0 = Doc-Id
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 5); $d1 = Doc-Id
    Click-Name -Name 'Volver a la bienvenida'; [void](Find-El -Id 'welcome-title' -Timeout 5); $d2 = Doc-Id
    Set-State 'doc-inicial' $d0
    Check 'CONTROL NEGATIVO: navegar dentro de la app no cambia la identidad del documento' ($d0 -ne '' -and $d0 -eq $d1 -and $d1 -eq $d2) "$d0 / $d1 / $d2"
}
Step 'VC4 Bienvenida y Crear, en arranque en frio y antes de cualquier bloqueo' -Continue {
    $st = 'bienvenida, antes de bloquear'
    $c = Center-Of (Find-El -Id 'welcome-title')
    [void](RC-Row $st 'el titulo' $c[0] $c[1] 'VC04-bienvenida-clic-derecho')
    $i = Get-WinInfo
    [void](RC-Row $st 'el fondo' ($i.X + 80) ($i.Y + $i.OuterH - 140))
    $c = Center-Of (Find-El -Name 'Crear bóveda nueva' -Type Button)
    [void](RC-Row $st 'un boton' $c[0] $c[1])
    [void](Probe-Surface $st 'Ctrl+P' 'VC05-bienvenida-ctrl-p')
    Probe-Many $st @('Ctrl+Shift+P', 'Ctrl+S', 'F3', 'Ctrl+G', 'Ctrl+F', 'Ctrl+J', 'Ctrl+U', 'F12', 'Ctrl+Shift+I', 'Ctrl+Shift+S', 'Alt+Izquierda', 'Shift+F10', 'Tecla Menu')
    # La bienvenida no guarda estado propio: aqui la recarga solo se ve por la identidad del documento.
    foreach ($l in @('F5', 'Ctrl+R')) { [void](Probe-ReloadKey $st $l { $true } 'solo identidad del documento') }
    $st = 'crear, antes de bloquear'
    Click-Name -Name 'Crear bóveda nueva'
    [void](Find-El -Id 'create-title' -Timeout 5)
    Click-Name -Name 'Elegir dónde guardarla…'
    Answer-FileDialog -path $RfVault
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfOther -Secret
    Click-Name -Name 'Crear bóveda'
    Start-Sleep -Milliseconds 900
    $marker = { @(Get-Names | Where-Object { $_ -like '*no coinciden*' }).Count -gt 0 }
    Check 'Crear: el error "no coinciden" esta en pantalla (marcador de recarga)' ([bool](& $marker))
    $c = Center-Of (Find-El -Id 'create-password')
    [void](RC-Row $st 'el campo de la contrasena maestra' $c[0] $c[1] 'VC06-crear-clic-derecho-campo')
    foreach ($l in @('F5', 'Ctrl+R', 'Ctrl+Shift+R', 'Ctrl+F5')) { [void](Probe-ReloadKey $st $l $marker 'foco en un campo') }
    [void](Probe-Surface $st 'Ctrl+P' '' 'foco en un campo')
    Probe-Many $st @('Ctrl+S', 'F3', 'Ctrl+F', 'Ctrl+J', 'Shift+F10', 'Tecla Menu')
    Log "   al final: pantalla = $(Screen-Ext); marcador presente = $([bool](& $marker))"
}
Step 'VC5 Crear la boveda, una entrada y un grupo' {
    if ((Screen-Ext) -ne 'crear') { throw "Se esperaba la pantalla Crear y es $(Screen-Ext)" }
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
    Check 'Entrada con los canarios guardada (aviso despues del disco)' ($s.Msg -ge 0 -and $s.DiskAtMsg -eq $true)
    Start-Sleep -Milliseconds 800
    Check 'Grupo con nombre canario' (New-Group $RootName $Rf.Group)
    Start-Sleep -Milliseconds 2500
}
Step 'VC6 Boveda abierta con la contrasena mostrada, antes de cualquier bloqueo' -Continue {
    Vault-Battery 'boveda abierta, antes de bloquear' 'VC10'
    Log "   pantalla al terminar: $(Screen-Ext) (si no es 'boveda abierta', hubo un bloqueo no previsto)"
}
Rows-Summary
Summary
