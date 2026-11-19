#!/usr/bin/env pwsh
# RePortal installer for Windows PowerShell.
#
# Downloads the latest (or pinned) reportal release archive from GitHub,
# installs the reportal and rep binaries into
# $env:LOCALAPPDATA\Programs\reportal, ensures that directory is on the
# user-scope PATH, and writes an idempotent guarded RePortal block to $PROFILE.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -c "irm https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.ps1 | iex"
#
# Optional env:
#   $env:REPORTAL_VERSION = 'v0.18.5'   Pin to a specific tag.
#
# Licensed under MIT.

$ErrorActionPreference = 'Stop'

$Repo          = 'CheckPickerUpper/reportal'
$ApiLatest     = "https://api.github.com/repos/$Repo/releases/latest"
$DownloadBase  = "https://github.com/$Repo/releases/download"
$InstallDir    = Join-Path $env:LOCALAPPDATA 'Programs\reportal'
$MarkerStart   = '# >>> reportal shell integration (do not edit) >>>'
$MarkerEnd     = '# <<< reportal shell integration <<<'
$ReinstallCommand = 'powershell -ExecutionPolicy Bypass -c "irm https://github.com/CheckPickerUpper/reportal/releases/latest/download/reportal-installer.ps1 | iex"'

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

function Test-PathContainsDirectory {
    param(
        [AllowEmptyString()][string]$PathValue,
        [string]$Dir
    )
    if (-not $PathValue) { return $false }

    $normalizedDir = $Dir.Trim().Replace('/', '\').TrimEnd([char]'\')
    foreach ($pathPart in ($PathValue -split ';')) {
        $normalizedPart = $pathPart.Trim().Replace('/', '\').TrimEnd([char]'\')
        if ([string]::Equals($normalizedPart, $normalizedDir, [System.StringComparison]::OrdinalIgnoreCase)) {
            return $true
        }
    }
    return $false
}

function Add-ToUserPath {
    param([string]$Dir)

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not $userPath) { $userPath = '' }
    if (Test-PathContainsDirectory -PathValue $userPath -Dir $Dir) {
        Write-Info "$Dir already on user PATH"
    } else {
        $newUserPath = if ($userPath) { "$userPath;$Dir" } else { $Dir }
        [Environment]::SetEnvironmentVariable('Path', $newUserPath, 'User')
        Write-Info "added $Dir to user PATH"
    }

    # A user-scope PATH update does not mutate the environment inherited by
    # this installer process. Refresh it before profile integration runs so
    # the newly installed commands resolve in the current PowerShell session.
    $processPath = $env:Path
    if (-not (Test-PathContainsDirectory -PathValue $processPath -Dir $Dir)) {
        $env:Path = if ($processPath) { "$Dir;$processPath" } else { $Dir }
        Write-Info "refreshed current process PATH with $Dir"
    }
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
    $lineEnding = if ($content.Contains("`r`n")) {
        "`r`n"
    } elseif ($content.Contains("`n")) {
        "`n"
    } else {
        [Environment]::NewLine
    }
    $managedBlock = @(
        $MarkerStart
        'if (Get-Command rep -CommandType Application -ErrorAction SilentlyContinue) {'
        '    Invoke-Expression (& rep init powershell | Out-String)'
        '} elseif (Get-Command reportal -CommandType Application -ErrorAction SilentlyContinue) {'
        '    Invoke-Expression (& reportal init powershell | Out-String)'
        '} else {'
        '    function global:rep {'
        "        Write-Error 'RePortal: rep.exe is not on PATH. Reinstall with: $ReinstallCommand'"
        '        $global:LASTEXITCODE = 127'
        '    }'
        '}'
        $MarkerEnd
    )
    $managedBlockText = $managedBlock -join $lineEnding

    $markerStartIndex = $content.IndexOf($MarkerStart, [System.StringComparison]::Ordinal)
    if ($markerStartIndex -ge 0) {
        $markerEndSearchStart = $markerStartIndex + $MarkerStart.Length
        $markerEndIndex = $content.IndexOf(
            $MarkerEnd,
            $markerEndSearchStart,
            [System.StringComparison]::Ordinal
        )
        if ($markerEndIndex -lt 0) {
            throw "found an incomplete RePortal shell integration block in $profilePath"
        }
        $afterMarkerEnd = $markerEndIndex + $MarkerEnd.Length
        $updatedContent = $content.Substring(0, $markerStartIndex) + $managedBlockText + $content.Substring($afterMarkerEnd)
        Set-Content -Path $profilePath -Value $updatedContent -NoNewline
        Write-Info "updated shell integration in $profilePath"
        return
    }

    $block = @('')
    if ($content -and -not $content.EndsWith($lineEnding)) {
        $block += ''
    }
    $block += $managedBlock
    Add-Content -Path $profilePath -Value ($block -join $lineEnding)
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
