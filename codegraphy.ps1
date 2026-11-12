param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CodeGraphyArgs
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $repoRoot
try {
    $cliArgs = @($CodeGraphyArgs)
    if ($cliArgs.Count -eq 0) { $cliArgs = @("ui") }
    $isInit = $cliArgs.Count -gt 0 -and ($cliArgs[0] -eq "init" -or $cliArgs[0] -eq "scan")

    if (-not $isInit) {
        $webModules = Join-Path $repoRoot "web/node_modules"
        $needsBuild = -not (Test-Path -LiteralPath $webModules)
        if (-not (Test-Path -LiteralPath $webModules)) {
            Write-Host "Installing pinned CodeGraphy web dependencies..."
            npm ci --prefix web
            if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        }
        $webDist = Join-Path $repoRoot "web/dist"
        if (-not (Test-Path -LiteralPath $webDist)) {
            $needsBuild = $true
        } else {
            $webInputs = @("src", "public", "index.html", "package.json", "package-lock.json", "vite.config.ts", "tsconfig.json") |
                ForEach-Object { Join-Path (Join-Path $repoRoot "web") $_ } |
                Where-Object { Test-Path -LiteralPath $_ }
            $latestSource = Get-ChildItem $webInputs -Recurse -File |
                Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
            $latestOutput = Get-ChildItem $webDist -Recurse -File |
                Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
            if (-not $latestOutput -or ($latestSource -and $latestSource.LastWriteTimeUtc -gt $latestOutput.LastWriteTimeUtc)) {
                $needsBuild = $true
            }
        }
        if ($needsBuild) {
            npm --prefix web run build
            if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        }
    }

    cargo run --manifest-path (Join-Path $repoRoot "Cargo.toml") -p codewiki-cli -- @cliArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
