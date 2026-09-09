param(
    [ValidateRange(0.0001, 168)][double]$Hours = 12,
    [ValidateRange(1, 1000000)][int]$ChunkTicks = 50000,
    [uint32]$Seed = 42,
    [ValidateRange(60, 86400)][int]$ChunkTimeoutSeconds = 1800,
    [string]$ResumeDirectory,
    [string]$Executable
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$runDirectory = $null
$runLock = $null
function Write-Receipt($value, $path) {
    $temporary = "$path.tmp"
    $json = $value | ConvertTo-Json -Depth 12
    [IO.File]::WriteAllText($temporary, $json, (New-Object Text.UTF8Encoding($false)))
    if (Test-Path -LiteralPath $path) {
        [IO.File]::Replace($temporary, $path, "$path.bak")
    } else {
        [IO.File]::Move($temporary, $path)
    }
}
function Invoke-SoakProcess($program, $arguments, $outputLog) {
    # Argument values are fixed switches/numbers or Windows paths, which cannot
    # contain a double quote. Quote each value for Start-Process on PowerShell 5/7.
    $quoted = @($arguments | ForEach-Object {
        if ($_.Contains('"')) { throw 'Unexpected quote in process argument.' }
        '"' + $_ + '"'
    })
    $process = Start-Process -FilePath $program -ArgumentList $quoted -WindowStyle Hidden -PassThru -RedirectStandardOutput $outputLog -RedirectStandardError ($outputLog + '.stderr')
    $deadline = [DateTime]::UtcNow.AddSeconds($ChunkTimeoutSeconds)
    $peakBytes = [long]0
    while (-not $process.WaitForExit(1000)) {
        $process.Refresh()
        $peakBytes = [Math]::Max($peakBytes, $process.WorkingSet64)
        if ([DateTime]::UtcNow -gt $deadline) {
            $process.Kill()
            throw "Process timeout after $ChunkTimeoutSeconds seconds; last validated receipt is preserved."
        }
    }
    $process.WaitForExit()
    $exitCode = $process.ExitCode
    Write-Host ("Process exit {0}; sampled peak working set {1:N0} MiB" -f $exitCode, ($peakBytes / 1MB))
    $process.Dispose()
    if ($exitCode -ne 0) { throw "Process failed ($exitCode). Inspect $outputLog and its .stderr file; last validated receipt is preserved." }
}
try {
    if ($ResumeDirectory) {
        $runDirectory = (Resolve-Path -LiteralPath $ResumeDirectory).Path
    } else {
        $runDirectory = Join-Path $repoRoot ('reports/soak-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0,8))
        New-Item -ItemType Directory -Path $runDirectory | Out-Null
    }
    $runLock = [IO.File]::Open((Join-Path $runDirectory 'run.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    $receiptPath = Join-Path $runDirectory 'session.json'
    $runExecutable = Join-Path $runDirectory 'primitive_world.exe'
    if ($ResumeDirectory) {
        $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
        if ($receipt.status -eq 'engine_capacity') { throw 'This run reached an engine limit and is censored. Start a fresh experiment.' }
        if ((Get-FileHash -LiteralPath $runExecutable -Algorithm SHA256).Hash -ne $receipt.executable_sha256) {
            throw 'The saved executable changed. Resume requires the original binary.'
        }
        if ($Executable) { throw '-Executable only applies when starting a new run.' }
        $ChunkTicks = $receipt.chunk_ticks
    } else {
        if (-not $Executable) {
            $buildDirectory = Join-Path $repoRoot 'target/soak'
            & cargo build --release --manifest-path (Join-Path $repoRoot 'Cargo.toml') --target-dir $buildDirectory
            if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
            $Executable = Join-Path $buildDirectory 'release/primitive_world.exe'
        }
        Copy-Item -LiteralPath (Resolve-Path -LiteralPath $Executable).Path -Destination $runExecutable
        $receipt = [pscustomobject]@{
            schema = 1; seed = $Seed; chunk_ticks = $ChunkTicks; completed_chunks = 0
            elapsed_ticks = [long]0; latest_checkpoint = $null; previous_checkpoint = $null
            status = 'ready'; executable_sha256 = (Get-FileHash -LiteralPath $runExecutable -Algorithm SHA256).Hash
            started_utc = [DateTime]::UtcNow.ToString('o'); updated_utc = [DateTime]::UtcNow.ToString('o')
        }
        Write-Receipt $receipt $receiptPath
    }
    Write-Host "Run directory: $runDirectory"
    Write-Host 'Ctrl+C stops this invocation; resume from the latest completed chunk. At most one unfinished chunk is lost.'
    $deadline = [DateTime]::UtcNow.AddHours($Hours)
    do {
        # Incomplete attempts remain diagnosable, but cannot consume unbounded disk.
        $runBytes = (Get-ChildItem -LiteralPath $runDirectory -File | Measure-Object -Property Length -Sum).Sum
        if ($runBytes -gt 8GB) { throw 'Run directory exceeded the 8 GiB storage budget. Inspect incomplete attempts before resuming.' }
        # Unique names preserve incomplete attempts after a crash or interruption.
        $attempt = ('chunk-{0:D6}-{1}' -f ([int]$receipt.completed_chunks + 1), [guid]::NewGuid().ToString('N').Substring(0,8))
        $reportPath = Join-Path $runDirectory "$attempt.json"
        $checkpointName = "$attempt.checkpoint"
        $checkpointPath = Join-Path $runDirectory $checkpointName
        $runArguments = @('--headless','--ticks',"$ChunkTicks",'--sample','4096','--output',$reportPath,'--save-checkpoint',$checkpointPath)
        if ($receipt.latest_checkpoint) {
            if ([IO.Path]::GetFileName($receipt.latest_checkpoint) -ne $receipt.latest_checkpoint) { throw 'Invalid checkpoint name in receipt.' }
            $runArguments += @('--checkpoint',(Join-Path $runDirectory $receipt.latest_checkpoint))
        } else { $runArguments += @('--seed',"$($receipt.seed)") }
        Invoke-SoakProcess $runExecutable $runArguments (Join-Path $runDirectory "$attempt.log")
        $report = Get-Content -LiteralPath $reportPath -Raw | ConvertFrom-Json
        if (-not (Test-Path -LiteralPath $checkpointPath -PathType Leaf)) { throw 'No completed checkpoint was produced.' }
        # Validate the exact save with its original binary before advancing the receipt.
        Invoke-SoakProcess $runExecutable @('--headless','--ticks','0','--checkpoint',$checkpointPath,'--output',(Join-Path $runDirectory "$attempt-validation.json")) (Join-Path $runDirectory "$attempt-validation.log")
        $expired = $receipt.previous_checkpoint
        $receipt.previous_checkpoint = $receipt.latest_checkpoint
        $receipt.latest_checkpoint = $checkpointName
        $receipt.completed_chunks++
        $receipt.elapsed_ticks = [long]$receipt.elapsed_ticks + [long]$report.elapsed_ticks
        $receipt.updated_utc = [DateTime]::UtcNow.ToString('o')
        $receipt.status = if ($report.termination_reason -in @('engine_capacity','tick_capacity')) { 'engine_capacity' } else { 'ready' }
        Write-Receipt $receipt $receiptPath
        # Only an explicitly named older checkpoint inside this run may be removed.
        if ($expired) {
            if ([IO.Path]::GetFileName($expired) -ne $expired) { throw 'Invalid retention path in receipt.' }
            $expiredPath = [IO.Path]::GetFullPath((Join-Path $runDirectory $expired))
            if ([IO.Path]::GetDirectoryName($expiredPath) -ne $runDirectory) { throw 'Retention path is outside the run directory.' }
            if (Test-Path -LiteralPath $expiredPath) { Remove-Item -LiteralPath $expiredPath }
        }
        # Reports and logs are observational: retain a bounded recent window.
        $oldDiagnostics = Get-ChildItem -LiteralPath $runDirectory -File |
            Where-Object { $_.Name -match '^chunk-[0-9]{6}-[0-9a-f]{8}(-validation)?[.](json|log)([.]stderr)?$' } |
            Sort-Object LastWriteTimeUtc -Descending | Select-Object -Skip 256
        foreach ($diagnostic in $oldDiagnostics) {
            $diagnosticPath = [IO.Path]::GetFullPath($diagnostic.FullName)
            if ([IO.Path]::GetDirectoryName($diagnosticPath) -ne $runDirectory) { throw 'Diagnostic retention path is outside the run directory.' }
            Remove-Item -LiteralPath $diagnosticPath
        }
        Write-Host ("Completed {0} chunks; {1} ticks; world {2}; status {3}" -f $receipt.completed_chunks,$receipt.elapsed_ticks,$report.final_progress.world,$receipt.status)
        if ($receipt.status -eq 'engine_capacity') { break }
    } while ([DateTime]::UtcNow -lt $deadline)
    Write-Host "Saved. Resume with: .\tools\Soak.ps1 -ResumeDirectory '$runDirectory' -Hours $Hours"
} finally {
    if ($runLock) { $runLock.Dispose() }
}
