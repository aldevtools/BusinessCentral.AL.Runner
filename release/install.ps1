# Installs al-runner from GitHub releases (Windows x64).
#   install.ps1 [-Version <edge | latest-alpha | v3.0.0-alpha.2>]
# Default is latest-alpha. Environment: ALRUNNER_INSTALL_DIR (default %LOCALAPPDATA%\alrunner\bin),
# ALRUNNER_REPO (default aldevtools/BusinessCentral.AL.Runner).
param([string]$Version = "latest-alpha")
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$repo = if ($env:ALRUNNER_REPO) { $env:ALRUNNER_REPO } else { "aldevtools/BusinessCentral.AL.Runner" }
$dir = if ($env:ALRUNNER_INSTALL_DIR) { $env:ALRUNNER_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "alrunner\bin" }
$target = "x86_64-pc-windows-msvc"

switch -Regex ($Version) {
  '^edge$' { $tag = "edge"; $label = "edge" }
  '^latest-alpha$' {
    $releases = Invoke-RestMethod "https://api.github.com/repos/$repo/releases?per_page=30"
    $first = $releases | Where-Object { $_.tag_name -ne "edge" -and -not $_.draft } | Select-Object -First 1
    if (-not $first) { throw "No numbered alpha release found in $repo" }
    $tag = $first.tag_name; $label = $tag.TrimStart("v")
  }
  '^v' { $tag = $Version; $label = $Version.TrimStart("v") }
  default { $tag = "v$Version"; $label = $Version }
}

$base = "https://github.com/$repo/releases/download/$tag"
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("al-runner-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Invoke-WebRequest "$base/SHA256SUMS" -OutFile (Join-Path $tmp "SHA256SUMS")
  $sums = Get-Content (Join-Path $tmp "SHA256SUMS")
  # The binary is al-runner. Releases from before the rename (up to 3.0.0-alpha.2) hold alrunner.
  $exe = "al-runner"
  if (-not ($sums | Where-Object { $_ -match [regex]::Escape(" al-runner-$label-$target.zip") + '$' }) -and
      ($sums | Where-Object { $_ -match [regex]::Escape(" alrunner-$label-$target.zip") + '$' })) { $exe = "alrunner" }
  $archive = "$exe-$label-$target.zip"
  Write-Host "Installing $exe $tag for $target"
  Invoke-WebRequest "$base/$archive" -OutFile (Join-Path $tmp $archive)
  $line = $sums | Where-Object { $_ -match [regex]::Escape($archive) + '$' } | Select-Object -First 1
  if (-not $line) { throw "$archive is not in SHA256SUMS" }
  $expected = ($line -split '\s+')[0].ToLower()
  $actual = (Get-FileHash (Join-Path $tmp $archive) -Algorithm SHA256).Hash.ToLower()
  if ($expected -ne $actual) { throw "Checksum mismatch for $archive" }

  Expand-Archive (Join-Path $tmp $archive) -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
  Copy-Item (Join-Path $tmp "$exe.exe") (Join-Path $dir "$exe.exe") -Force
  # The binary is signed, but a new signing certificate can still trigger SmartScreen until it
  # has reputation. Remove the download mark so SmartScreen does not block it.
  Unblock-File (Join-Path $dir "$exe.exe")
  & (Join-Path $dir "$exe.exe") --version
  $pathDirs = $env:PATH -split ';'
  if ($pathDirs -notcontains $dir) { Write-Host "Add $dir to your PATH." }

  $old = Join-Path $dir "alrunner.exe"
  if ($exe -eq "al-runner" -and (Test-Path $old)) {
    Write-Host "note: $old is an older build under the old name. Remove it: Remove-Item `"$old`""
  }
  # AL Runner v2's dotnet tool is also called al-runner. Whichever comes first on PATH runs.
  $v2dir = Join-Path $env:USERPROFILE ".dotnet\tools"
  if ($exe -eq "al-runner" -and (Test-Path (Join-Path $v2dir "al-runner.exe"))) {
    Write-Host "note: AL Runner v2 is installed at $(Join-Path $v2dir 'al-runner.exe'), a command of the same name."
    Write-Host "  Remove it with `"dotnet tool uninstall -g MSDyn365BC.AL.Runner`" if you no longer need it."
    $first = $pathDirs | Where-Object { $_ -eq $dir -or $_ -eq $v2dir } | Select-Object -First 1
    if ($first -eq $v2dir) {
      Write-Host "  It comes first on your PATH now, so `"al-runner`" starts v2. To keep both, put $dir before $v2dir on PATH."
    }
  }
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
