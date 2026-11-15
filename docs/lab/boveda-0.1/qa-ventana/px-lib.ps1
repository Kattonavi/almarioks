# Pase de interaccion (2026-11-12): variante de rf-lib.ps1 (se carga con RF_VARIANT=px-lib) para el recorrido en la
# app instalada. Carga vn-lib.ps1 (historial del portapapeles, copia por el navegador, detector de la superficie de
# navegador) y pone encima carpetas, canarios y ayudas con los nombres accesibles de la interfaz nueva.
# Las capturas de trabajo van a la carpeta de estado (RF_STATE, fuera del repo): ahi pueden caer capturas de fallo con
# dialogos nativos. A .impeccable\review\pase\app solo se copia lo que ya se miro.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'vn-lib.ps1'), [Text.Encoding]::UTF8)))
$script:Shots = Join-Path $StateDir 'capturas'
New-Item -ItemType Directory -Force $Shots | Out-Null
$script:FinalShots = Join-Path $Repo '.impeccable\review\pase\app'
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-px-20261112'
$script:RfMaster = "PX-Maestra-Pase-64-$([char]0x00F1)$([char]0x20AC)"
$script:FamMaster = "PX-Familia-Llave-27-$([char]0x00F1)$([char]0x20AC)"
$script:RfOther = 'PX-Repeticion-Otra-35'
# La contrasena canario no lleva separadores: asi el doble clic la selecciona entera.
$script:Rf = @{
    Title = 'FAROPX-Titulo'; User = 'FAROPX-usuario@example.com'; Url = 'https://example.com/FAROPX-url'
    Notes = 'FAROPX-notas reservadas'; Group = 'FAROPX-Grupo'; Pass = 'FAROPXclave5306wq'; Draft = 'FAROPX-borrador-sin-guardar'
    Title2 = 'FAROPX-Generada'
}
$script:VaultName = 'Cuentas'
$script:RfVault = Join-Path $Work 'Cuentas.kdbx'
$script:Vault = $RfVault
$script:Master = $RfMaster
$script:RootName = 'Cuentas'
$script:DocsDir = [Environment]::GetFolderPath('MyDocuments')
$script:FamDir = Join-Path $DocsDir 'AlmarioKS'
$script:FamVault = Join-Path $FamDir 'Familia.kdbx'
$script:FamKey = Join-Path $FamDir 'Familia.keyx'
function Rf-Secrets {
    $plain = @{ 'titulo' = $Rf.Title; 'usuario' = $Rf.User; 'URL' = $Rf.Url; 'notas' = $Rf.Notes; 'grupo' = $Rf.Group
        'contrasena de la entrada' = $Rf.Pass; 'borrador sin guardar' = $Rf.Draft; 'prefijo FAROPX' = 'FAROPX'
        'dato de muestra (usuario)' = 'ana.casa@example.com'; 'dato de muestra (notas)' = 'Renovar el token de acceso' }
    $k = 0
    foreach ($g in @((Get-State 'generadas') -split "`n" | Where-Object { $_ })) { $k++; $plain["contrasena generada $k"] = $g }
    New-Secrets $plain @{ 'maestra' = $RfMaster; 'maestra de Familia' = $FamMaster; 'repeticion distinta' = $RfOther }
}
function Add-Generated([string]$value) {
    if (-not $value) { return }
    $all = @((Get-State 'generadas') -split "`n" | Where-Object { $_ })
    if ($all -notcontains $value) { Set-State 'generadas' ((@($all) + $value) -join "`n") }
}
function Purge-Mine { History-Purge (@('FAROPX', 'centinela-vn', 'centinela-px', 'almarioks-px', 'PX-Maestra', 'PX-Familia') + @((Get-State 'generadas') -split "`n" | Where-Object { $_ })) }

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
namespace PX {
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
  public static class Win {
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  }
}
"@

# ---------- Ventana y capturas ----------
function Client-Origin {
    $p = New-Object PX.POINT
    [void][PX.Win]::ClientToScreen((Get-AppHwnd), [ref]$p)
    return $p
}
# Solo el area interior de la ventana (sin barra de titulo), en pixeles nativos. Con el puntero fuera.
function Shot-Client([string]$name, [switch]$KeepPointer) {
    $h = Focus-App
    if (-not $KeepPointer) { Pointer-Away }
    Start-Sleep -Milliseconds 450
    $i = Get-WinInfo; $o = Client-Origin
    # Nunca se captura fuera del area util: si la ventana no cabe (1280 x 800 en una pantalla de 1080 px al 125 %),
    # el pie queda bajo la barra de tareas y la captura se recorta ahi, sin llevarse el escritorio.
    $wa = [System.Windows.Forms.Screen]::FromHandle($i.Hwnd).WorkingArea
    $w = [math]::Min($i.ClientW, $wa.Right - $o.X); $h = [math]::Min($i.ClientH, $wa.Bottom - $o.Y)
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size); $g.Dispose()
    $bmp.Save((Join-Path $Shots "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Log "   captura (interior): $name.png ($w x $h fisicos de $($i.ClientW) x $($i.ClientH) = $($i.LogicalW) x $($i.LogicalH) logicos a escala $($i.Scale))$(if ($h -lt $i.ClientH) { " RECORTADA: $($i.ClientH - $h) px fisicos del pie quedan fuera del area util" })"
}
# Pone el interior en WxH logicos y comprueba que la ventana cabe entera en el area util.
function Size-Window([int]$w, [int]$h) {
    [void](Focus-App)
    $i = Set-WinClient $w $h
    $wa = [System.Windows.Forms.Screen]::FromHandle($i.Hwnd).WorkingArea
    $r = [QA.Native]::VisibleRect($i.Hwnd)
    if ($r.Bottom -gt $wa.Bottom -or $r.Right -gt $wa.Right) {
        $nx = [math]::Max($wa.Left, [math]::Min($i.X, $wa.Right - ($r.Right - $r.Left) - 8)); $ny = [math]::Max($wa.Top, $wa.Bottom - ($r.Bottom - $r.Top) - 4)
        [void][QA.Native]::SetWindowPos($i.Hwnd, [IntPtr]::Zero, $nx, $ny, 0, 0, 0x0005); Start-Sleep -Milliseconds 400
        $i = Get-WinInfo
    }
    $inside = (Where-Window "$w x $h").Inside
    Log "VENTANA: interior $($i.ClientW) x $($i.ClientH) fisicos = $($i.LogicalW) x $($i.LogicalH) logicos; escala $($i.Scale); cabe en el area util = $inside"
    return ($i.LogicalW -eq $w -and $i.LogicalH -eq $h -and $inside)
}
# Rectangulo de un control en px logicos relativos al interior de la ventana.
function LRect($el) {
    $r = $el.Current.BoundingRectangle; $i = Get-WinInfo; $o = Client-Origin
    if ($r.IsEmpty) { return $null }
    [pscustomobject]@{ X = [math]::Round(($r.X - $o.X) / $i.Scale, 1); Y = [math]::Round(($r.Y - $o.Y) / $i.Scale, 1); W = [math]::Round($r.Width / $i.Scale, 1); H = [math]::Round($r.Height / $i.Scale, 1)
        R = [math]::Round(($r.X + $r.Width - $o.X) / $i.Scale, 1); B = [math]::Round(($r.Y + $r.Height - $o.Y) / $i.Scale, 1) }
}
function LRect-Text($el) { $l = LRect $el; if ($null -eq $l) { return '(sin rectangulo)' }; return "x=$($l.X) y=$($l.Y) w=$($l.W) h=$($l.H)" }
# Volcado con rectangulos logicos. Nunca lee el valor de un campo de contrasena. -All: tambien lo que no tiene nombre.
function Dump-Rects([switch]$All, [string]$file = '') {
    $i = Get-WinInfo; $o = Client-Origin
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($el in Get-All) {
        $c = $el.Current
        if (-not $All -and -not $c.Name -and -not $c.AutomationId) { continue }
        $r = $c.BoundingRectangle
        $rect = if ($r.IsEmpty) { '-' } else { "{0},{1} {2}x{3}" -f [math]::Round(($r.X - $o.X) / $i.Scale), [math]::Round(($r.Y - $o.Y) / $i.Scale), [math]::Round($r.Width / $i.Scale, 1), [math]::Round($r.Height / $i.Scale, 1) }
        $flags = ''
        if ($c.IsOffscreen) { $flags += ' (fuera)' }
        if (-not $c.IsEnabled) { $flags += ' (inactivo)' }
        if ($c.HasKeyboardFocus) { $flags += ' (FOCO)' }
        if ($c.IsPassword) { $flags += ' (contrasena)' }
        $val = Get-Value $el
        $help = $c.HelpText
        $lines.Add((Redact ("[{0}] `"{1}`"{2}{3}{4} @ {5}{6}" -f (Type-Of $el), $c.Name, $(if ($c.AutomationId) { " id=$($c.AutomationId)" } else { '' }),
                    $(if ($null -ne $val -and $val -ne '') { " valor=`"$val`"" } else { '' }), $(if ($help) { ' ayuda=si' } else { '' }), $rect, $flags)))
    }
    if ($file) { [IO.File]::WriteAllLines((Join-Path $Shots $file), $lines, [Text.Encoding]::UTF8) }
    return $lines
}
function Print-Rects([switch]$All) { foreach ($l in (Dump-Rects -All:$All)) { [Console]::Out.WriteLine("    $l") } }
function Has-Name([string]$like) { return @(Get-Names | Where-Object { $_ -like $like }).Count -gt 0 }
