# Uzumaki installer for Windows (PowerShell)
# Usage: irm https://uzumaki.dev/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "golok727/uzumaki"
$InstallDir = if ($env:UZUMAKI_INSTALL) { $env:UZUMAKI_INSTALL } else { "$env:USERPROFILE\.uzumaki\bin" }

# Detect architecture
$Arch = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64"   { "x64" }
    "Arm64" { "arm64" }
    default { Write-Error "Unsupported architecture: $_"; exit 1 }
}

$Asset = "uzumaki-windows-${Arch}.zip"

# Fetch latest version
if ($env:UZUMAKI_VERSION) {
    $Version = "v$($env:UZUMAKI_VERSION -replace '^v', '')"
} else {
    $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "uzumaki-installer" }
    $Version = $Response.tag_name
    if (-not $Version) {
        Write-Error "Could not determine latest version"
        exit 1
    }
}

$Url = "https://github.com/$Repo/releases/download/$Version/$Asset"

Write-Host ""
Write-Host "  " -NoNewline
Write-Host "Uzumaki" -ForegroundColor Cyan -NoNewline
Write-Host " installer"
Write-Host ""
Write-Host "  Version:  $Version"
Write-Host "  Platform: windows-$Arch"
Write-Host "  Install:  $InstallDir"
Write-Host ""

# Download
$TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("uzumaki-install-" + [System.Guid]::NewGuid().ToString("N").Substring(0,8))
New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null
$ZipPath = Join-Path $TmpDir $Asset

Write-Host "  Downloading $Url..."
Invoke-WebRequest -Uri $Url -OutFile $ZipPath -UseBasicParsing

Write-Host "  Extracting..."
Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

# Install
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
$BinaryPath = Join-Path $InstallDir "uzumaki.exe"
Move-Item -Path (Join-Path $TmpDir "uzumaki.exe") -Destination $BinaryPath -Force

# Cleanup
Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "  Uzumaki was installed successfully!" -ForegroundColor Green
Write-Host ""

# Check PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
    Write-Host "  Adding $InstallDir to your PATH..."
    [Environment]::SetEnvironmentVariable("PATH", "$InstallDir;$UserPath", "User")
    $env:PATH = "$InstallDir;$env:PATH"
    Write-Host "  Done! You may need to restart your terminal." -ForegroundColor Yellow
    Write-Host ""
}
