param(
  [Parameter(Mandatory = $true)]
  [string]$ModelsUrl,

  [Parameter(Mandatory = $false)]
  [string]$OrtUrl,

  [Parameter(Mandatory = $false)]
  [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
)

$ErrorActionPreference = "Stop"

$modelsDir = Join-Path $RepoRoot "src-tauri\models"
$libsDir = Join-Path $RepoRoot "src-tauri\libs"
$manifestPath = Join-Path $RepoRoot "models-manifest.json"
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
  $modelsZip = Join-Path $tempDir "models.zip"
  Write-Host "Downloading models from $ModelsUrl"
  Invoke-WebRequest -Uri $ModelsUrl -OutFile $modelsZip

  $modelsExtracted = Join-Path $tempDir "models"
  Expand-ArchiveToDirectory -ArchivePath $modelsZip -DestinationPath $modelsExtracted
  $modelsSource = Get-ArchiveRoot -ExtractedDir $modelsExtracted
  Copy-DirectoryContent -SourceDir $modelsSource -DestinationDir $modelsDir

  if ($OrtUrl) {
    $ortExtension = [System.IO.Path]::GetExtension($OrtUrl)
    if ([string]::IsNullOrWhiteSpace($ortExtension)) {
      $ortExtension = ".zip"
    }
    $ortDownload = Join-Path $tempDir ("onnxruntime" + $ortExtension)
    Write-Host "Downloading ONNX Runtime from $OrtUrl"
    Invoke-WebRequest -Uri $OrtUrl -OutFile $ortDownload

    if ($OrtUrl.ToLowerInvariant().EndsWith(".dll")) {
      New-CleanDirectory -Path $libsDir
      Copy-Item -Path $ortDownload -Destination (Join-Path $libsDir "onnxruntime.dll") -Force
    } else {
      $ortExtracted = Join-Path $tempDir "ort"
      Expand-ArchiveToDirectory -ArchivePath $ortDownload -DestinationPath $ortExtracted
      $dll = Get-ChildItem -Path $ortExtracted -Filter "onnxruntime.dll" -Recurse | Select-Object -First 1
      if (-not $dll) {
        throw "Downloaded ONNX Runtime archive does not contain onnxruntime.dll"
      }
      New-CleanDirectory -Path $libsDir
      Copy-Item -Path $dll.FullName -Destination (Join-Path $libsDir "onnxruntime.dll") -Force
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
