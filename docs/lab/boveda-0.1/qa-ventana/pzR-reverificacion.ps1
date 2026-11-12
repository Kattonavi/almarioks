# Ronda 2, reverificacion con el build final: (A) boveda de prueba pequena en la carpeta temporal propia, sin archivo
# llave, con una entrada; (B) teclear en la casilla de longitud ya no genera: solo "Generar", Enter en la casilla o el
# deslizante; (C) "Copiar contraseña" en la ventana de la entrada y Esc de inmediato: el aviso sale en la lista.
# Limpieza despues con pz9-limpieza.ps1. RF_VARIANT=pz-lib.
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'rf-lib.ps1'), [Text.Encoding]::UTF8)))
$script:Vault = Join-Path $Pz 'Cuentas.kdbx'; $script:RfVault = $Vault
$ExactId = 'entry-dialog-password-length-exact'
$Canary = $PyPass['Jira']
Add-Type -TypeDefinition @"
using System; using System.Runtime.InteropServices; using System.Threading;
namespace PZR { public static class Fast {
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr e);
  // Un clic y Escape pegado, sin nada en medio.
  public static void ClickEsc(int x, int y) {
    SetCursorPos(x, y); Thread.Sleep(40);
    mouse_event(2, 0, 0, 0, UIntPtr.Zero); Thread.Sleep(12); mouse_event(4, 0, 0, 0, UIntPtr.Zero);
    keybd_event(0x1B, 0, 0, UIntPtr.Zero); Thread.Sleep(10); keybd_event(0x1B, 0, 2, UIntPtr.Zero);
  } } }
"@
function Field { $pw = Find-El -Id 'entry-dialog-password' -Timeout 3; [pscustomobject]@{ V = "$(Raw-Value $pw)"; Masked = $pw.Current.IsPassword } }
function Box { "$(Get-Value (Find-El -Id $ExactId))" }
function Msg { (Names-Like '^Entre 8 y 500') -join '|' }
function Gen { Click-El (Find-El -Name 'Generar' -Type Button); Start-Sleep -Milliseconds 800 }
# Vacia la casilla y teclea los digitos de uno en uno, leyendo el campo de contrasena tras cada tecla.
function Type-Box([string]$digits, [string]$base) {
    Click-El (Find-El -Id $ExactId); Press-Ctrl 'a'; Press 'DEL'; Start-Sleep -Milliseconds 450
    $now = (Field).V; $same = ($now -ceq $base)
    $trace = @("vacia: campo igual = $($now -ceq $base), aviso `"$(Msg)`"")
    foreach ($ch in $digits.ToCharArray()) {
        [QA.Native]::TypeUnicode([string]$ch, 6); Start-Sleep -Milliseconds 450
        $now = (Field).V
        if ($now -cne $base) { $same = $false; Add-Generated $now }
        $trace += "`"$(Box)`": campo igual = $($now -ceq $base) ($($now.Length) caracteres), aviso `"$(Msg)`""
    }
    return [pscustomobject]@{ Same = $same; Trace = ($trace -join ' -> ') }
}

Step 'PZR1 (A) Boveda de prueba y una entrada' {
    New-Item -ItemType Directory -Force $Pz | Out-Null
    if (Test-Path -LiteralPath $Vault) { throw 'La boveda de prueba ya existe' }
    [void](To-Welcome); Check 'Ventana a 1120 x 720' (Size-Window 1120 720)
    Click-Name -Name 'Crear bóveda nueva'; [void](Find-El -Id 'create-title' -Timeout 6); Start-Sleep -Milliseconds 500
    $r = Radios
    Fill-Id 'create-name' 'Cuentas'
    Pick-Dialog 'Cambiar ubicación…' $Vault
    Fill-Id 'create-password' $RfMaster -Secret
    Fill-Id 'create-confirm' $RfMaster -Secret
    Click-Name -Name 'Crear bóveda' -Type Button
    $ok = $null -ne (Find-El -Name 'Bloquear' -Type Button -Timeout 45 -Quiet); Start-Sleep -Milliseconds 800
    Check '(A) Boveda de prueba creada en la carpeta temporal, sin archivo llave' ($ok -and (Test-Path -LiteralPath $Vault) -and @(Get-ChildItem -LiteralPath $Pz -Filter '*.keyx').Count -eq 0) "opciones: $($r -join ' | '); archivos: $((Get-ChildItem -LiteralPath $Pz | ForEach-Object { $_.Name }) -join ', ')"
    $e = New-EntryPy 'Jira' 'ana.g' '' '' $Canary
    Check '(A) Una entrada creada' ($e.Closed -and $e.Disk -and (Has-Entry 'Jira')) "cerrado=$($e.Closed) disco=$($e.Disk); foco: $($e.FocusAfter)"
}
Step 'PZR2 (B) Teclear en la casilla ya no genera' -Continue {
    Click-Name -Name 'Nueva entrada'; [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 400
    Click-El (Find-El -Name 'Opciones del generador' -Type Button -Timeout 3); Start-Sleep -Milliseconds 500
    Gen; $f = Field
    if ($f.Masked) { Click-El (Find-El -Name 'Mostrar contraseña' -Type Button -Root (Find-El -Name 'Nueva entrada' -Type Window)); Start-Sleep -Milliseconds 400; $f = Field }
    $v0 = $f.V; Add-Generated $v0
    Check '(B) "Generar": 20 caracteres a la vista' ($v0.Length -eq 20 -and -not $f.Masked) "longitud $($v0.Length); casilla `"$(Box)`""
    $t1 = Type-Box '501' $v0; $m1 = Msg
    Gen; $a1 = (Field).V; if ($a1 -cne $v0) { Add-Generated $a1 }
    Check '(B) 501 tecleado caracter por caracter: el campo sigue EXACTAMENTE igual (tambien al pasar por 50), sale el aviso y "Generar" no genera' ($t1.Same -and $m1 -ceq 'Entre 8 y 500 caracteres.' -and $a1 -ceq $v0) "$($t1.Trace); tras pulsar Generar: campo igual = $($a1 -ceq $v0) ($($a1.Length) caracteres), aviso `"$(Msg)`""
    $base = $a1
    $t2 = Type-Box '500' $base
    Check '(B) 500 tecleado: el campo sigue igual hasta pulsar "Generar"' $t2.Same $t2.Trace
    Gen; $v500 = (Field).V; Add-Generated $v500
    Check '(B) "Generar" con 500 en la casilla: 500 caracteres' ($v500.Length -eq 500 -and $v500 -cne $base) "longitud $($v500.Length); deslizante = `"$(Get-Value (Find-El -Id 'entry-dialog-password-length'))`"; aviso `"$(Msg)`""
    $t3 = Type-Box '30' $v500
    Press 'ENTER'; Start-Sleep -Milliseconds 900
    $open = Dialog-Open 'Nueva entrada'; $v30 = if ($open) { (Field).V } else { '' }; Add-Generated $v30
    Check '(B) 30 tecleado no cambia el campo; Enter en la casilla genera 30 (y no cierra el dialogo)' ($t3.Same -and $open -and $v30.Length -eq 30) "$($t3.Trace); tras Enter: dialogo abierto = $open, longitud $($v30.Length)"
    $sl = Find-El -Id 'entry-dialog-password-length'; Click-El $sl; Start-Sleep -Milliseconds 900
    $s1 = "$(Get-Value (Find-El -Id 'entry-dialog-password-length'))"; $vs1 = (Field).V; Add-Generated $vs1
    Press 'RIGHT'; Start-Sleep -Milliseconds 900
    $s2 = "$(Get-Value (Find-El -Id 'entry-dialog-password-length'))"; $vs2 = (Field).V; Add-Generated $vs2
    Check '(B) Mover el deslizante regenera con esa longitud' ("$($vs1.Length)" -eq $s1 -and "$($vs2.Length)" -eq $s2 -and $vs1 -cne $v30 -and $vs2 -cne $vs1) "clic en el deslizante: valor $s1, campo de $($vs1.Length) (cambio = $($vs1 -cne $v30)); flecha derecha: valor $s2, campo de $($vs2.Length) (cambio = $($vs2 -cne $vs1)); casilla `"$(Box)`""
    Click-El (Find-El -Name 'Cancelar' -Type Button); Start-Sleep -Milliseconds 600
    $ask = Find-El -Name 'Descartar' -Type Button -Timeout 1.5 -Quiet; if ($null -ne $ask) { Click-El $ask; Start-Sleep -Milliseconds 600 }
}
Step 'PZR3 (C) Copiar en la ventana de la entrada y Esc de inmediato' -Continue {
    if (Dialog-Open 'Nueva entrada') { Press 'ESC'; Start-Sleep -Milliseconds 400; $ask = Find-El -Name 'Descartar' -Type Button -Timeout 1 -Quiet; if ($null -ne $ask) { Click-El $ask; Start-Sleep -Milliseconds 500 } }
    Pick-Entry 'Jira'; Click-Name -Name 'Abrir Jira'
    $w = Entry-Window 'Jira' 4; Start-Sleep -Milliseconds 600
    $c = Mid (Find-El -Name 'Copiar contraseña' -Type Button -Root $w)
    Set-ClipExternal 'centinela-pz-r'; Start-Sleep -Milliseconds 300; $s0 = Get-ClipSeq
    [void](Focus-App); Start-Sleep -Milliseconds 250
    [PZR.Fast]::ClickEsc($c[0], $c[1])
    $sw = [Diagnostics.Stopwatch]::StartNew(); $at = -1; $note = ''; $goneAt = -1
    while ($sw.ElapsedMilliseconds -lt 3500 -and ($at -lt 0 -or $goneAt -lt 0)) {
        if ($goneAt -lt 0 -and $null -eq (Entry-Window 'Jira' 0.05)) { $goneAt = [int]$sw.ElapsedMilliseconds }
        if ($at -lt 0) { $n = @(Names-Like '^Contraseña copiada'); if ($n.Count) { $at = [int]$sw.ElapsedMilliseconds; $note = $n[0] } }
    }
    $cs = Clip-State $Canary
    Check '(C) Copiar y Esc de inmediato: el aviso sale en la lista y el portapapeles tiene el valor por el nucleo' ($at -ge 0 -and $goneAt -ge 0 -and $cs.Exact -and $cs.Formats -match 'ExcludeClipboardContentFromMonitorProcessing' -and $cs.Formats -notmatch 'Chromium') "ventana cerrada (vista a los $goneAt ms); aviso `"$note`" (visto a los $at ms); portapapeles exacto = $($cs.Exact), cambio = $((Get-ClipSeq) -ne $s0), formatos de exclusion $($cs.Excl)/3; foco: $(Focused-Desc)"
    [System.Windows.Forms.Clipboard]::Clear()
}
Summary
