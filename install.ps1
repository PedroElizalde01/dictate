param(
  [string]$RepoUrl = $env:REPO_URL,
  [string]$InstallDir = $env:INSTALL_DIR,
  [string]$Branch = $env:BRANCH,
  [switch]$Vulkan
)

$ErrorActionPreference = "Stop"

if (!$RepoUrl) { $RepoUrl = "https://github.com/USERNAME/dictate.git" }
if (!$InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA "dictate-src" }
if (!$Branch) { $Branch = "main" }
$UseVulkan = $Vulkan.IsPresent -or $env:VULKAN -eq "1"

function Info($Message) { Write-Host "==> $Message" -ForegroundColor Cyan }
function Done($Message) { Write-Host "OK  $Message" -ForegroundColor Green }
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
function Update-SessionPath {
  $MachinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $CargoPath = Join-Path $env:USERPROFILE ".cargo\bin"
  $env:Path = "$CargoPath;$MachinePath;$UserPath"
}
function Need-Command($Name, $PackageId) {
  if (Get-Command $Name -ErrorAction SilentlyContinue) {
    Done "$Name already installed"
    return
  }
  if (!(Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "$Name is missing and winget is not available. Install $PackageId manually."
  }
  Info "Installing $PackageId"
  Invoke-Native winget install --id $PackageId --exact --accept-package-agreements --accept-source-agreements
  Update-SessionPath
}
function Test-MsvcBuildTools {
  if (Get-Command cl -ErrorAction SilentlyContinue) {
    return $true
  }
  $VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
  if (Test-Path $VsWhere) {
    $Install = & $VsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return ![string]::IsNullOrWhiteSpace($Install)
  }
  return $false
}

$Caption = (Get-CimInstance Win32_OperatingSystem).Caption
if ($Caption -notmatch "Windows 10|Windows 11") {
  Write-Warning "This installer is tested for Windows 10 and Windows 11. Detected: $Caption"
}

Need-Command git "Git.Git"
Need-Command rustup "Rustlang.Rustup"
Need-Command node "OpenJS.NodeJS.LTS"
Need-Command cmake "Kitware.CMake"

if (!(Test-MsvcBuildTools)) {
  if (!(Get-Command winget -ErrorAction SilentlyContinue)) {
    throw "MSVC Build Tools are missing and winget is not available."
  }
  Info "Installing Visual Studio 2022 Build Tools with C++ workload"
  Invoke-Native winget install --id Microsoft.VisualStudio.2022.BuildTools --exact `
    --accept-package-agreements --accept-source-agreements `
    --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
  Update-SessionPath
}

if ($UseVulkan) {
  Need-Command glslc "LunarG.VulkanSDK"
}

if (Test-Path (Join-Path $InstallDir ".git")) {
  Info "Updating checkout at $InstallDir"
  Invoke-Native git -C $InstallDir fetch --depth 1 origin $Branch
  Invoke-Native git -C $InstallDir checkout $Branch
  Invoke-Native git -C $InstallDir reset --hard "origin/$Branch"
} else {
  Info "Cloning $RepoUrl to $InstallDir"
  New-Item -ItemType Directory -Force -Path (Split-Path $InstallDir -Parent) | Out-Null
  Invoke-Native git clone --depth 1 --branch $Branch $RepoUrl $InstallDir
}

Set-Location $InstallDir

Info "Building whisper.cpp"
if ($UseVulkan) {
  & .\scripts\build-whisper.ps1 -Vulkan
} else {
  & .\scripts\build-whisper.ps1
}

Info "Installing JavaScript dependencies"
Invoke-Native npm.cmd install --no-audit --no-fund

Info "Building Dictate"
Invoke-Native npm.cmd run tauri build

$Bundles = Get-ChildItem .\src-tauri\target\release\bundle -Recurse -File -Include *.msi,*.exe |
  Sort-Object LastWriteTime -Descending
$Installer = $Bundles | Select-Object -First 1
if ($Installer) {
  Info "Starting installer $($Installer.FullName)"
  Start-Process -FilePath $Installer.FullName -Wait
} else {
  Write-Warning "No Windows installer was found under src-tauri\target\release\bundle."
}

Done "Dictate build complete"
Write-Host "Source: $InstallDir"
Write-Host "Settings: $env:APPDATA\dictate\settings.json"
Write-Host "Models: $env:LOCALAPPDATA\dictate\models"
