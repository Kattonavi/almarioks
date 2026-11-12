# Ronda 2, puntos 5 y 6: ayudas de los pasos py* con los nombres accesibles de la lista en tabla y de la ventana de la
# entrada (descubiertos por UIA con px0-ver.ps1 sobre la app instalada, no supuestos). Se carga despues de rf-lib.ps1
# con RF_VARIANT=py-lib.
#   Tabla: [Table] > fila [DataItem] > celdas [DataItem]; la celda del titulo lleva un [Button] con el titulo;
#   la de la contrasena se llama "Oculta"; la fila elegida trae "Copiar contraseña de <titulo>" y "Abrir <titulo>".
#   Ventana de la entrada: [Window] con el titulo; "Editar", "Más acciones de la entrada", "Cerrar".

# El boton del titulo de una fila (el punto donde se hace clic o doble clic para elegir o abrir).
function Entry-Row([string]$title, [double]$timeout = 1.5) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    do {
        $b = @(Els-Named $title 'Button')
        if ($b.Count) { return $b[0] }
        Start-Sleep -Milliseconds 120
    } while ($sw.Elapsed.TotalSeconds -lt $timeout)
    return $null
}
function Raw-Parent($el) { [System.Windows.Automation.TreeWalker]::RawViewWalker.GetParent($el) }
function Row-Of([string]$title) {
    $b = Entry-Row $title 3
    if ($null -eq $b) { return $null }
    $cell = Up-To (Raw-Parent $b) 'DataItem'
    return (Up-To (Raw-Parent $cell) 'DataItem')
}
function Header-Of([string]$col) { @(Els-Named $col 'DataItem') | Select-Object -First 1 }
# La celda de una fila bajo el rotulo $col (TÍTULO, USUARIO, CONTRASEÑA, SITIO, MODIFICADA): se casa por la X del rotulo.
function Cell-Of([string]$title, [string]$col) {
    $h = Header-Of $col; if ($null -eq $h) { throw "No hay rotulo $col" }
    $hx = (LRect $h).X
    $row = Row-Of $title; if ($null -eq $row) { throw "No esta la fila de $title" }
    foreach ($k in (Kids $row)) { $r = LRect $k; if ($null -ne $r -and $r.W -gt 10 -and [math]::Abs($r.X - $hx) -lt 2) { return $k } }
    return $null
}
function Clear-Search {
    $s = Find-El -Id 'vault-search' -Timeout 4
    if ("$(Get-Value $s)" -ne '') { Fill $s ''; Start-Sleep -Milliseconds 400 }
}
function Entry-Window([string]$title, [double]$timeout = 0.5) { Find-El -Name $title -Type Window -Timeout $timeout -Quiet }
# Nueva entrada con todo tecleado. Sin $password no se toca el campo (ni "Generar"): entrada sin contrasena.
function New-EntryPy([string]$title, [string]$user, [string]$url, [string]$notes, [string]$password) {
    Click-Name -Name 'Nueva entrada'
    [void](Find-El -Name 'Nueva entrada' -Type Window -Timeout 5); Start-Sleep -Milliseconds 350
    Type-Text $title
    if ($user) { Fill-Id 'entry-dialog-username' $user }
    if ($password) { Fill-Id 'entry-dialog-password' $password -Secret; Start-Sleep -Milliseconds 600 }   # el medidor de fortaleza mueve lo de abajo
    $retry = 0
    if ($url) {
        Fill-Id 'entry-dialog-url' $url
        if ("$(Get-Value (Find-El -Id 'entry-dialog-url'))" -cne $url) { $retry = 1; Start-Sleep -Milliseconds 500; Fill-Id 'entry-dialog-url' $url }
    }
    if ($notes) { Fill-Id 'entry-dialog-notes' $notes }
    $urlNow = "$(Get-Value (Find-El -Id 'entry-dialog-url'))"
    $stamp0 = (Get-Item $Vault).LastWriteTimeUtc
    Click-El (Find-El -Name 'Guardar entrada' -Type Button)
    $sw = [Diagnostics.Stopwatch]::StartNew(); $closed = $false; $disk = $false
    while ($sw.Elapsed.TotalSeconds -lt 20 -and -not ($closed -and $disk)) {
        if (-not $closed -and -not (Dialog-Open 'Nueva entrada')) { $closed = $true }
        if (-not $disk -and (Get-Item $Vault).LastWriteTimeUtc -ne $stamp0) { $disk = $true }
        Start-Sleep -Milliseconds 50
    }
    Start-Sleep -Milliseconds 500
    return [pscustomobject]@{ Closed = $closed; Disk = $disk; UrlTyped = $urlNow; UrlRetry = $retry; FocusAfter = (Focused-Desc) }
}
# Brillo medio (0-255) de un rectangulo logico del interior de la ventana, leido de la pantalla.
function Luma-Of($el, [double]$inset = 6) {
    $r = $el.Current.BoundingRectangle; $i = Get-WinInfo; $p = [int]($inset * $i.Scale)
    $w = [int]$r.Width - 2 * $p; $h = [int]$r.Height - 2 * $p
    $bmp = New-Object System.Drawing.Bitmap($w, $h); $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen([int]$r.X + $p, [int]$r.Y + $p, 0, 0, $bmp.Size); $g.Dispose()
    $sum = 0.0; $n = 0
    for ($y = 0; $y -lt $h; $y += 2) { for ($x = 0; $x -lt $w; $x += 2) { $c = $bmp.GetPixel($x, $y); $sum += 0.299 * $c.R + 0.587 * $c.G + 0.114 * $c.B; $n++ } }
    $bmp.Dispose()
    return [math]::Round($sum / $n)
}
# Elementos con ScrollPattern: ¿se puede desplazar en horizontal o en vertical?
function Scroll-Report {
    $out = @()
    foreach ($el in @(Get-All)) {
        $p = $null
        if ($el.TryGetCurrentPattern([System.Windows.Automation.ScrollPattern]::Pattern, [ref]$p)) {
            $c = $p.Current
            $out += [pscustomobject]@{ What = "[$(Type-Of $el)] `"$($el.Current.Name)`" $(LRect-Text $el)"; H = $c.HorizontallyScrollable; V = $c.VerticallyScrollable; HView = [math]::Round($c.HorizontalViewSize, 1); VView = [math]::Round($c.VerticalViewSize, 1) }
        }
    }
    return $out
}
