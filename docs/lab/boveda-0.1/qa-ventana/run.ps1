# QA en la ventana real (T28, spec 001). Lanzador: carga la biblioteca y un paso leyendolos como UTF-8
# (PowerShell 5.1 lee los .ps1 sin BOM como ANSI y los nombres con tilde no casarian).
# Uso: powershell -NoProfile -ExecutionPolicy Bypass -File run.ps1 <paso.ps1> [argumentos]
# Este archivo va en ASCII a proposito.
param(
    [Parameter(Mandatory = $true)][string]$Step,
    [string[]]$StepArgs = @()
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$QaDir = $PSScriptRoot
$utf8 = [Text.Encoding]::UTF8
. ([scriptblock]::Create([IO.File]::ReadAllText((Join-Path $QaDir 'qa-lib.ps1'), $utf8)))
$stepPath = $Step
if (-not (Test-Path $stepPath)) { $stepPath = Join-Path $QaDir $Step }
try {
    . ([scriptblock]::Create([IO.File]::ReadAllText($stepPath, $utf8)))
} catch {
    Log "ERROR NO CONTROLADO: $($_.Exception.Message)"
    Log ($_.ScriptStackTrace)
    try { Shot "error-$([IO.Path]::GetFileNameWithoutExtension($stepPath))" | Out-Null } catch { }
    try { Show-Names } catch { }
    exit 1
}
