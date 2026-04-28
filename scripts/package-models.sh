#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODELS_DIR="${ROOT_DIR}/src-tauri/models"
OUTPUT_DIR="${ROOT_DIR}/release-assets/models"
VERSION="${1:-$(date +%Y.%m.%d)}"

TEXT_MODEL_CANDIDATES=(
  "clip_text.onnx"
  "vit-b-16.txt.fp32.onnx"
  "vit-b-16.txt.fp16.onnx"
)

IMAGE_MODEL_CANDIDATES=(
  "clip_image.onnx"
  "vit-b-16.img.fp32.onnx"
  "vit-b-16.img.fp16.onnx"
)

REQUIRED_STATIC_FILES=(
  "vocab.txt"
  "ch_PP-OCRv4_det_infer.onnx"
  "ch_PP-OCRv4_rec_infer.onnx"
  "ppocr_keys_v1.txt"
)

if [[ ! -d "${MODELS_DIR}" ]]; then
  echo "missing models directory: ${MODELS_DIR}" >&2
  exit 1
fi

pick_existing_file() {
  local file
  for file in "$@"; do
    if [[ -f "${MODELS_DIR}/${file}" ]]; then
      printf '%s\n' "${file}"
      return 0
    fi
  done
  return 1
}

append_if_exists() {
  local file="$1"
  if [[ -f "${MODELS_DIR}/${file}" ]]; then
    PACKAGE_FILES+=("${file}")
  fi
}

TEXT_MODEL="$(pick_existing_file "${TEXT_MODEL_CANDIDATES[@]}")" || {
  echo "missing text model, expected one of: ${TEXT_MODEL_CANDIDATES[*]}" >&2
  exit 1
}

IMAGE_MODEL="$(pick_existing_file "${IMAGE_MODEL_CANDIDATES[@]}")" || {
  echo "missing image model, expected one of: ${IMAGE_MODEL_CANDIDATES[*]}" >&2
  exit 1
}

PACKAGE_FILES=("${TEXT_MODEL}" "${IMAGE_MODEL}")

for file in "${REQUIRED_STATIC_FILES[@]}"; do
  if [[ ! -f "${MODELS_DIR}/${file}" ]]; then
    echo "missing required model file: ${file}" >&2
    exit 1
  fi
  PACKAGE_FILES+=("${file}")
done

append_if_exists "${TEXT_MODEL}.data"
append_if_exists "${TEXT_MODEL}.extra_file"
append_if_exists "${IMAGE_MODEL}.data"
append_if_exists "${IMAGE_MODEL}.extra_file"

mkdir -p "${OUTPUT_DIR}"

MANIFEST_PATH="${ROOT_DIR}/models-manifest.json"
ZIP_NAME="meme-retriever-models-${VERSION}.zip"
ZIP_PATH="${OUTPUT_DIR}/${ZIP_NAME}"
CHECKSUMS_PATH="${OUTPUT_DIR}/SHA256SUMS.txt"

generated_at="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

manifest_json=$(
  {
    printf '{\n'
    printf '  "version": "%s",\n' "${VERSION}"
    printf '  "generatedAt": "%s",\n' "${generated_at}"
    printf '  "selectedModels": {\n'
    printf '    "text": "%s",\n' "${TEXT_MODEL}"
    printf '    "image": "%s"\n' "${IMAGE_MODEL}"
    printf '  },\n'
    printf '  "files": [\n'

    for i in "${!PACKAGE_FILES[@]}"; do
      file="${PACKAGE_FILES[$i]}"
      sha="$(sha256sum "${MODELS_DIR}/${file}" | awk '{print $1}')"
      printf '    {\n'
      printf '      "name": "%s",\n' "${file}"
      printf '      "sha256": "%s"\n' "${sha}"
      if [[ "${i}" -lt $((${#PACKAGE_FILES[@]} - 1)) ]]; then
        printf '    },\n'
      else
        printf '    }\n'
      fi
    done

    printf '  ]\n'
    printf '}\n'
  }
)

printf '%s' "${manifest_json}" > "${MANIFEST_PATH}"
printf '%s' "${manifest_json}" > "${OUTPUT_DIR}/models-manifest.json"

rm -f "${ZIP_PATH}"
(
  cd "${MODELS_DIR}"
  zip -q "${ZIP_PATH}" "${PACKAGE_FILES[@]}"
)

(
  cd "${OUTPUT_DIR}"
  sha256sum "$(basename "${ZIP_PATH}")" "models-manifest.json" > "${CHECKSUMS_PATH}"
)

echo "model package created:"
echo "  manifest: ${MANIFEST_PATH}"
echo "  archive:  ${ZIP_PATH}"
echo "  checksums:${CHECKSUMS_PATH}"
