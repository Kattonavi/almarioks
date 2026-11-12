# Pase de interaccion (10): reverificacion con el build final. En este orden:
#  A. "Crear boveda" a 1120 x 720 con "Familia" y la contrasena maestra escrita: ¿cabe sin barra de desplazamiento? + captura.
#  B. P-1: los campos de contrasena protegidos no dejan copiar ni cortar (maestra en crear; Contrasena del dialogo de
#     entrada tras "Generar"), con control positivo (Titulo/Nombre si copian) y sin romper escribir, pegar y borrar.
#  C. P-2: "Crear uno nuevo" con las dos contrasenas vacias no crea nada (en la carpeta temporal, no en Documentos).
#  Deja creada la boveda de muestra "Cuentas" con sus 5 grupos, lista para px2-construir.ps1 y px3-capturas.ps1.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'px-pasos.ps1'), [Text.Encoding]::UTF8)))
$script:ProbeN = 0
# Centinela al portapapeles, clic en el campo, Ctrl+A y Ctrl+C (o Ctrl+X). Devuelve lo que quedo.
function Field-Copy($field, [string]$needle, [switch]$Cut) {
    $script:ProbeN++
    $sent = "centinela-px-final-$ProbeN"
    Set-ClipExternal $sent; Start-Sleep -Milliseconds 300
    Click-El $field; Press-Ctrl 'a'; Start-Sleep -Milliseconds 120
    if ($Cut) { Press-Ctrl 'x' } else { Press-Ctrl 'c' }
    Start-Sleep -Milliseconds 600
    $got = "$(Get-Clip)"
    $mid = if ($needle.Length -ge 14) { $needle.Substring(4, 8) } else { $needle }
    $has = $got.Contains($needle) -or $got.Contains($mid)
    $text = if ($got -ceq $needle) { 'EXACTAMENTE el valor' } elseif ($has) { 'el valor o un trozo' } elseif ($got -ceq $sent) { 'sigue el centinela: no se copio nada' } else { "otro texto de $($got.Length) caracteres, sin el valor" }
    return [pscustomobject]@{ Has = $has; Exact = ($got -ceq $needle); Text = $text }
}
function Shown-Value([string]$id) {
    $f = Find-El -Id $id -Timeout 3
    if ($f.Current.IsPassword) { Click-Name -Name 'Mostrar contraseña'; Start-Sleep -Milliseconds 350; $f = Find-El -Id $id -Timeout 3 }
    if ($f.Current.IsPassword) { return $null }
    return "$(Get-Value $f)"
}
# La linea de la ubicacion se redibuja un instante despues de teclear el nombre y el enlace cambia de sitio: se espera
# antes de buscarlo y, si el dialogo nativo no aparece, se repite el clic una vez.
function Set-Location-Px([string]$file) {
    Start-Sleep -Milliseconds 900
    Click-Name -Name 'Cambiar ubicación…'
    if ((Find-FileDialog 4) -eq [IntPtr]::Zero) { Log '   (el dialogo nativo no aparecio con el primer clic: se repite)'; Start-Sleep -Milliseconds 500; Click-Name -Name 'Cambiar ubicación…' }
    Answer-FileDialog (Join-Path $Work $file); Start-Sleep -Milliseconds 600; [void](Focus-App)
}
$from = if ($StepArgs.Count) { [int]$StepArgs[0] } else { 100 }   # para reanudar: 102 salta A y B de crear

if ($from -le 100) { Step 'PX100 (A) Crear boveda a 1120 x 720 con la contrasena escrita' {
    New-Item -ItemType Directory -Force $Work | Out-Null
    if ((Any-AlmarioProc).Count -eq 0) { Start-Checked 'build final' }
    Check 'Ventana a 1120 x 720 logicos' (Size-Window 1120 720)
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
    $g0 = LRect (Find-El -Name 'Crear bóveda' -Type Group)
    Fill-Id 'create-name' 'Familia'
    Fill-Id 'create-password' $FamMaster -Secret
    Click-El (Find-El -Id 'create-title'); Start-Sleep -Milliseconds 500
    $g = LRect (Find-El -Name 'Crear bóveda' -Type Group)
    $warn = LRect (Find-El -Like 'Si pierdes la contraseña maestra*' -Type Text)
    $btn = LRect (Find-El -Name 'Cancelar' -Type Button)
    $form = LRect (Find-El -Id 'create-name')
    $strength = Has-Name 'Fortaleza*'
    $line = (Get-Names | Where-Object { $_ -like 'Se guarda en*' -or $_ -eq 'Familia.kdbx' }) -join ' '
    $radio = (Find-El -Name 'Crear uno nuevo' -Type RadioButton).GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Current.IsSelected
    Log "   contenido: vacio $($g0.W) x $($g0.H); con la contrasena $($g.W) x $($g.H) (ventana 880 x 720 a la derecha del panel de 240); campo Nombre x=$($form.X) w=$($form.W); aviso de y=$($warn.Y) a $($warn.B); botones desde y=$($btn.Y); fortaleza visible = $strength; linea: $line; 'Crear uno nuevo' marcado = $radio"
    Check '(A) Con la contrasena escrita el contenido cabe en 720 px: sin barra de desplazamiento' ($g.H -le 720.5 -and $g.W -ge 879 -and $strength) "alto del contenido $($g.H) / 720; ancho $($g.W) / 880"
    Check '(A) El aviso "Si pierdes..." queda entero por encima de los botones' ($warn.B -lt $btn.Y) "aviso termina en $($warn.B); botones empiezan en $($btn.Y)"
    Shot-Client 'app-crear-1120x720'
} }
if ($from -le 101) { Step 'PX101 (B) Crear boveda: la maestra a la vista no se copia ni se corta' -Continue {
    $shown = Shown-Value 'create-password'
    $c = Field-Copy (Find-El -Id 'create-password') $FamMaster
    $x = Field-Copy (Find-El -Id 'create-password') $FamMaster -Cut
    $after = Shown-Value 'create-password'
    $n = Field-Copy (Find-El -Id 'create-name') 'Familia'
    Check '(B) CONTROL POSITIVO: Ctrl+A + Ctrl+C en el campo Nombre si copia' $n.Exact $n.Text
    Check '(B) Maestra a la vista (ojo): Ctrl+A + Ctrl+C no la deja en el portapapeles' ($shown -ceq $FamMaster -and -not $c.Has -and $n.Exact) "a la vista = $($shown -ceq $FamMaster); Ctrl+C: $($c.Text)"
    Check '(B) Maestra a la vista: Ctrl+A + Ctrl+X no la saca y el campo conserva el valor' (-not $x.Has -and $after -ceq $FamMaster) "Ctrl+X: $($x.Text); el campo conserva el valor = $($after -ceq $FamMaster)"
} }
Step 'PX102 (C) Crear uno nuevo con las dos contrasenas vacias' -Continue {
    Fill-Id 'create-name' 'FAROPX-Vacia'
    Set-Location-Px 'Vacia.kdbx'
    Fill-Id 'create-password' ''
    Fill-Id 'create-confirm' ''
    Click-El (Find-El -Name 'Crear uno nuevo' -Type RadioButton); Start-Sleep -Milliseconds 300
    $radio = (Find-El -Name 'Crear uno nuevo' -Type RadioButton).GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Current.IsSelected
    $before = @(Get-Names)
    Click-Name -Name 'Crear bóveda' -Type Button
    Start-Sleep -Milliseconds 2500
    $new = @(Get-Names | Where-Object { $before -notcontains $_ } | Select-Object -Unique)
    Log "   'Crear uno nuevo' marcado = $radio; textos nuevos tras pulsar Crear: $($new -join ' | '); foco: $(Focused-Desc)"
    Shot-Area 'c-crear-sin-contrasena'
    Start-Sleep -Seconds 10
    $files = @(Get-ChildItem -LiteralPath $Work -Force | Where-Object { $_.Name -notlike 'app-*.log' } | ForEach-Object { $_.Name })
    Check '(C) Se niega: sigue en "Crear boveda", con un mensaje, y no aparece ningun archivo (ni .kdbx ni .keyx)' ($radio -and (Screen-Ext) -eq 'crear' -and $new.Count -gt 0 -and $files.Count -eq 0) "pantalla = $(Screen-Ext); archivos en la carpeta temporal 12 s despues: $(if ($files.Count) { $files -join ', ' } else { 'ninguno' })"
}
Step 'PX103 Boveda de muestra "Cuentas" y sus grupos' {
    if ((Screen-Ext) -ne 'crear') { throw "No estoy en Crear boveda: $(Screen-Ext)" }
    Fill-Id 'create-name' 'Cuentas'
    Set-Location-Px 'Cuentas.kdbx'
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-El (Find-El -Name 'Sin archivo llave' -Type RadioButton); Start-Sleep -Milliseconds 300
    Click-Name -Name 'Crear bóveda' -Type Button
    [void](Find-El -Name 'Bloquear' -Type Button -Timeout 45); Start-Sleep -Milliseconds 800
    foreach ($g in @('Personal', 'Trabajo', 'Bancos', 'Compras')) {
        Click-Name -Name 'Nuevo grupo'; [void](Find-El -Id 'group-name' -Timeout 5); Start-Sleep -Milliseconds 250
        Type-Text $g; Click-Name -Name 'Crear grupo'
        [void](Find-El -Like "$g · * entrada*" -Type Button -Timeout 8); Start-Sleep -Milliseconds 2300
    }
    Click-Name -Name 'Más acciones de Trabajo'; Start-Sleep -Milliseconds 400
    Click-El (Find-El -Name 'Nuevo subgrupo…' -Type MenuItem); [void](Find-El -Id 'group-name' -Timeout 5); Start-Sleep -Milliseconds 250
    $title = (Find-El -Type Window -Timeout 2).Current.Name
    Type-Text 'Clientes'; Click-El (Find-El -Like 'Crear*' -Type Button)
    [void](Find-El -Like 'Clientes · * entrada*' -Type Button -Timeout 8); Start-Sleep -Milliseconds 2300
    $tree = @(Get-All | Where-Object { (Type-Of $_) -eq 'Button' -and $_.Current.Name -match ' · \d+ entradas?$' } | ForEach-Object { $_.Current.Name -replace ' · .*', '' })
    Check 'Grupos creados en el orden del prototipo (Clientes con "Nuevo subgrupo…" dentro de Trabajo)' (($tree -join '|') -eq 'Todas las entradas|Personal|Trabajo|Clientes|Bancos|Compras') "$($tree -join ' | '); titulo del dialogo de subgrupo: $title"
}
Step 'PX104 (B) Dialogo de entrada: la contrasena generada a la vista no se copia ni se corta; escribir, pegar y borrar siguen' -Continue {
    Ensure-Cuentas; Pick-Group 'Personal'
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
    Type-Text $Rf.Title
    Click-Name -Name 'Generar'; Start-Sleep -Milliseconds 500
    $gen = Shown-Value 'entry-dialog-password'
    Add-Generated $gen
    $notes = LRect (Find-El -Id 'entry-dialog-notes')
    $c = Field-Copy (Find-El -Id 'entry-dialog-password') $gen
    $x = Field-Copy (Find-El -Id 'entry-dialog-password') $gen -Cut
    $after = Shown-Value 'entry-dialog-password'
    $t = Field-Copy (Find-El -Id 'entry-dialog-title') $Rf.Title
    Check '(B) CONTROL POSITIVO: Ctrl+A + Ctrl+C en el campo Titulo si copia' $t.Exact $t.Text
    Check '(B) Generada a la vista: clic en el campo, Ctrl+A, Ctrl+C no la deja en el portapapeles' ($gen.Length -ge 8 -and -not $c.Has -and $t.Exact) "generada a la vista de $($gen.Length) caracteres; Ctrl+C: $($c.Text)"
    Check '(B) Generada a la vista: Ctrl+A + Ctrl+X no la saca y el campo conserva el valor' (-not $x.Has -and $after -ceq $gen) "Ctrl+X: $($x.Text); el campo conserva el valor = $($after -ceq $gen)"
    # Lo que debe seguir funcionando en el campo protegido: pegar desde otro proceso, escribir y borrar.
    $pasted = 'FAROPXpegada7731k'
    Set-ClipExternal $pasted; Start-Sleep -Milliseconds 300
    Click-El (Find-El -Id 'entry-dialog-password'); Press-Ctrl 'a'; Press-Ctrl 'v'; Start-Sleep -Milliseconds 400
    $v1 = Shown-Value 'entry-dialog-password'
    Click-El (Find-El -Id 'entry-dialog-password'); Press-Ctrl 'a'; Type-Text $Rf.Pass -Secret; Start-Sleep -Milliseconds 300
    $v2 = Shown-Value 'entry-dialog-password'
    Click-El (Find-El -Id 'entry-dialog-password'); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 300
    $f = Find-El -Id 'entry-dialog-password'; $v3 = if ($f.Current.IsPassword) { '<oculto>' } else { "$(Get-Value $f)" }
    Check '(B) En el campo Contrasena siguen funcionando pegar (Ctrl+V de otro proceso), escribir y borrar' ($v1 -ceq $pasted -and $v2 -ceq $Rf.Pass -and $v3 -eq '') "pegado = $($v1 -ceq $pasted); tecleado encima = $($v2 -ceq $Rf.Pass); tras Ctrl+A + Supr queda vacio = $($v3 -eq '') ($v3)"
    Log "   Notas en el dialogo: $($notes.W) x $($notes.H) px"
    [System.Windows.Forms.Clipboard]::Clear()
    Press 'ESC'; Start-Sleep -Milliseconds 500
    $d = Find-El -Name 'Descartar' -Type Button -Timeout 0.6 -Quiet; if ($d) { Click-El $d; Start-Sleep -Milliseconds 400 }
    [void](Purge-Mine)
}
Summary
