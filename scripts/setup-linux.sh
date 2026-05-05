#!/usr/bin/env bash

set -euo pipefail

MODELS_URL=""
ORT_URL=""
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKIP_NPM_INSTALL=0
SKIP_CARGO_FETCH=0

usage() {
  cat <<'EOF'
Usage:
  bash scripts/setup-linux.sh --models-url <url> [--ort-url <url>] [--repo-root <path>] [--skip-npm-install] [--skip-cargo-fetch]

This script installs npm dependencies, prefetches cargo dependencies, downloads model assets,
and places the ONNX Runtime shared library into src-tauri/libs/.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --models-url)
      MODELS_URL="$2"
      shift 2
      ;;
    --ort-url)
      ORT_URL="$2"
      shift 2
      ;;
    --repo-root)
      REPO_ROOT="$2"
      shift 2
      ;;
    --skip-npm-install)
      SKIP_NPM_INSTALL=1
      shift
      ;;
    --skip-cargo-fetch)
      SKIP_CARGO_FETCH=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

MODELS_DIR="${REPO_ROOT}/src-tauri/models"
LIBS_DIR="${REPO_ROOT}/src-tauri/libs"
MANIFEST_PATH="${REPO_ROOT}/models-manifest.json"
RUNTIME_MANIFEST_PATH="${REPO_ROOT}/runtime-manifest.json"
TEMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "${TEMP_DIR}"
}
trap cleanup EXIT

assert_command() {
  local command="$1"
  local hint="$2"
  if ! command -v "${command}" >/dev/null 2>&1; then
    echo "missing required command: ${command}. ${hint}" >&2
    exit 1
  fi
}

clean_dir() {
  local path="$1"
  rm -rf "${path}"
  mkdir -p "${path}"
}

archive_root() {
  local extracted_dir="$1"
  local entries=()

  shopt -s nullglob dotglob
  entries=("${extracted_dir}"/*)
  shopt -u nullglob dotglob

  if [[ ${#entries[@]} -eq 1 && -d "${entries[0]}" ]]; then
    printf '%s\n' "${entries[0]}"
  else
    printf '%s\n' "${extracted_dir}"
  fi
}

extract_archive() {
  local archive_path="$1"
  local destination="$2"

  clean_dir "${destination}"

  case "${archive_path}" in
    *.zip)
      unzip -q "${archive_path}" -d "${destination}"
      ;;
    *.tar.gz|*.tgz)
      tar -xzf "${archive_path}" -C "${destination}"
      ;;
    *)
      echo "unsupported archive format: ${archive_path}" >&2
      exit 1
      ;;
  esac
}

copy_dir_contents() {
  local source_dir="$1"
  local destination_dir="$2"

  clean_dir "${destination_dir}"
  cp -a "${source_dir}/." "${destination_dir}/"
}

assert_command "npm" "Please install Node.js and ensure npm is on PATH."
assert_command "cargo" "Please install Rust via rustup and ensure cargo is on PATH."
assert_command "curl" "Please install curl."
assert_command "unzip" "Please install unzip."
assert_command "tar" "Please install tar."
assert_command "sha256sum" "Please install coreutils."

resolve_runtime_field() {
  local field_name="$1"
  if [[ ! -f "${RUNTIME_MANIFEST_PATH}" ]]; then
    return 0
  fi

  node -e '
const fs = require("fs");
const manifestPath = process.argv[1];
const platformKey = process.argv[2];
const fieldName = process.argv[3];
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const platform = manifest.platforms?.[platformKey];
if (platform && platform[fieldName]) {
  process.stdout.write(String(platform[fieldName]));
}
' "${RUNTIME_MANIFEST_PATH}" "linux-x64" "${field_name}"
}

resolve_models_field() {
  local field_name="$1"
  if [[ ! -f "${MANIFEST_PATH}" ]]; then
    return 0
  fi

  node -e '
const fs = require("fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const distribution = manifest.distribution;
if (distribution && distribution[process.argv[2]]) {
  process.stdout.write(String(distribution[process.argv[2]]));
}
' "${MANIFEST_PATH}" "${field_name}"
}

if [[ "${SKIP_NPM_INSTALL}" -eq 0 ]]; then
  echo "Installing npm dependencies"
  (cd "${REPO_ROOT}" && npm install)
fi

if [[ "${SKIP_CARGO_FETCH}" -eq 0 ]]; then
  echo "Fetching cargo dependencies"
  cargo fetch --manifest-path "${REPO_ROOT}/src-tauri/Cargo.toml"
fi

if [[ -z "${MODELS_URL}" ]]; then
  MODELS_URL="$(resolve_models_field artifactUrl)"
fi

MODELS_SHA256="$(resolve_models_field artifactSha256)"

if [[ -z "${MODELS_URL}" ]]; then
  echo "models URL is not configured. Provide --models-url or set distribution.artifactUrl in models-manifest.json" >&2
  exit 1
fi

MODELS_ARCHIVE="${TEMP_DIR}/models.zip"
echo "Downloading models from ${MODELS_URL}"
curl -L --fail --output "${MODELS_ARCHIVE}" "${MODELS_URL}"

if [[ -n "${MODELS_SHA256}" ]]; then
  DOWNLOAD_SHA="$(sha256sum "${MODELS_ARCHIVE}" | awk '{print $1}')"
  if [[ "${DOWNLOAD_SHA}" != "${MODELS_SHA256,,}" ]]; then
    echo "Model artifact SHA256 mismatch. expected=${MODELS_SHA256} actual=${DOWNLOAD_SHA}" >&2
    exit 1
  fi
  echo "Model artifact checksum verification passed."
fi

MODELS_EXTRACTED="${TEMP_DIR}/models"
extract_archive "${MODELS_ARCHIVE}" "${MODELS_EXTRACTED}"
MODELS_SOURCE="$(archive_root "${MODELS_EXTRACTED}")"
copy_dir_contents "${MODELS_SOURCE}" "${MODELS_DIR}"

if [[ -z "${ORT_URL}" ]]; then
  ORT_URL="$(resolve_runtime_field artifactUrl)"
fi

RUNTIME_LIBRARY_NAME="$(resolve_runtime_field libraryName)"
if [[ -z "${RUNTIME_LIBRARY_NAME}" ]]; then
  RUNTIME_LIBRARY_NAME="libonnxruntime.so"
fi

RUNTIME_SHA256="$(resolve_runtime_field sha256)"

if [[ -n "${ORT_URL}" ]]; then
  ORT_DOWNLOAD="${TEMP_DIR}/onnxruntime$(basename "${ORT_URL}")"
  echo "Downloading ONNX Runtime from ${ORT_URL}"
  curl -L --fail --output "${ORT_DOWNLOAD}" "${ORT_URL}"

  if [[ -n "${RUNTIME_SHA256}" ]]; then
    DOWNLOAD_SHA="$(sha256sum "${ORT_DOWNLOAD}" | awk '{print $1}')"
    if [[ "${DOWNLOAD_SHA}" != "${RUNTIME_SHA256,,}" ]]; then
      echo "ONNX Runtime artifact SHA256 mismatch. expected=${RUNTIME_SHA256} actual=${DOWNLOAD_SHA}" >&2
      exit 1
    fi
    echo "Runtime artifact checksum verification passed."
  fi

  clean_dir "${LIBS_DIR}"

  case "${ORT_DOWNLOAD}" in
    *.so|*.dylib)
      cp "${ORT_DOWNLOAD}" "${LIBS_DIR}/${RUNTIME_LIBRARY_NAME}"
      ;;
    *.zip|*.tar.gz|*.tgz)
      ORT_EXTRACTED="${TEMP_DIR}/ort"
      extract_archive "${ORT_DOWNLOAD}" "${ORT_EXTRACTED}"
      ORT_LIB="$(find "${ORT_EXTRACTED}" -type f -name "${RUNTIME_LIBRARY_NAME}" | head -n 1)"
      if [[ -z "${ORT_LIB}" ]]; then
        echo "downloaded ONNX Runtime archive does not contain ${RUNTIME_LIBRARY_NAME}" >&2
        exit 1
      fi
      cp "${ORT_LIB}" "${LIBS_DIR}/${RUNTIME_LIBRARY_NAME}"
      ;;
    *)
      echo "unsupported ONNX Runtime artifact: ${ORT_URL}" >&2
      exit 1
      ;;
  esac
fi

if [[ -f "${MANIFEST_PATH}" ]]; then
  mapfile -t files_to_check < <(node -e '
const fs = require("fs");
const manifest = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
for (const entry of manifest.files || []) {
  if (entry.sha256) {
    console.log(`${entry.name}\t${String(entry.sha256).toLowerCase()}`);
  }
}
' "${MANIFEST_PATH}")

  if [[ ${#files_to_check[@]} -gt 0 ]]; then
    for entry in "${files_to_check[@]}"; do
      file_name="${entry%%$'\t'*}"
      expected_sha="${entry#*$'\t'}"
      target="${MODELS_DIR}/${file_name}"

      if [[ ! -f "${target}" ]]; then
        echo "missing model file after extraction: ${file_name}" >&2
        exit 1
      fi

      actual_sha="$(sha256sum "${target}" | awk '{print $1}')"
      if [[ "${actual_sha}" != "${expected_sha}" ]]; then
        echo "SHA256 mismatch for ${file_name}. expected=${expected_sha} actual=${actual_sha}" >&2
        exit 1
      fi
    done
    echo "Model checksum verification passed."
  else
    echo "models-manifest.json has no SHA256 entries yet, skipping verification."
  fi
fi

echo "Linux setup completed."
echo "Models directory: ${MODELS_DIR}"
if [[ -d "${LIBS_DIR}" ]]; then
  echo "Runtime library directory: ${LIBS_DIR}"
fi
