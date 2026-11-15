# Biblioteca de la QA en la ventana real de AlmarioKS (T28, spec 001).
# Maneja la ventana como una persona: clic real de raton y tecleo real (SendInput Unicode). Lo que cae a
# InvokePattern o ValuePattern queda anotado como "ALTERNATIVA", porque es candidato a hallazgo.
# Solo datos falsos. Las rutas salen de variables de entorno: nada del PC queda escrito aqui.

Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
namespace QA {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct KEYBDINPUT { public ushort wVk; public ushort wScan; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Sequential)] public struct MOUSEINPUT { public int dx; public int dy; public uint mouseData; public uint dwFlags; public uint time; public IntPtr dwExtraInfo; }
  [StructLayout(LayoutKind.Explicit)] public struct INPUTUNION { [FieldOffset(0)] public MOUSEINPUT mi; [FieldOffset(0)] public KEYBDINPUT ki; }
  [StructLayout(LayoutKind.Sequential)] public struct INPUT { public uint type; public INPUTUNION u; }
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  public static class Native {
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT r, int size);
    public static RECT VisibleRect(IntPtr h) {
      RECT r; if (DwmGetWindowAttribute(h, 9, out r, Marshal.SizeOf(typeof(RECT))) != 0) GetWindowRect(h, out r);
      return r;
    }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll", SetLastError = true)] public static extern uint SendInput(uint n, INPUT[] inputs, int size);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetClipboardSequenceNumber();
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder sb, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder sb, int n);

    public static void TypeUnicode(string s, int delayMs) {
      foreach (char c in s) {
        if (c == '\n') { Key(0x0D); Thread.Sleep(delayMs); continue; }
        INPUT[] pair = new INPUT[2];
        pair[0].type = 1; pair[0].u.ki.wScan = c; pair[0].u.ki.dwFlags = 0x0004;
        pair[1].type = 1; pair[1].u.ki.wScan = c; pair[1].u.ki.dwFlags = 0x0004 | 0x0002;
        SendInput(2, pair, Marshal.SizeOf(typeof(INPUT)));
        Thread.Sleep(delayMs);
      }
    }
    public static void Key(byte vk) { keybd_event(vk, 0, 0, UIntPtr.Zero); Thread.Sleep(15); keybd_event(vk, 0, 2, UIntPtr.Zero); Thread.Sleep(15); }
    public static void Chord(byte mod, byte vk) {
      keybd_event(mod, 0, 0, UIntPtr.Zero); Thread.Sleep(20);
      keybd_event(vk, 0, 0, UIntPtr.Zero); Thread.Sleep(20);
      keybd_event(vk, 0, 2, UIntPtr.Zero); Thread.Sleep(20);
      keybd_event(mod, 0, 2, UIntPtr.Zero); Thread.Sleep(20);
    }
    public static void ClickAt(int x, int y) {
      SetCursorPos(x, y); Thread.Sleep(60);
      mouse_event(0x0002, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(30);
      mouse_event(0x0004, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(30);
    }
    public static List<IntPtr> WindowsOfPid(uint pid) {
      List<IntPtr> found = new List<IntPtr>();
      EnumWindows(delegate(IntPtr h, IntPtr l) {
        uint owner; GetWindowThreadProcessId(h, out owner);
        if (owner == pid && IsWindowVisible(h)) found.Add(h);
        return true;
      }, IntPtr.Zero);
      return found;
    }
    public static string ClassOf(IntPtr h) { StringBuilder sb = new StringBuilder(256); GetClassName(h, sb, 256); return sb.ToString(); }
    public static string TitleOf(IntPtr h) { StringBuilder sb = new StringBuilder(512); GetWindowText(h, sb, 512); return sb.ToString(); }
  }
}
"@

# DPI: antes de cargar UIA y WinForms, para que las coordenadas de UIA, el raton y las capturas sean pixeles fisicos.
if (-not [QA.Native]::SetProcessDpiAwarenessContext([IntPtr](-4))) { [void][QA.Native]::SetProcessDPIAware() }
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Windows.Forms, System.Drawing

# ---------- Rutas (siempre desde variables de entorno) ----------
$script:Repo = (Resolve-Path (Join-Path $QaDir '..\..\..\..')).Path
$script:Shots = Join-Path $Repo '.impeccable\review\qa'
$script:Work = Join-Path $env:LOCALAPPDATA 'Temp\almarioks-qa-20261111'
$script:InstallDir = Join-Path $env:LOCALAPPDATA 'AlmarioKS'
$script:AppExe = Join-Path $InstallDir 'almarioks.exe'
$script:ConfigDir = Join-Path $env:APPDATA 'io.github.kattonavi.almarioks'
$script:DataDir = Join-Path $env:LOCALAPPDATA 'io.github.kattonavi.almarioks'
$script:SettingsFile = Join-Path $ConfigDir 'settings.json'
New-Item -ItemType Directory -Force $Shots | Out-Null

# ---------- Datos falsos ----------
$script:Master = "QA-Maestra-2026-$([char]0x00F1)$([char]0x20AC)"
$script:Canary = @{
    Title = 'CANARIO-QA-Titulo-Correo'
    User  = 'CANARIO-QA-usuario@example.com'
    Url   = 'https://example.com/CANARIO-QA-url'
    Notes = 'CANARIO-QA-notas de prueba'
    Tag   = 'CANARIO-QA-etiqueta'
    Pass2 = "CANARIO-QA-Clave-7391-$([char]0x00F1)$([char]0x20AC)"
}

# ---------- Registro ----------
$script:T0 = [Diagnostics.Stopwatch]::StartNew()
# Escribe directo a la salida estandar, no a la canalizacion: asi una funcion que registra no ensucia lo que devuelve.
function Log([string]$text) { [Console]::Out.WriteLine(("[{0,6:n1}s] {1}" -f $T0.Elapsed.TotalSeconds, (Redact $text))) }
# Quita del texto lo que identifica al PC: carpetas del usuario (forma larga y corta 8.3) y su nombre.
function Redact([string]$text) {
    if ($null -eq $text) { return '' }
    $out = $text
    $pairs = @(
        @($env:LOCALAPPDATA, '%LOCALAPPDATA%'), @($env:APPDATA, '%APPDATA%'), @($env:TEMP, '%TEMP%'),
        @($env:USERPROFILE, '%USERPROFILE%'), @($Repo, '<repo>')
    )
    foreach ($pair in $pairs) { if ($pair[0]) { $out = $out -ireplace [regex]::Escape($pair[0]), $pair[1] } }
    $out = $out -ireplace '[A-Z]:\\Users\\[^\\"\s]+', '%USERPROFILE%'
    $out = $out -ireplace ('(?<!io\.github\.)' + [regex]::Escape($env:USERNAME)), '<usuario>'
    return $out
}
$script:Results = New-Object System.Collections.Generic.List[string]
function Check([string]$what, [bool]$ok, [string]$detail = '') {
    $line = ("{0} | {1}{2}" -f $(if ($ok) { 'PASA ' } else { 'FALLA' }), $what, $(if ($detail) { " | $detail" } else { '' }))
    $Results.Add($line); Log ">> $line"
}
# Un subpaso: si revienta, deja captura y nombres visibles y sigue con el siguiente.
function Step([string]$name, [scriptblock]$body, [switch]$Continue) {
    Log "=== $name"
    try { & $body } catch {
        Check "$name (excepcion)" $false "$($_.Exception.Message) @ $($_.InvocationInfo.ScriptLineNumber)"
        try { Shot ("fallo-" + ($name -replace '[^A-Za-z0-9]+', '-')) } catch { }
        try { Show-Names } catch { }
        if (-not $Continue) { Summary; exit 2 }   # por defecto un subpaso roto detiene el paso: lo siguiente dependia de el
    }
}

# ---------- Proceso y ventana ----------
function Get-AppProc {
    @(Get-Process almarioks -ErrorAction SilentlyContinue | Where-Object { $_.Path -and $_.Path -ieq $AppExe })
}
function Get-AppHwnd {
    foreach ($proc in Get-AppProc) {
        foreach ($h in [QA.Native]::WindowsOfPid([uint32]$proc.Id)) {
            # La ventana de Tauri; la consola de la compilacion de depuracion y los dialogos nativos son otras clases.
            if ([QA.Native]::TitleOf($h) -eq 'AlmarioKS' -and [QA.Native]::ClassOf($h) -notin @('ConsoleWindowClass', '#32770')) { return $h }
        }
    }
    return [IntPtr]::Zero
}
function Start-App([int]$timeoutSec = 30) {
    New-Item -ItemType Directory -Force $Work | Out-Null
    $stamp = Get-Date -Format 'HHmmss'
    $out = Join-Path $Work "app-$stamp.out.log"; $err = Join-Path $Work "app-$stamp.err.log"
    # Por ShellExecute y con cmd de intermediario: la app no hereda los descriptores de este script (si los hereda,
    # quien lea la salida del script se queda esperando a que la app cierre) y su salida va a dos archivos.
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $env:ComSpec
    $psi.Arguments = "/d /c `"`"$AppExe`" > `"$out`" 2> `"$err`"`""
    $psi.WorkingDirectory = $InstallDir
    $psi.UseShellExecute = $true
    $psi.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    $sw = [Diagnostics.Stopwatch]::StartNew()
    [void][System.Diagnostics.Process]::Start($psi)
    while ($sw.Elapsed.TotalSeconds -lt $timeoutSec) {
        if ((Get-AppHwnd) -ne [IntPtr]::Zero) { break }
        Start-Sleep -Milliseconds 50
    }
    $script:LastStartMs = [int]$sw.ElapsedMilliseconds
    Log "App lanzada; ventana visible a los $LastStartMs ms"
}
function Close-App([int]$timeoutSec = 15) {
    $h = Get-AppHwnd
    if ($h -eq [IntPtr]::Zero) { Log 'Close-App: no hay ventana'; return $true }
    [void][QA.Native]::PostMessage($h, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)   # WM_CLOSE: cierre normal
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $timeoutSec) {
        if ((Get-AppProc).Count -eq 0) { Log "App cerrada en $([int]$sw.ElapsedMilliseconds) ms"; return $true }
        Start-Sleep -Milliseconds 100
    }
    Log 'Close-App: el proceso sigue vivo'; return $false
}
function Focus-App {
    $h = Get-AppHwnd
    if ($h -eq [IntPtr]::Zero) { throw 'La ventana de AlmarioKS no existe' }
    if ([QA.Native]::IsIconic($h)) { [void][QA.Native]::ShowWindow($h, 9) }
    if ([QA.Native]::GetForegroundWindow() -ne $h) {
        [void][QA.Native]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0003)
        [void][QA.Native]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0003)
        [void][QA.Native]::SetForegroundWindow($h)
        Start-Sleep -Milliseconds 150
    }
    return $h
}
function Get-WinInfo {
    $h = Get-AppHwnd
    $w = New-Object QA.RECT; $c = New-Object QA.RECT
    [void][QA.Native]::GetWindowRect($h, [ref]$w); [void][QA.Native]::GetClientRect($h, [ref]$c)
    $scale = [QA.Native]::GetDpiForWindow($h) / 96.0
    [pscustomobject]@{
        Hwnd = $h; X = $w.Left; Y = $w.Top; OuterW = $w.Right - $w.Left; OuterH = $w.Bottom - $w.Top
        ClientW = $c.Right; ClientH = $c.Bottom; Scale = $scale
        LogicalW = [math]::Round($c.Right / $scale); LogicalH = [math]::Round($c.Bottom / $scale)
        Maximized = [QA.Native]::IsZoomed($h)
    }
}
# Tamano interior en px logicos (como los de tauri.conf.json).
function Set-WinClient([int]$logicalW, [int]$logicalH) {
    $i = Get-WinInfo
    if ($i.Maximized) { [void][QA.Native]::ShowWindow($i.Hwnd, 9); Start-Sleep -Milliseconds 300; $i = Get-WinInfo }
    $frameW = $i.OuterW - $i.ClientW; $frameH = $i.OuterH - $i.ClientH
    $w = [int]($logicalW * $i.Scale) + $frameW; $h = [int]($logicalH * $i.Scale) + $frameH
    [void][QA.Native]::SetWindowPos($i.Hwnd, [IntPtr]::Zero, 60, 40, $w, $h, 0x0004)
    Start-Sleep -Milliseconds 500
    return Get-WinInfo
}

# ---------- UIA ----------
$script:AE = [System.Windows.Automation.AutomationElement]
$script:TreeScope = [System.Windows.Automation.TreeScope]
function Get-Root {
    $h = Get-AppHwnd
    if ($h -eq [IntPtr]::Zero) { throw 'La ventana de AlmarioKS no existe' }
    return $AE::FromHandle($h)   # siempre nueva: el bloqueo recarga la webview y las referencias viejas mueren
}
function Type-Of($el) { $el.Current.ControlType.ProgrammaticName -replace '^ControlType\.', '' }
function Get-All($root = $null) {
    if ($null -eq $root) { $root = Get-Root }
    return $root.FindAll($TreeScope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
}
# Busca por nombre exacto (-Name), comodin (-Like) o id del HTML (-Id), y tipo de control opcional. Espera hasta -Timeout s.
function Find-El {
    param([string]$Name, [string]$Like, [string]$Id, [string]$Type, [double]$Timeout = 6, $Root = $null, [switch]$Quiet, [int]$Index = 0)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        try {
            $base = if ($null -ne $Root) { $Root } else { Get-Root }
            $conds = New-Object System.Collections.Generic.List[System.Windows.Automation.Condition]
            if ($Name) { $conds.Add((New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, $Name))) }
            if ($Id) { $conds.Add((New-Object System.Windows.Automation.PropertyCondition($AE::AutomationIdProperty, $Id))) }
            if ($Type) {
                $ct = [System.Windows.Automation.ControlType].GetField($Type).GetValue($null)
                $conds.Add((New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $ct)))
            }
            $cond = if ($conds.Count -eq 0) { [System.Windows.Automation.Condition]::TrueCondition }
            elseif ($conds.Count -eq 1) { $conds[0] }
            else { New-Object System.Windows.Automation.AndCondition($conds.ToArray()) }
            $found = @($base.FindAll($TreeScope::Descendants, $cond))
            if ($Like) { $found = @($found | Where-Object { $_.Current.Name -like $Like }) }
            if ($found.Count -gt $Index) { return $found[$Index] }
        } catch { }
        Start-Sleep -Milliseconds 80
    } while ($sw.Elapsed.TotalSeconds -lt $Timeout)
    if ($Quiet) { return $null }
    throw "No aparecio el control (Name='$Name' Like='$Like' Id='$Id' Type='$Type') en $Timeout s"
}
function Has-El { param([string]$Name, [string]$Like, [string]$Id, [string]$Type, [double]$Timeout = 0.3)
    return $null -ne (Find-El -Name $Name -Like $Like -Id $Id -Type $Type -Timeout $Timeout -Quiet)
}
# Espera a que aparezca un texto (comodin) en cualquier nombre de la ventana. Devuelve los ms, o -1.
function Wait-Text([string]$like, [double]$timeout = 10) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        try { foreach ($el in Get-All) { if ($el.Current.Name -like $like) { return [int]$sw.ElapsedMilliseconds } } } catch { }
        Start-Sleep -Milliseconds 40
    } while ($sw.Elapsed.TotalSeconds -lt $timeout)
    return -1
}
function Wait-Gone([string]$like, [double]$timeout = 10) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        $still = $false
        try { foreach ($el in Get-All) { if ($el.Current.Name -like $like) { $still = $true; break } } } catch { $still = $true }
        if (-not $still) { return [int]$sw.ElapsedMilliseconds }
        Start-Sleep -Milliseconds 40
    } while ($sw.Elapsed.TotalSeconds -lt $timeout)
    return -1
}
function Get-Names($root = $null) {
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($el in Get-All $root) { $n = $el.Current.Name; if ($n) { $names.Add($n) } }
    return $names
}
function Get-Value($el) {
    if ($el.Current.IsPassword) { return '<campo de contrasena: no se lee>' }
    $p = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$p)) { return $p.Current.Value }
    return $null
}
# Volcado del arbol: tipo, nombre, id, valor y banderas. Nunca lee el valor de un campo de contrasena.
function Dump-Tree([string]$file = '', $root = $null) {
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($el in Get-All $root) {
        $c = $el.Current
        $flags = ''
        if ($c.IsOffscreen) { $flags += ' (fuera)' }
        if (-not $c.IsEnabled) { $flags += ' (inactivo)' }
        if ($c.HasKeyboardFocus) { $flags += ' (FOCO)' }
        if ($c.IsPassword) { $flags += ' (contrasena)' }
        $val = Get-Value $el
        $line = "[{0}] `"{1}`"{2}{3}{4}" -f (Type-Of $el), $c.Name, $(if ($c.AutomationId) { " id=$($c.AutomationId)" } else { '' }),
            $(if ($null -ne $val -and $val -ne '') { " valor=`"$val`"" } else { '' }), $flags
        $lines.Add((Redact $line))
    }
    if ($file) { [IO.File]::WriteAllLines((Join-Path $Shots $file), $lines, [Text.Encoding]::UTF8) }
    return $lines
}
# Version corta para la consola: solo lo que tiene nombre, sin repetir.
function Show-Names([int]$max = 120) {
    $seen = @{}; $n = 0
    foreach ($line in Dump-Tree) {
        if ($line -match '^\[\w+\] ""( |$)' -and $line -notmatch 'id=|valor=') { continue }
        if ($seen.ContainsKey($line)) { continue }
        $seen[$line] = 1; [Console]::Out.WriteLine("    $line"); $n++
        if ($n -ge $max) { [Console]::Out.WriteLine('    ...'); break }
    }
}
function Focused-Desc {
    try {
        $f = $AE::FocusedElement
        return ("[{0}] `"{1}`"{2}" -f (Type-Of $f), $f.Current.Name, $(if ($f.Current.AutomationId) { " id=$($f.Current.AutomationId)" } else { '' }))
    } catch { return '<sin foco legible>' }
}

# ---------- Entrada real ----------
Add-Type -AssemblyName WindowsBase
# ¿Lo que hay en el punto (x, y) es el control o un descendiente suyo?
function Test-Hit($el, [int]$x, [int]$y) {
    try {
        $cur = $AE::FromPoint((New-Object System.Windows.Point($x, $y)))
        for ($k = 0; $k -lt 6 -and $null -ne $cur; $k++) {
            if ([System.Windows.Automation.Automation]::Compare($cur, $el)) { return $true }
            $cur = [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($cur)
        }
        return $false
    } catch { return $true }   # si la prueba no se puede hacer, no bloquea el clic
}
function Click-El($el, [string]$label = '') {
    [void](Focus-App)
    $c = $el.Current
    if (-not $label) { $label = "[$(Type-Of $el)] `"$($c.Name)`"" }
    $r = $c.BoundingRectangle
    $usable = (-not $c.IsOffscreen) -and (-not $r.IsEmpty) -and $r.Width -gt 1 -and $r.Height -gt 1
    if ($usable) {
        $i = Get-WinInfo
        $x = [int]($r.X + $r.Width / 2); $y = [int]($r.Y + $r.Height / 2)
        if ($x -lt $i.X -or $x -gt ($i.X + $i.OuterW) -or $y -lt $i.Y -or $y -gt ($i.Y + $i.OuterH)) { $usable = $false }
    }
    # UIA puede dar por visible un control recortado por su panel o tapado por otro: se comprueba que lo que hay
    # bajo el puntero es el control (o algo dentro de el) antes de hacer clic.
    if ($usable -and (Test-Hit $el $x $y)) {
        [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 120
        return
    }
    $why = if ($usable) { 'TAPADO O RECORTADO: bajo su centro hay otra cosa' } else { 'fuera de la vista' }
    # Se trae a la vista con ScrollItem (lo que haria la rueda del raton) y, si no, se invoca por UIA (anotado).
    $sp = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.ScrollItemPattern]::Pattern, [ref]$sp)) {
        $sp.ScrollIntoView(); Start-Sleep -Milliseconds 350
        $r = $el.Current.BoundingRectangle
        if (-not $el.Current.IsOffscreen -and -not $r.IsEmpty) {
            $x = [int]($r.X + $r.Width / 2); $y = [int]($r.Y + $r.Height / 2)
            $hit = Test-Hit $el $x $y
            [QA.Native]::ClickAt($x, $y); Start-Sleep -Milliseconds 120
            Log "   ($why; desplazado a la vista antes del clic$(if (-not $hit) { ', y AUN ASI bajo el puntero no esta el control' }): $label)"
            return
        }
    }
    $ip = $null
    if ($el.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern, [ref]$ip)) {
        Log "   ALTERNATIVA: Invoke por UIA, el control no estaba a la vista: $label"
        $ip.Invoke(); Start-Sleep -Milliseconds 120; return
    }
    throw "No se pudo hacer clic en $label (fuera de la vista y sin Invoke)"
}
function Click-Name { param([string]$Name, [string]$Like, [string]$Id, [string]$Type = 'Button', [double]$Timeout = 6, [int]$Index = 0)
    $el = Find-El -Name $Name -Like $Like -Id $Id -Type $Type -Timeout $Timeout -Index $Index
    Click-El $el
}
function Type-Text([string]$text, [switch]$Secret) {
    [QA.Native]::TypeUnicode($text, 6)
    if ($Secret) { Log "   tecleado: <secreto, $($text.Length) caracteres>" } else { Log "   tecleado: $text" }
    Start-Sleep -Milliseconds 80
}
$script:VK = @{ TAB = 0x09; ENTER = 0x0D; ESC = 0x1B; SPACE = 0x20; END = 0x23; HOME = 0x24; LEFT = 0x25; UP = 0x26; RIGHT = 0x27; DOWN = 0x28; DEL = 0x2E; BACK = 0x08; F6 = 0x75 }
function Press([string]$key, [int]$times = 1) { for ($k = 0; $k -lt $times; $k++) { [QA.Native]::Key([byte]$VK[$key]); Start-Sleep -Milliseconds 60 } }
function Press-Ctrl([char]$letter) { [QA.Native]::Chord(0x11, [byte][char]::ToUpper($letter)); Start-Sleep -Milliseconds 80 }
function Press-ShiftTab { [QA.Native]::Chord(0x10, 0x09); Start-Sleep -Milliseconds 60 }
# Clic en el campo, seleccionar todo y teclear encima.
function Fill { param($el, [string]$text, [switch]$Secret, [switch]$NoClear)
    Click-El $el
    if (-not $NoClear) { Press-Ctrl 'a'; Press 'DEL' }
    if ($text -ne '') { Type-Text $text -Secret:$Secret }
}
function Fill-Id { param([string]$Id, [string]$text, [switch]$Secret)
    Fill (Find-El -Id $Id) $text -Secret:$Secret
}

# ---------- Capturas ----------
# Captura de pantalla del rectangulo de la ventana, con la ventana traida al frente. PrintWindow no sirve aqui:
# con WebView2 devuelve el area web en blanco (probado: A01 salio blanca con el arbol UIA completo).
function Shot([string]$name, [IntPtr]$hwnd = [IntPtr]::Zero) {
    if ($hwnd -eq [IntPtr]::Zero) { $hwnd = Focus-App; Start-Sleep -Milliseconds 200 }
    if ($hwnd -eq [IntPtr]::Zero) { Log "   (sin ventana para capturar $name)"; return }
    $r = [QA.Native]::VisibleRect($hwnd)   # sin el borde invisible de redimensionar: ahi se colaria lo que hay detras
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { return }
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size); $g.Dispose()
    $bmp.Save((Join-Path $Shots "$name.png"), [System.Drawing.Imaging.ImageFormat]::Png); $bmp.Dispose()
    Log "   captura: $name.png ($w x $h)"
}

# ---------- Portapapeles ----------
function Get-Clip {
    for ($try = 0; $try -lt 5; $try++) {
        try {
            if ([System.Windows.Forms.Clipboard]::ContainsText()) { return [System.Windows.Forms.Clipboard]::GetText() }
            return ''
        } catch { Start-Sleep -Milliseconds 60 }
    }
    return $null
}
function Get-ClipFormats {
    try { return @([System.Windows.Forms.Clipboard]::GetDataObject().GetFormats()) } catch { return @() }
}
function Get-ClipSeq { [QA.Native]::GetClipboardSequenceNumber() }
function Set-ClipExternal([string]$text) { [System.Windows.Forms.Clipboard]::SetText($text) }   # "otra app" = este proceso

# ---------- Dialogos nativos de archivo ----------
function Find-FileDialog([double]$timeout = 8) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        foreach ($proc in Get-AppProc) {
            foreach ($h in [QA.Native]::WindowsOfPid([uint32]$proc.Id)) {
                if ([QA.Native]::ClassOf($h) -eq '#32770') { return $h }
            }
        }
        Start-Sleep -Milliseconds 100
    } while ($sw.Elapsed.TotalSeconds -lt $timeout)
    return [IntPtr]::Zero
}
# Escribe la ruta en el campo "Nombre" y acepta. -Replace: responde "Si" si Windows pregunta por reemplazar.
function Answer-FileDialog { param([string]$path, [switch]$Replace, [string]$shot = '')
    $h = Find-FileDialog
    if ($h -eq [IntPtr]::Zero) { throw 'No aparecio el dialogo nativo de archivo' }
    Start-Sleep -Milliseconds 500
    $dlg = $AE::FromHandle($h)
    Log "   dialogo nativo: `"$($dlg.Current.Name)`""
    # El campo "Nombre de archivo" tiene el foco al abrir. Se espera a que el dialogo termine de armarse y se
    # escribe ahi: por ValuePattern si ya se expone como Edit, y si no con el teclado, como una persona.
    $edit = $null
    foreach ($id in @('1001', '1148')) {
        $edit = Find-El -Id $id -Type 'Edit' -Root $dlg -Timeout 2.5 -Quiet
        if ($null -ne $edit) { break }
    }
    [void][QA.Native]::SetForegroundWindow($h)
    if ($null -ne $edit) {
        $edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($path)
    } else {
        Log '   dialogo nativo: el campo no se expone como Edit; se teclea la ruta en el campo enfocado'
        Start-Sleep -Milliseconds 800
        [QA.Native]::Chord(0x12, [byte][char]'N'); Start-Sleep -Milliseconds 150   # Alt+N: "Nombre de archivo"
        Press-Ctrl 'a'
        [QA.Native]::TypeUnicode($path, 4)
    }
    Start-Sleep -Milliseconds 300
    if ($shot) { Shot $shot $h }
    Press 'ENTER'
    Start-Sleep -Milliseconds 600
    # Posible pregunta "ya existe, ¿reemplazar?"
    $h2 = Find-FileDialog 0.6
    if ($h2 -ne [IntPtr]::Zero) {
        $dlg2 = $AE::FromHandle($h2)
        $yes = Find-El -Id 'CommandButton_6' -Root $dlg2 -Timeout 1.0 -Quiet
        if ($null -eq $yes) { $yes = Find-El -Name ("S" + [char]0x00ED) -Type 'Button' -Root $dlg2 -Timeout 0.8 -Quiet }
        if ($null -ne $yes) {
            $texts = (Get-Names $dlg2) -join ' / '
            Log "   Windows pregunta: $texts"
            if ($Replace) {
                # El boton "Si" de este aviso no admite Invoke por UIA: se responde con el teclado (Alt+S).
                [void][QA.Native]::SetForegroundWindow($h2); Start-Sleep -Milliseconds 200
                [QA.Native]::Chord(0x12, [byte][char]'S'); Log '   respondido: Si (Alt+S)'
            }
            else { throw 'Windows pregunto por reemplazar y el paso no lo esperaba' }
            Start-Sleep -Milliseconds 500
        }
    }
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ((Find-FileDialog 0.2) -ne [IntPtr]::Zero -and $sw.Elapsed.TotalSeconds -lt 5) { Start-Sleep -Milliseconds 100 }
    if ((Find-FileDialog 0.2) -ne [IntPtr]::Zero) { throw 'El dialogo nativo no se cerro' }
}

# ---------- Red ----------
function Get-AppPids {
    $ids = New-Object System.Collections.Generic.List[int]
    foreach ($p in Get-AppProc) { $ids.Add($p.Id) }
    $all = Get-CimInstance Win32_Process -Property ProcessId, ParentProcessId, Name
    $grew = $true
    while ($grew) {
        $grew = $false
        foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId) -and -not $ids.Contains([int]$p.ProcessId)) { $ids.Add([int]$p.ProcessId); $grew = $true } }
    }
    return $ids
}
# Conexiones TCP del proceso de la app y de sus hijos (WebView2) que no son de la propia maquina.
function Show-Net([string]$when) {
    $ids = Get-AppPids
    $conns = @(Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $ids -contains $_.OwningProcess })
    $remote = @($conns | Where-Object { $_.RemoteAddress -notin @('127.0.0.1', '::1', '0.0.0.0', '::') -and $_.State -ne 'Listen' })
    Log "RED [$when]: procesos=$($ids.Count) conexiones TCP=$($conns.Count) no locales=$($remote.Count)"
    foreach ($c in $remote) {
        $name = (Get-Process -Id $c.OwningProcess -ErrorAction SilentlyContinue).Name
        Log "   $name -> $($c.RemoteAddress):$($c.RemotePort) ($($c.State))"
    }
}
function Sha([string]$path) { (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.Substring(0, 16) }
function Summary { Log '--- resumen del paso ---'; foreach ($line in $Results) { [Console]::Out.WriteLine("  $line") } }
