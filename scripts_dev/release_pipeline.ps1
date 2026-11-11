param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("check", "notes", "bump", "gate")]
    [string]$Mode,

    [string]$Version = "",
    [switch]$StrictZip,
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = Split-Path -Parent $ScriptDir

$VersionJsonPath = Join-Path $RepoRoot "version.json"
$CargoTomlPath = Join-Path $RepoRoot "tauri_app\src-tauri\Cargo.toml"
$CargoLockPath = Join-Path $RepoRoot "tauri_app\src-tauri\Cargo.lock"
$TauriConfPath = Join-Path $RepoRoot "tauri_app\src-tauri\tauri.conf.json"
$ChangelogPath = Join-Path $RepoRoot "CHANGELOG.md"
$ZipPath = Join-Path $RepoRoot "CecDesk_Windows_x64.zip"
$ExePath = Join-Path $RepoRoot "CecDesk.exe"

function Read-Utf8([string]$Path) {
    return [System.IO.File]::ReadAllText($Path, $Utf8NoBom)
}

function Write-Utf8([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, $Utf8NoBom)
}

function Get-ProjectVersions {
    $vj = Read-Utf8 $VersionJsonPath | ConvertFrom-Json
    $cargoTxt = Read-Utf8 $CargoTomlPath
    $tauriObj = Read-Utf8 $TauriConfPath | ConvertFrom-Json
    $lockTxt = Read-Utf8 $CargoLockPath
    $changelogTxt = Read-Utf8 $ChangelogPath

    $cargoVer = ""
    if ($cargoTxt -match '(?ms)^\[package\].*?^version\s*=\s*"([^"]+)"') {
        $cargoVer = $Matches[1]
    }

    $lockVer = ""
    if ($lockTxt -match '(?ms)name\s*=\s*"cecdesk"\r?\nversion\s*=\s*"([^"]+)"') {
        $lockVer = $Matches[1]
    }

    return [pscustomobject]@{
        VersionJson = [string]$vj.version
        CargoToml   = $cargoVer
        TauriConf   = [string]$tauriObj.version
        CargoLock   = $lockVer
        ManifestObj = $vj
        Changelog   = $changelogTxt
    }
}

function Extract-ChangelogSection([string]$ChangelogText, [string]$TargetVersion) {
    $escaped = [regex]::Escape($TargetVersion)
    $pattern = "(?ms)^## \[$escaped\][^\r\n]*\r?\n(.*?)(?=^## \[|\z)"
    $m = [regex]::Match($ChangelogText, $pattern)
    if (-not $m.Success) {
        return ""
    }
    return ($m.Groups[1].Value -replace '(?m)^---\s*$', '').Trim()
}

function Convert-SectionToManifestNotes([string]$TargetVersion, [string]$SectionBody) {
    $bullets = @()
    foreach ($line in ($SectionBody -split "\r?\n")) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("- ")) {
            $bullets += $trimmed
        }
    }
    if ($bullets.Count -eq 0) {
        return "CecDesk v${TargetVersion}"
    }
    $header = "CecDesk v${TargetVersion}:"
    return ($header + "`n" + ($bullets -join "`n"))
}

function Invoke-Check {
    $info = Get-ProjectVersions
    $errors = @()

    if ($info.CargoToml -ne $info.VersionJson) {
        $errors += "Cargo.toml version ($($info.CargoToml)) != version.json ($($info.VersionJson))"
    }
    if ($info.TauriConf -ne $info.VersionJson) {
        $errors += "tauri.conf.json version ($($info.TauriConf)) != version.json ($($info.VersionJson))"
    }
    if ($info.CargoLock -ne $info.VersionJson) {
        $errors += "Cargo.lock cecdesk version ($($info.CargoLock)) != version.json ($($info.VersionJson)) (run cargo check/test to sync)"
    }

    $section = Extract-ChangelogSection $info.Changelog $info.VersionJson
    if ([string]::IsNullOrWhiteSpace($section)) {
        $errors += "CHANGELOG.md missing section ## [$($info.VersionJson)]"
    }

    $expectedTag = "v$($info.VersionJson)"
    if ($info.ManifestObj.windows_package_url -notmatch [regex]::Escape("mitrafun-creator/CecDesk/releases/download/$expectedTag/CecDesk_Windows_x64.zip")) {
        $errors += "version.json windows_package_url does not match tag $expectedTag"
    }

    if (Test-Path $ZipPath) {
        $actualZipSha = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLower()
        if ($info.ManifestObj.windows_sha256 -ne $actualZipSha) {
            $errors += "version.json windows_sha256 ($($info.ManifestObj.windows_sha256)) != local CecDesk_Windows_x64.zip ($actualZipSha)"
        } else {
            Write-Host "[OK] Local CecDesk_Windows_x64.zip SHA-256 matches version.json ($actualZipSha)"
        }
    } elseif ($StrictZip) {
        $errors += "CecDesk_Windows_x64.zip not found at $ZipPath"
    } else {
        Write-Host "[INFO] CecDesk_Windows_x64.zip not present locally; skipped hash comparison."
    }

    if ($errors.Count -gt 0) {
        foreach ($err in $errors) {
            Write-Host "[FAIL] $err" -ForegroundColor Red
        }
        exit 1
    }

    Write-Host "[OK] Version parity verified: $($info.VersionJson)"
}

function Invoke-Notes {
    $info = Get-ProjectVersions
    $target = if ([string]::IsNullOrWhiteSpace($Version)) { $info.VersionJson } else { $Version.TrimStart("v") }
    $section = Extract-ChangelogSection $info.Changelog $target
    if ([string]::IsNullOrWhiteSpace($section)) {
        throw "Section ## [$target] not found in CHANGELOG.md"
    }
    $header = "## CecDesk v$target"
    $output = "$header`n`n$section"
    if (-not [string]::IsNullOrWhiteSpace($OutFile)) {
        Write-Utf8 $OutFile $output
        Write-Host "[OK] Wrote release notes to $OutFile"
    } else {
        Write-Output $output
    }
}

function Invoke-Bump {
    $target = $Version.TrimStart("v").Trim()
    if ($target -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
        throw "Invalid SemVer version '$Version'. Expected e.g. 1.1.0-beta.2 or 1.2.0"
    }

    $today = (Get-Date).ToString("yyyy-MM-dd")

    # 1. Update Cargo.toml
    $cargoTxt = Read-Utf8 $CargoTomlPath
    $cargoUpdated = [regex]::Replace(
        $cargoTxt,
        '(?ms)(^\[package\].*?^version\s*=\s*")[^"]+(")',
        ('${1}' + $target + '${2}'),
        1
    )
    Write-Utf8 $CargoTomlPath $cargoUpdated

    # 2. Update tauri.conf.json
    $tauriTxt = Read-Utf8 $TauriConfPath
    $tauriUpdated = [regex]::Replace(
        $tauriTxt,
        '("version"\s*:\s*")[^"]+(")',
        ('${1}' + $target + '${2}'),
        1
    )
    Write-Utf8 $TauriConfPath $tauriUpdated

    # 3. Promote [Unreleased] in CHANGELOG.md
    $changelogTxt = Read-Utf8 $ChangelogPath
    $existingSection = Extract-ChangelogSection $changelogTxt $target
    if ([string]::IsNullOrWhiteSpace($existingSection)) {
        $unreleasedPattern = '(?ms)^## \[Unreleased\]\s*\r?\n(.*?)(?=^## \[|\z)'
        $um = [regex]::Match($changelogTxt, $unreleasedPattern)
        if (-not $um.Success) {
            throw "CHANGELOG.md must contain a ## [Unreleased] section."
        }
        $body = $um.Groups[1].Value.Trim()
        if ([string]::IsNullOrWhiteSpace($body)) {
            $body = "### Changed`n- Техническое обновление версии $target."
        }
        $replacement = "## [Unreleased]`n`n### Added`n### Changed`n### Fixed`n`n---`n`n## [$target] - $today`n`n$body`n`n"
        $changelogUpdated = $changelogTxt.Substring(0, $um.Index) + $replacement + $changelogTxt.Substring($um.Index + $um.Length)
        Write-Utf8 $ChangelogPath $changelogUpdated
        $changelogTxt = $changelogUpdated
    }
    $newSectionBody = Extract-ChangelogSection $changelogTxt $target
    $manifestNotes = Convert-SectionToManifestNotes $target $newSectionBody

    # 4. Update version.json
    $vjObj = Read-Utf8 $VersionJsonPath | ConvertFrom-Json
    $vjObj.version = $target
    $vjObj.published_at = $today
    $vjObj.notes = $manifestNotes
    $vjObj.download_url = "https://github.com/mitrafun-creator/CecDesk/releases/tag/v$target"
    $vjObj.windows_package_url = "https://github.com/mitrafun-creator/CecDesk/releases/download/v$target/CecDesk_Windows_x64.zip"
    $vjJson = $vjObj | ConvertTo-Json -Depth 5
    Write-Utf8 $VersionJsonPath ($vjJson + "`n")

    # 5. Refresh Cargo.lock using cargo metadata/check
    Push-Location (Join-Path $RepoRoot "tauri_app\src-tauri")
    try {
        & "C:\rust\toolchain\bin\cargo.exe" update -p cecdesk --offline 2>$null | Out-Null
    } finally {
        Pop-Location
    }

    Write-Host "[OK] Bumped version to $target across Cargo.toml, tauri.conf.json, version.json, and CHANGELOG.md."
    Write-Host "[NEXT] Run 'release_tool.bat gate' to build, test, and sync windows_sha256."
}

function Invoke-Gate {
    Write-Host "=== Step 1/6: Checking version & CHANGELOG parity ==="
    Invoke-Check

    Write-Host "=== Step 2/6: Running WebUI Node unit tests ==="
    & node --test (Join-Path $RepoRoot "tauri_app\ui\js\catalog.test.js")
    if ($LASTEXITCODE -ne 0) { throw "Node tests failed" }

    Write-Host "=== Step 3/6: Running Cargo Clippy ==="
    & cmd /c (Join-Path $RepoRoot "run_clippy.bat")
    if ($LASTEXITCODE -ne 0) { throw "Clippy failed" }

    Write-Host "=== Step 4/6: Running Rust unit tests ==="
    & cmd /c (Join-Path $RepoRoot "run_tests.bat")
    if ($LASTEXITCODE -ne 0) { throw "Rust unit tests failed" }

    Write-Host "=== Step 5/6: Building Release & packaging ZIP ==="
    & cmd /c (Join-Path $RepoRoot "build_release.bat")
    if ($LASTEXITCODE -ne 0) { throw "build_release.bat failed" }

    Write-Host "=== Step 6/6: Checking binary hygiene & final SHA-256 ==="
    & python -c "
import os, sys
data = open(sys.argv[1], 'rb').read()
user = os.environ.get('USERNAME', '')
hits = 0
if user:
    hits += data.count(user.encode('utf-8')) + data.count(user.encode('utf-16le'))
print('Username leak hits in CecDesk.exe:', hits)
sys.exit(1 if hits > 0 else 0)
" $ExePath
    if ($LASTEXITCODE -ne 0) { throw "Binary username check failed" }

    $StrictZip = $true
    Invoke-Check

    $info = Get-ProjectVersions
    $tag = "v$($info.VersionJson)"
    Write-Host ""
    Write-Host "=== RELEASE GATE PASSED ($tag) ===" -ForegroundColor Green
    Write-Host "Suggested publish commands:"
    Write-Host "  git add -A && git commit -m `"release: $tag`""
    Write-Host "  git tag $tag && git push origin main --tags"
    Write-Host "  release_tool.bat notes > release_notes.tmp"
    Write-Host "  gh release create $tag CecDesk_Windows_x64.zip bin/firmware/rp2040_zero_firmware.uf2 bin/firmware/esp32_s3_firmware.bin --prerelease --title `"CecDesk $tag`" --notes-file release_notes.tmp"
}

switch ($Mode) {
    "check" { Invoke-Check }
    "notes" { Invoke-Notes }
    "bump"  { Invoke-Bump }
    "gate"  { Invoke-Gate }
}
