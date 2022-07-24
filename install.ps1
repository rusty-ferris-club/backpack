#!/usr/bin/env pwsh
# Copyright 2018 the Deno authors. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

$ErrorActionPreference = 'Stop'

if ($v) {
  $Version = "v${v}"
}
if ($args.Length -eq 1) {
  $Version = $args.Get(0)
}

$Install = $env:BP_INSTALL
$BinDir = if ($Install) {
  "$Install\bin"
} else {
  "$Home\.backpack-bin\bin"
}

$Zip = "$BinDir\backpack.zip"
$Exe = "$BinDir\bp.exe"
$Target = 'x86_64-windows'

# GitHub requires TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Uri = if (!$Version) {
  "https://github.com/rusty-ferris-club/backpack/releases/latest/download/backpack-${Target}.zip"
} else {
  "https://github.com/rusty-ferris-club/backpack/releases/download/${Version}/backpack-${Target}.zip"
}

if (!(Test-Path $BinDir)) {
  New-Item $BinDir -ItemType Directory | Out-Null
}

curl.exe -Lo $Zip $Uri

tar.exe xf $Zip -C $BinDir

Remove-Item $Zip

$User = [EnvironmentVariableTarget]::User
$Path = [Environment]::GetEnvironmentVariable('Path', $User)
if (!(";$Path;".ToLower() -like "*;$BinDir;*".ToLower())) {
  [Environment]::SetEnvironmentVariable('Path', "$Path;$BinDir", $User)
  $Env:Path += ";$BinDir"
}

Write-Output "Backpack was installed successfully to $Exe"
Write-Output "Run 'bp --help' to get started"
