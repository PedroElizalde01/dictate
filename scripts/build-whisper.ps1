param(
  [string]$RepoUrl = "https://github.com/ggerganov/whisper.cpp",
  [string]$BuildType = "Release",
  [switch]$Vulkan
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$BuildDir = Join-Path $Root ".whisper-build"
$WhisperDir = Join-Path $BuildDir "whisper.cpp"
$BinDir = Join-Path $Root "src-tauri\binaries"
$UseVulkan = $Vulkan.IsPresent -or $env:VULKAN -eq "1"

function Invoke-Native {
  param(
    [Parameter(Mandatory = $true)][string]$FilePath,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
  )
  & $FilePath @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$FilePath exited with code $LASTEXITCODE"
  }
}

New-Item -ItemType Directory -Force -Path $BuildDir, $BinDir | Out-Null

if (!(Test-Path (Join-Path $WhisperDir ".git"))) {
  Invoke-Native git clone --depth 1 $RepoUrl $WhisperDir
} else {
  Invoke-Native git -C $WhisperDir pull --ff-only
}

$CmakeArgs = @(
  "-B", "build",
  "-DCMAKE_BUILD_TYPE=$BuildType",
  "-DBUILD_SHARED_LIBS=OFF",
  "-DWHISPER_BUILD_TESTS=OFF",
  "-DWHISPER_BUILD_EXAMPLES=ON"
)

if ($UseVulkan) {
  $CmakeArgs += "-DGGML_VULKAN=ON"
}

Push-Location $WhisperDir
try {
  Invoke-Native cmake @CmakeArgs
  Invoke-Native cmake --build build --config $BuildType --parallel

  $Candidates = @(
    "build\bin\$BuildType\whisper-cli.exe",
    "build\bin\whisper-cli.exe",
    "build\examples\$BuildType\whisper-cli.exe",
    "build\examples\whisper-cli.exe",
    "build\whisper-cli.exe"
  )
  $Source = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
  if (!$Source) {
    throw "whisper-cli.exe was not found after build."
  }

  $PlainDest = Join-Path $BinDir "whisper-cli.exe"
  Copy-Item -Force $Source $PlainDest
  Write-Host "Installed: $PlainDest"

  $Rustc = Get-Command rustc -ErrorAction SilentlyContinue
  if ($Rustc) {
    $HostInfo = & rustc -vV
    if ($LASTEXITCODE -ne 0) {
      throw "rustc exited with code $LASTEXITCODE"
    }
    $HostLine = $HostInfo | Select-String "^host:"
    if ($HostLine) {
      $Triple = ($HostLine.ToString() -replace "^host:\s*", "").Trim()
      $SidecarDest = Join-Path $BinDir "whisper-cli-$Triple.exe"
      Copy-Item -Force $PlainDest $SidecarDest
      Write-Host "Sidecar: $SidecarDest"
    }
  } else {
    Write-Warning "rustc was not found on PATH; skipped target-triple sidecar copy."
  }
} finally {
  Pop-Location
}
