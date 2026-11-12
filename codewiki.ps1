param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CodeWikiArgs
)

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
& (Join-Path $repoRoot 'codegraphy.ps1') @CodeWikiArgs
exit $LASTEXITCODE