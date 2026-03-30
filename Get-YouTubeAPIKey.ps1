param(
    [string]$OnePasswordReference = $(
        if (-not [string]::IsNullOrWhiteSpace($env:TEAMY_YOUTUBE_1PASSWORD_YOUTUBE_API_KEY_REFERENCE)) {
            $env:TEAMY_YOUTUBE_1PASSWORD_YOUTUBE_API_KEY_REFERENCE
        }
        else {
            "op://Private/YouTube Data API v3 - Nanuak/credential"
        }
    )
)

$ErrorActionPreference = "Stop"

function Write-YouTubeApiKeyTroubleshooting {
    Write-Host "[Get-YouTubeAPIKey] Manage API keys: https://console.cloud.google.com/apis/credentials" -ForegroundColor Yellow
    Write-Host "[Get-YouTubeAPIKey] Enable YouTube Data API v3: https://console.cloud.google.com/apis/library/youtube.googleapis.com" -ForegroundColor Yellow
}

$validateOutput = cargo run -- api key validate 2>$null
if ($LASTEXITCODE -eq 0) {
    Write-Host "[Get-YouTubeAPIKey] API key already configured."
    $validateOutput
    return
}

if ([string]::IsNullOrWhiteSpace($OnePasswordReference)) {
    throw "Set TEAMY_YOUTUBE_1PASSWORD_YOUTUBE_API_KEY_REFERENCE or pass -OnePasswordReference."
}

Write-Host "[Get-YouTubeAPIKey] Reading YouTube API key from 1Password..."
$apiKey = op read $OnePasswordReference --no-newline
if ([string]::IsNullOrWhiteSpace($apiKey)) {
    throw "1Password returned an empty YouTube API key."
}

cargo run -- api key set $apiKey
if ($LASTEXITCODE -ne 0) {
    throw "Failed to persist the YouTube API key."
}

cargo run -- api key validate
if ($LASTEXITCODE -ne 0) {
    Write-YouTubeApiKeyTroubleshooting
    throw "The persisted YouTube API key failed validation."
}