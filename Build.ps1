# Build a self-contained Windows application without launching it.
$ErrorActionPreference = 'Stop'
$desktopTarget = Join-Path $PSScriptRoot 'target/release'
$builtExecutable = Join-Path $desktopTarget 'primitive_world.exe'
$desktopExecutable = Join-Path $PSScriptRoot 'Primitive World.exe'

function Get-ExecutableDigest([string]$path) {
    $stream = [System.IO.File]::OpenRead($path)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { return [System.BitConverter]::ToString($sha.ComputeHash($stream)) }
    finally { $sha.Dispose(); $stream.Dispose() }
}

Push-Location -LiteralPath $PSScriptRoot
try {
    & cargo build --release --bin primitive_world --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml') --target-dir (Join-Path $PSScriptRoot 'target')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    # Avoid touching a running copy when it already matches this build.
    $needsCopy = -not (Test-Path -LiteralPath $desktopExecutable -PathType Leaf)
    if (-not $needsCopy) {
        $needsCopy = (Get-ExecutableDigest $builtExecutable) -ne (Get-ExecutableDigest $desktopExecutable)
    }
    if ($needsCopy) {
        try { Copy-Item -LiteralPath $builtExecutable -Destination $desktopExecutable -Force }
        catch { throw "Could not update Primitive World.exe. Choose Save and quit in its tray menu, then build again. $($_.Exception.Message)" }
    }
    Write-Host "Ready: $desktopExecutable"
}
catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
finally { Pop-Location }
exit 0
