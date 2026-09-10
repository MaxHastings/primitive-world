# Build the current app, then return immediately for wallpaper/windowed play.
# Diagnostics and maintenance commands retain their exit code and console output.
$ErrorActionPreference = 'Stop'
[string[]]$playArguments = @($args)
if ($playArguments.Count -eq 0) { $playArguments = @('--viewer') }
$playExecutable = Join-Path $PSScriptRoot 'Primitive World.exe'

# Start-Process joins ArgumentList into a Windows command line. Quote each
# argument explicitly so spaces, quotes and trailing backslashes survive.
function ConvertTo-NativeArgument([string]$value) {
    if ($value.Length -gt 0 -and $value -notmatch '[\s"]') { return $value }
    $escaped = [regex]::Replace($value, '(\\*)"', '$1$1\"')
    $escaped = [regex]::Replace($escaped, '(\\+)$', '$1$1')
    return '"' + $escaped + '"'
}

try {
    & (Join-Path $PSScriptRoot 'Build.ps1')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    $commandLine = ($playArguments | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '
    $consoleOptions = @('--headless', '--help', '--version', '--install-startup', '--uninstall-startup', '--stop-wallpaper', '--prune-saves')
    $consoleMode = @($playArguments | Where-Object { $_ -in $consoleOptions }).Count -gt 0
    if ($consoleMode) {
        $process = Start-Process -FilePath $playExecutable -ArgumentList $commandLine -NoNewWindow -Wait -PassThru
        exit $process.ExitCode
    }
    Start-Process -FilePath $playExecutable -ArgumentList $commandLine -WorkingDirectory $PSScriptRoot -WindowStyle Hidden
    Write-Host 'Primitive World started. You can close this terminal.'
}
catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
exit 0
