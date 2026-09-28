#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Installer for cucco on Windows.

.DESCRIPTION
    Downloads the latest (or a specified) cucco release for Windows from
    GitHub, verifies its checksum, extracts the binary into an install
    directory and adds it to the current user's PATH.

.PARAMETER Version
    The cucco version to install (defaults to the latest released version).

.PARAMETER InstallDir
    The directory the cucco.exe binary is installed into.
    Defaults to "$env:LOCALAPPDATA\Programs\cucco".

.PARAMETER Force
    Skip the confirmation prompt during installation. Alias: -Yes.

.EXAMPLE
    irm https://veeso.github.io/cucco/install.ps1 | iex

.EXAMPLE
    .\install.ps1 -Version 1.0.0 -Force
#>
[CmdletBinding()]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingWriteHost', '', Justification = 'Colored console output is the point of an interactive installer script')]
[Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSReviewUnusedParameter', '', Justification = '$InstallDir and $Force are read from script scope inside Install-Cucco and Confirm-Action')]
param(
    [string]$Version = "",
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\cucco",
    [Alias("Yes")]
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$GithubRepo = "veeso/cucco"
$IssuesUrl = "https://github.com/$GithubRepo/issues/new"

# -- output helpers ----------------------------------------------------------

function Write-Info {
    param([string]$Message)
    Write-Host "> " -ForegroundColor DarkGray -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "! $Message" -ForegroundColor Yellow
}

function Write-Err {
    param([string]$Message)
    Write-Host "x $Message" -ForegroundColor Red
}

function Write-Completed {
    param([string]$Message)
    Write-Host "✓ " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Confirm-Action {
    param([string]$Message)
    if ($Force) {
        return
    }
    $answer = Read-Host "? $Message [y/N]"
    if ($answer -ne "y" -and $answer -ne "yes") {
        Write-Err 'Aborting (please answer "yes" to continue)'
        exit 1
    }
}

# -- platform detection ------------------------------------------------------

function Get-CuccoTarget {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($env:PROCESSOR_ARCHITEW6432) {
        $arch = $env:PROCESSOR_ARCHITEW6432
    }

    switch ($arch.ToUpper()) {
        "AMD64" { return "x86_64-pc-windows-msvc" }
        "ARM64" { return "aarch64-pc-windows-msvc" }
        default {
            Write-Err "Unsupported architecture: $arch"
            Write-Info "Only x86_64 (AMD64) and ARM64 are supported by this installer."
            Write-Info "Alternatively, download a release archive from <https://github.com/veeso/cucco/releases>"
            exit 1
        }
    }
}

# -- version resolution ------------------------------------------------------

function Get-LatestCuccoVersion {
    try {
        $release = Invoke-RestMethod `
            -Uri "https://api.github.com/repos/$GithubRepo/releases/latest" `
            -Headers @{ "User-Agent" = "cucco-installer" } `
            -UseBasicParsing
    } catch {
        Write-Err "Could not query the latest cucco release: $($_.Exception.Message)"
        Write-Warn "If no release has been published yet, pass a version explicitly with '-Version X.Y.Z'."
        Write-Warn "If you believe this is a bug, please report an issue at <$IssuesUrl>"
        exit 1
    }
    return $release.tag_name.TrimStart("v")
}

# -- installation ------------------------------------------------------------

function Install-Cucco {
    param(
        [string]$InstallVersion,
        [string]$Target
    )

    $asset = "cucco-v$InstallVersion-$Target.zip"
    $url = "https://github.com/$GithubRepo/releases/download/v$InstallVersion/$asset"

    Write-Host ""
    Write-Host "  cucco configuration"
    Write-Info "Version:       $InstallVersion"
    Write-Info "Target:        $Target"
    Write-Info "Install dir:   $InstallDir"
    Write-Host ""

    Confirm-Action "Install cucco $InstallVersion?"

    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "cucco-$([System.IO.Path]::GetRandomFileName())"
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

    try {
        $archive = Join-Path $tmpDir $asset
        Write-Info "Downloading cucco from $url …"
        try {
            Invoke-WebRequest -Uri $url -OutFile $archive -UseBasicParsing
        } catch {
            Write-Err "Failed to download cucco: $($_.Exception.Message)"
            Write-Warn "Check that release v$InstallVersion exists and provides artifacts for $Target."
            Write-Warn "If you believe this is a bug, please report an issue at <$IssuesUrl>"
            exit 1
        }

        $checksumFile = "$archive.sha256"
        try {
            Invoke-WebRequest -Uri "$url.sha256" -OutFile $checksumFile -UseBasicParsing
            $expected = (Get-Content $checksumFile -Raw).Trim().ToLower()
            $actual = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLower()
            if ($expected -ne $actual) {
                Write-Err "Checksum mismatch for the downloaded archive (expected $expected, got $actual)."
                Write-Err "Please retry, and report an issue at <$IssuesUrl> if the problem persists."
                exit 1
            }
            Write-Info "Checksum verified"
        } catch {
            Write-Warn "Could not verify the archive checksum: $($_.Exception.Message)"
        }

        Write-Info "Extracting archive …"
        Expand-Archive -Path $archive -DestinationPath $tmpDir -Force

        $binary = Join-Path $tmpDir "cucco.exe"
        if (-not (Test-Path $binary)) {
            Write-Err "cucco.exe was not found in the downloaded archive."
            Write-Warn "Please report an issue at <$IssuesUrl>"
            exit 1
        }

        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        }

        Write-Info "Installing cucco to $InstallDir …"
        Copy-Item -Path $binary -Destination (Join-Path $InstallDir "cucco.exe") -Force

        Add-ToUserPath -Directory $InstallDir
    } finally {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Add-ToUserPath {
    param([string]$Directory)

    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @()
    if ($userPath) {
        $entries = $userPath.Split(";") | Where-Object { $_ -ne "" }
    }

    if ($entries -contains $Directory) {
        return
    }

    Write-Info "Adding $Directory to your user PATH …"
    $newPath = (@($entries) + $Directory) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    # make cucco available in the current session too
    $env:Path = "$env:Path;$Directory"
    Write-Warn "Restart your terminal for the PATH change to take effect in new sessions."
}

# -- main --------------------------------------------------------------------

$target = Get-CuccoTarget
if (-not $Version) {
    Write-Info "Resolving the latest cucco version…"
    $Version = Get-LatestCuccoVersion
}

Install-Cucco -InstallVersion $Version -Target $target

Write-Completed "cucco has successfully been installed on your system!"
Write-Info "Usage: cucco --help"
Write-Info "If you encounter any issue, please report it at <$IssuesUrl>"

exit 0
