#!/usr/bin/env pwsh
# RePortal installer for Windows PowerShell.
#
# Downloads the latest (or pinned) reportal release archive from GitHub,
# installs the reportal and rep binaries into
# $env:LOCALAPPDATA\Programs\reportal, ensures that directory is on the
# user-scope PATH, and appends an idempotent
# `Invoke-Expression (& rep init powershell | Out-String)` block to
# $PROFILE.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -c "irm https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.ps1 | iex"
#
# Optional env:
#   $env:REPORTAL_VERSION = 'v0.15.0'   Pin to a specific tag.
#
# Licensed under MIT.

$ErrorActionPreference = 'Stop'

$Repo          = 'CheckPickerUpper/reportal'
$ApiLatest     = "https://api.github.com/repos/$Repo/releases/latest"
$DownloadBase  = "https://github.com/$Repo/releases/download"
$InstallDir    = Join-Path $env:LOCALAPPDATA 'Programs\reportal'
$MarkerStart   = '# >>> reportal shell integration (do not edit) >>>'
$MarkerEnd     = '# <<< reportal shell integration <<<'
$IntegrationLine = 'Invoke-Expression (& rep init powershell | Out-String)'

function Write-Info {
    param([string]$Message)
    Write-Host "reportal-installer: $Message"
}

function Resolve-Target {
    # cargo-dist currently builds x86_64-pc-windows-msvc only for Windows.
    # Accept arm64 Windows in the future by adding a mapping here.
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        'AMD64' { return 'x86_64-pc-windows-msvc' }
        'x86'   { return 'x86_64-pc-windows-msvc' }  # WOW64 path, still use x86_64 build
        default {
            throw "unsupported Windows architecture: $arch (only x86_64-pc-windows-msvc is published; use cargo install reportal)"
        }
    }
}

function Resolve-Tag {
    if ($env:REPORTAL_VERSION) {
        return $env:REPORTAL_VERSION
    }
    try {
        $headers = @{ 'User-Agent' = 'reportal-installer' }
        $response = Invoke-RestMethod -Uri $ApiLatest -Headers $headers
    } catch {
        throw "failed to query $ApiLatest : $($_.Exception.Message)"
    }
    if (-not $response.tag_name) {
        throw "could not parse tag_name from $ApiLatest"
    }
    return $response.tag_name
}

function Test-Sha256 {
    param(
        [string]$FilePath,
        [string]$ExpectedFile
    )
    $expectedLine = (Get-Content -Path $ExpectedFile -First 1).Trim()
    if (-not $expectedLine) {
        throw "empty checksum in $ExpectedFile"
    }
    # sha256 files may be "<digest>" or "<digest>  <filename>".
    $expected = ($expectedLine -split '\s+')[0].ToLower()
    $actual   = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $expected) {
        throw "checksum mismatch: expected $expected, got $actual"
    }
    Write-Info "checksum OK"
}

function Add-ToUserPath {
    param([string]$Dir)
    $current = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $current) { $current = '' }
    $parts = $current.Split(';') | Where-Object { $_ -ne '' }
    if ($parts -contains $Dir) {
        Write-Info "$Dir already on user PATH"
        return
    }
    $newPath = if ($current) { "$current;$Dir" } else { $Dir }
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
    # Also update the current session PATH.
    $env:Path = "$env:Path;$Dir"
    Write-Info "added $Dir to user PATH"
}

function Install-ShellIntegration {
    $profilePath = $PROFILE
    $profileDir  = Split-Path -Parent $profilePath
    if (-not (Test-Path $profileDir)) {
        New-Item -ItemType Directory -Path $profileDir -Force | Out-Null
    }
    if (-not (Test-Path $profilePath)) {
        New-Item -ItemType File -Path $profilePath -Force | Out-Null
    }

    $content = Get-Content -Path $profilePath -Raw -ErrorAction SilentlyContinue
    if ($null -eq $content) { $content = '' }
    if ($content.Contains($MarkerStart)) {
        Write-Info "shell integration already present in $profilePath"
        return
    }

    $block = @()
    if ($content -and -not $content.EndsWith([Environment]::NewLine)) {
        $block += ''
    }
    $block += ''
    $block += $MarkerStart
    $block += $IntegrationLine
    $block += $MarkerEnd
    Add-Content -Path $profilePath -Value ($block -join [Environment]::NewLine)
    Write-Info "appended shell integration to $profilePath"
}

function Main {
    $target = Resolve-Target
    $tag    = Resolve-Tag
    $archiveName = "reportal-$target.zip"
    $url     = "$DownloadBase/$tag/$archiveName"
    $shaUrl  = "$url.sha256"

    Write-Info "installing reportal $tag for $target"

    $tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("reportal-installer-" + [System.Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null
    try {
        $archivePath = Join-Path $tmpDir $archiveName
        $shaPath     = "$archivePath.sha256"

        Write-Info "downloading $url"
        Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing

        try {
            Invoke-WebRequest -Uri $shaUrl -OutFile $shaPath -UseBasicParsing -ErrorAction Stop
            Test-Sha256 -FilePath $archivePath -ExpectedFile $shaPath
        } catch {
            Write-Info "note: no .sha256 sidecar found; skipping checksum verification"
        }

        $extractDir = Join-Path $tmpDir 'extract'
        New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
        Expand-Archive -Path $archivePath -DestinationPath $extractDir -Force

        # cargo-dist archives extract to a subdir like reportal-<target>\.
        $innerDir = Join-Path $extractDir "reportal-$target"
        if (-not (Test-Path $innerDir)) {
            # Fall back: find any dir containing reportal.exe.
            $found = Get-ChildItem -Path $extractDir -Recurse -Filter 'reportal.exe' -File | Select-Object -First 1
            if (-not $found) {
                throw "could not locate reportal.exe inside archive"
            }
            $innerDir = $found.Directory.FullName
        }

        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        foreach ($binName in @('reportal.exe', 'rep.exe')) {
            $src = Join-Path $innerDir $binName
            if (-not (Test-Path $src)) {
                throw "missing binary in archive: $binName"
            }
            Copy-Item -Path $src -Destination (Join-Path $InstallDir $binName) -Force
        }

        Write-Info "installed binaries to $InstallDir"
        Add-ToUserPath -Dir $InstallDir
        Install-ShellIntegration

        Write-Host "RePortal $tag installed. Restart PowerShell (or dot-source `$PROFILE) to activate."
    } finally {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Main
