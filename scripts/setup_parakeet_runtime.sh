#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
VENV_DIR="${PROJECT_ROOT}/.venv311"
PYTHON_BIN="${VENV_DIR}/bin/python3"
REQ_FILE="${PROJECT_ROOT}/requirements/parakeet-runtime.txt"

if [[ ! -d "${VENV_DIR}" ]]; then
  if ! command -v python3.11 >/dev/null 2>&1; then
    echo "python3.11 is required to create .venv311" >&2
    exit 1
  fi
  python3.11 -m venv "${VENV_DIR}"
fi

if ! "${PYTHON_BIN}" -m pip --version >/dev/null 2>&1; then
  "${PYTHON_BIN}" -m ensurepip --upgrade
fi

"${PYTHON_BIN}" -m pip install --upgrade pip
"${PYTHON_BIN}" -m pip install -r "${REQ_FILE}"

echo "Parakeet runtime dependencies installed."
echo "PARAKEET_PYTHON_BIN=\"${PYTHON_BIN}\""
