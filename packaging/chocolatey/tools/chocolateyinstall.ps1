$ErrorActionPreference = 'Stop'

# The binary is downloaded from the GitHub release rather than committed here,
# so the repo does not carry a ~9 MB file per version. Chocolatey checks the
# hash below, then shims any .exe in this folder, making dw.exe available as dw.

$version   = '0.1.25'
$toolsDir  = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64     = "https://github.com/doublewordai/dw/releases/download/v$version/dw-windows-amd64.exe"

# From checksums.txt on the release, entry `dw-windows-amd64.exe`.
$checksum64 = 'REPLACE_WITH_SHA256_FROM_RELEASE_CHECKSUMS'

Get-ChocolateyWebFile `
  -PackageName    'dw' `
  -FileFullPath   (Join-Path $toolsDir 'dw.exe') `
  -Url64bit       $url64 `
  -Checksum64     $checksum64 `
  -ChecksumType64 'sha256'
