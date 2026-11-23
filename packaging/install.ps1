# Installs sdc from a GitHub release on Windows:
#   powershell -ExecutionPolicy Bypass -c "irm https://github.com/__REPO__/releases/download/__TAG__/sdc-installer.ps1 | iex"
# SDC_INSTALL_DIR picks the directory (default %LOCALAPPDATA%\Programs\sdc), which is added
# to the user's PATH. The release workflow fills in the repository and tag; SDC_REPO and
# SDC_TAG override them.
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo = if ($env:SDC_REPO) { $env:SDC_REPO } else { '__REPO__' }
$tag = if ($env:SDC_TAG) { $env:SDC_TAG } else { '__TAG__' }
$dir = if ($env:SDC_INSTALL_DIR) { $env:SDC_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\sdc' }

if ("$repo$tag".Contains('__')) {
  throw 'sdc installer: set SDC_REPO=owner/name and SDC_TAG=vX.Y.Z, or use the installer attached to a release'
}

# PROCESSOR_ARCHITEW6432 is set when a 32-bit or emulated shell runs on a 64-bit system.
$machine = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$arch = switch ($machine) {
  'AMD64' { 'x86_64' }
  'ARM64' { 'aarch64' }
  default { throw "sdc installer: unsupported processor $machine" }
}

$archive = "sdc-$arch-pc-windows-msvc.zip"
$url = "https://github.com/$repo/releases/download/$tag/$archive"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ([Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Host "Downloading $url"
  Invoke-WebRequest -Uri $url -OutFile (Join-Path $tmp $archive) -UseBasicParsing
  Invoke-WebRequest -Uri "$url.sha256" -OutFile (Join-Path $tmp "$archive.sha256") -UseBasicParsing
  $expected = (Get-Content (Join-Path $tmp "$archive.sha256") -Raw).Split(' ')[0].Trim().ToLower()
  $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $tmp $archive)).Hash.ToLower()
  if ($expected -ne $actual) { throw "sdc installer: checksum mismatch for $archive" }

  Expand-Archive -Path (Join-Path $tmp $archive) -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  Copy-Item (Join-Path $tmp 'sdc.exe') (Join-Path $dir 'sdc.exe') -Force
  $version = & (Join-Path $dir 'sdc.exe') --version
  Write-Host "Installed $version to $dir\sdc.exe"
} finally {
  Remove-Item -Recurse -Force $tmp
}

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if (-not (($userPath -split ';') -contains $dir)) {
  [Environment]::SetEnvironmentVariable('Path', ($(if ($userPath) { "$userPath;" } else { '' }) + $dir), 'User')
  Write-Host "Added $dir to your user PATH; open a new terminal to use sdc."
}
