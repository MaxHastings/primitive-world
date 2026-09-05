# Build without opening a window, then play when you run this script.
# Example: .\Play.ps1 --random-founders --seed 42 --watch-loop runs/my-run --view-speed 16x
# All arguments go to the game, including --headless when desired.
# The separate target directory leaves target/debug and target/release alone.
$ErrorActionPreference = 'Stop'
[string[]]$playArguments = @($args)
$playTarget = Join-Path $PSScriptRoot 'target/play'
$playExecutable = Join-Path $playTarget 'release/primitive_world.exe'
$playExitCode = 1

Push-Location -LiteralPath $PSScriptRoot
try {
    [string[]]$buildArguments = @(
        'build', '--release', '--bin', 'primitive_world',
        '--manifest-path', (Join-Path $PSScriptRoot 'Cargo.toml'),
        '--target-dir', $playTarget
    )
    & cargo @buildArguments
    $playExitCode = $LASTEXITCODE
    if ($playExitCode -eq 0) {
        if (-not (Test-Path -LiteralPath $playExecutable -PathType Leaf)) {
            throw "Build finished but the play executable was not found: $playExecutable"
        }
        & $playExecutable @playArguments
        $playExitCode = $LASTEXITCODE
    }
}
finally {
    Pop-Location
}

exit $playExitCode
