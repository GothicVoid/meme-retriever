param(
  [Parameter(Mandatory = $false)]
  [string]$ModelsUrl,

  [Parameter(Mandatory = $false)]
  [string]$OrtUrl,

  [Parameter(Mandatory = $false)]
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,

  [Parameter(Mandatory = $false)]
  [switch]$SkipNpmInstall,

  [Parameter(Mandatory = $false)]
  [switch]$SkipCargoFetch
)

$ErrorActionPreference = "Stop"

$modelsDir = Join-Path $RepoRoot "src-tauri\models"
$libsDir = Join-Path $RepoRoot "src-tauri\libs"
$manifestPath = Join-Path $RepoRoot "models-manifest.json"
$runtimeManifestPath = Join-Path $RepoRoot "runtime-manifest.json"
$tempDir = Join-Path $env:TEMP ("meme-retriever-setup-" + [guid]::NewGuid().ToString("N"))

function New-CleanDirectory {
  param([string]$Path)

  if (Test-Path $Path) {
    Remove-Item -Path $Path -Recurse -Force
  }
  New-Item -ItemType Directory -Path $Path | Out-Null
}

function Get-Sha256 {
  param([string]$Path)

  return (Get-FileHash -Algorithm SHA256 -Path $Path).Hash.ToLowerInvariant()
}

function Assert-CommandExists {
  param(
    [string]$Command,
    [string]$InstallHint
  )

  if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
    throw "Missing required command: $Command. $InstallHint"
  }
}

function Resolve-RuntimeConfig {
  param(
    [string]$ManifestPath,
    [string]$ExplicitUrl
  )

  $config = [ordered]@{
    Url = $ExplicitUrl
    LibraryName = "onnxruntime.dll"
    Sha256 = $null
  }

  if (-not (Test-Path $ManifestPath)) {
    return $config
  }

  $manifest = Get-Content -Raw -Path $ManifestPath | ConvertFrom-Json
  $platform = $manifest.platforms."windows-x64"
  if (-not $platform) {
    return $config
  }

  if (-not $config.Url) {
    $config.Url = $platform.artifactUrl
  }
  if ($platform.libraryName) {
    $config.LibraryName = $platform.libraryName
  }
  if ($platform.sha256) {
    $config.Sha256 = $platform.sha256.ToLowerInvariant()
  }

  return $config
}

function Resolve-ModelsConfig {
  param(
    [string]$ManifestPath,
    [string]$ExplicitUrl
  )

  $config = [ordered]@{
    Url = $ExplicitUrl
    Sha256 = $null
  }

  if (-not (Test-Path $ManifestPath)) {
    return $config
  }

  $manifest = Get-Content -Raw -Path $ManifestPath | ConvertFrom-Json
  $distribution = $manifest.distribution
  if (-not $distribution) {
    return $config
  }

  if (-not $config.Url) {
    $config.Url = $distribution.artifactUrl
  }
  if ($distribution.artifactSha256) {
    $config.Sha256 = $distribution.artifactSha256.ToLowerInvariant()
  }

  return $config
}

function Expand-ArchiveToDirectory {
  param(
    [string]$ArchivePath,
    [string]$DestinationPath
  )

  New-CleanDirectory -Path $DestinationPath
  Expand-Archive -Path $ArchivePath -DestinationPath $DestinationPath -Force
}

function Copy-DirectoryContent {
  param(
    [string]$SourceDir,
    [string]$DestinationDir
  )

  New-CleanDirectory -Path $DestinationDir
  Get-ChildItem -Path $SourceDir | ForEach-Object {
    Copy-Item -Path $_.FullName -Destination $DestinationDir -Recurse -Force
  }
}

function Get-ArchiveRoot {
  param([string]$ExtractedDir)

  $children = @(Get-ChildItem -Path $ExtractedDir)
  if ($children.Count -eq 1 -and $children[0].PSIsContainer) {
    return $children[0].FullName
  }

  return $ExtractedDir
}

New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
  Assert-CommandExists -Command "npm" -InstallHint "Please install Node.js and ensure npm is on PATH."
  Assert-CommandExists -Command "cargo" -InstallHint "Please install Rust via rustup and ensure cargo is on PATH."

  if (-not $SkipNpmInstall) {
    Write-Host "Installing npm dependencies"
    & npm install
    if ($LASTEXITCODE -ne 0) {
      throw "npm install failed"
    }
  }

  if (-not $SkipCargoFetch) {
    Write-Host "Fetching cargo dependencies"
    & cargo fetch --manifest-path (Join-Path $RepoRoot "src-tauri\Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
      throw "cargo fetch failed"
    }
  }

  $modelsConfig = Resolve-ModelsConfig -ManifestPath $manifestPath -ExplicitUrl $ModelsUrl
  if (-not $modelsConfig.Url) {
    throw "Models URL is not configured. Provide -ModelsUrl or set distribution.artifactUrl in models-manifest.json"
  }

  $modelsZip = Join-Path $tempDir "models.zip"
  Write-Host "Downloading models from $($modelsConfig.Url)"
  Invoke-WebRequest -Uri $modelsConfig.Url -OutFile $modelsZip

  if ($modelsConfig.Sha256) {
    $modelsSha = Get-Sha256 -Path $modelsZip
    if ($modelsSha -ne $modelsConfig.Sha256) {
      throw "Model artifact SHA256 mismatch. expected=$($modelsConfig.Sha256) actual=$modelsSha"
    }
    Write-Host "Model artifact checksum verification passed."
  }

  $modelsExtracted = Join-Path $tempDir "models"
  Expand-ArchiveToDirectory -ArchivePath $modelsZip -DestinationPath $modelsExtracted
  $modelsSource = Get-ArchiveRoot -ExtractedDir $modelsExtracted
  Copy-DirectoryContent -SourceDir $modelsSource -DestinationDir $modelsDir

  $runtimeConfig = Resolve-RuntimeConfig -ManifestPath $runtimeManifestPath -ExplicitUrl $OrtUrl
  if ($runtimeConfig.Url) {
    $ortExtension = [System.IO.Path]::GetExtension($runtimeConfig.Url)
    if ([string]::IsNullOrWhiteSpace($ortExtension)) {
      $ortExtension = ".zip"
    }
    $ortDownload = Join-Path $tempDir ("onnxruntime" + $ortExtension)
    Write-Host "Downloading ONNX Runtime from $($runtimeConfig.Url)"
    Invoke-WebRequest -Uri $runtimeConfig.Url -OutFile $ortDownload

    if ($runtimeConfig.Sha256) {
      $downloadSha = Get-Sha256 -Path $ortDownload
      if ($downloadSha -ne $runtimeConfig.Sha256) {
        throw "ONNX Runtime artifact SHA256 mismatch. expected=$($runtimeConfig.Sha256) actual=$downloadSha"
      }
      Write-Host "Runtime artifact checksum verification passed."
    }

    if ($runtimeConfig.Url.ToLowerInvariant().EndsWith(".dll")) {
      New-CleanDirectory -Path $libsDir
      Copy-Item -Path $ortDownload -Destination (Join-Path $libsDir $runtimeConfig.LibraryName) -Force
    } else {
      $ortExtracted = Join-Path $tempDir "ort"
      Expand-ArchiveToDirectory -ArchivePath $ortDownload -DestinationPath $ortExtracted
      $dll = Get-ChildItem -Path $ortExtracted -Filter $runtimeConfig.LibraryName -Recurse | Select-Object -First 1
      if (-not $dll) {
        throw "Downloaded ONNX Runtime archive does not contain $($runtimeConfig.LibraryName)"
      }
      New-CleanDirectory -Path $libsDir
      Copy-Item -Path $dll.FullName -Destination (Join-Path $libsDir $runtimeConfig.LibraryName) -Force
    }
  }

  if (Test-Path $manifestPath) {
    $manifest = Get-Content -Raw -Path $manifestPath | ConvertFrom-Json
    $checks = @($manifest.files | Where-Object { $_.sha256 })
    if ($checks.Count -gt 0) {
      foreach ($entry in $checks) {
        $target = Join-Path $modelsDir $entry.name
        if (-not (Test-Path $target)) {
          throw "Missing model file after extraction: $($entry.name)"
        }

        $actual = Get-Sha256 -Path $target
        if ($actual -ne $entry.sha256.ToLowerInvariant()) {
          throw "SHA256 mismatch for $($entry.name). expected=$($entry.sha256) actual=$actual"
        }
      }
      Write-Host "Model checksum verification passed."
    } else {
      Write-Host "models-manifest.json has no SHA256 entries yet, skipping verification."
    }
  }

  Write-Host "Windows setup completed."
  Write-Host "Models directory: $modelsDir"
  if (Test-Path $libsDir) {
    Write-Host "Runtime library directory: $libsDir"
  }
} finally {
  if (Test-Path $tempDir) {
    Remove-Item -Path $tempDir -Recurse -Force
  }
}
