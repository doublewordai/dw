# Doubleword CLI installer for Windows
# Usage: irm https://raw.githubusercontent.com/doublewordai/dw/main/install.ps1 | iex

$ErrorActionPreference = 'Stop'

# Windows PowerShell 5.1 can still default to TLS 1.0, which GitHub rejects.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = 'doublewordai/dw'
$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\dw'

function Fail($message) {
    Write-Host "Error: $message" -ForegroundColor Red
    exit 1
}

Write-Host 'Doubleword CLI Installer' -ForegroundColor White

# PROCESSOR_ARCHITECTURE reports x86 under a 32-bit shell, so prefer the W6432
# variant when it is set. ARM64 runs the x64 build through emulation.
$arch = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
if ($arch -notin @('AMD64', 'ARM64')) {
    Fail "Unsupported architecture: $arch. Only x64 is published."
}

$version = (Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest").tag_name -replace '^v', ''
if (-not $version) { Fail "Could not determine the latest version. See https://github.com/$Repo/releases" }

$artifact = 'dw-windows-amd64.exe'
Write-Host ""
Write-Host "Downloading dw v$version..."

$tmp = Join-Path ([IO.Path]::GetTempPath()) ([IO.Path]::GetRandomFileName())
New-Item -ItemType Directory -Path $tmp | Out-Null
$tmpExe = Join-Path $tmp 'dw.exe'

try {
    $base = "https://github.com/$Repo/releases/download/v$version"
    Invoke-WebRequest "$base/$artifact" -OutFile $tmpExe -UseBasicParsing

    # Verify against checksums.txt, which lists every platform's artifact.
    $checksums = (Invoke-WebRequest "$base/checksums.txt" -UseBasicParsing).Content
    $expected = ($checksums -split "`n" |
        Where-Object { $_ -match "\s$([regex]::Escape($artifact))$" } |
        ForEach-Object { ($_ -split '\s+')[0] }) | Select-Object -First 1

    if ($expected) {
        $actual = (Get-FileHash $tmpExe -Algorithm SHA256).Hash
        if ($actual -ne $expected.ToUpper()) { Fail 'Checksum verification failed.' }
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Move-Item $tmpExe (Join-Path $InstallDir 'dw.exe') -Force
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

# Add to the user PATH, writing through the registry rather than
# [Environment]::SetEnvironmentVariable. That API rewrites the value as a plain
# string, which permanently expands any %VAR% references already in the user's
# PATH. Reading and writing the key directly preserves them.
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment', $true)
try {
    $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    $current = ''
    if ($key.GetValueNames() -contains 'Path') {
        $kind = $key.GetValueKind('Path')
        $current = $key.GetValue('Path', '', 'DoNotExpandEnvironmentNames')
    }

    # Compare whole entries, ignoring case and any trailing slash, so a path
    # that merely contains ours does not count as a match.
    $entries = @($current -split ';' | Where-Object { $_ -ne '' })
    $target = $InstallDir.TrimEnd('\')
    $present = $entries | Where-Object { $_.TrimEnd('\') -ieq $target }

    if (-not $present) {
        $key.SetValue('Path', (($entries + $InstallDir) -join ';'), $kind)
        Write-Host "Added $InstallDir to your PATH. Restart your terminal to pick it up."
    }
} finally {
    if ($key) { $key.Close() }
}

$env:Path = "$env:Path;$InstallDir"

Write-Host ""
Write-Host "Installed dw v$version to $InstallDir" -ForegroundColor Green
Write-Host ""
Write-Host 'Get started:'
Write-Host '  dw login              # Authenticate via browser'
Write-Host '  dw login --api-key <KEY>  # Authenticate with an API key'
Write-Host '  dw --help             # See all commands'
