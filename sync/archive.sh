#!/bin/bash

ARCHIVE_TARGET_DIR="/mnt/storage/TOSHIBA_2"

COMMON_BASE_DIR="/mnt/storage/cash_1"

# 強制移動フォルダ
FORCE_MOVE_PATHS=(
    "30_backup"
)

# 条件適用フォルダ
CONDITIONAL_SCAN_PATHS=(
    "."
)

# 条件
IMMEDIATE_MOVE_SIZE_MB=1024
RULE_DAYS_OLD=7
RULE_SIZE_MB=15

echo "Archive process started at $(date)..."
echo "Archive destination: ${ARCHIVE_TARGET_DIR}"
echo "Common base directory: ${COMMON_BASE_DIR}"

if [ ! -d "${COMMON_BASE_DIR}" ]; then
  echo "Error: Common base directory not found: ${COMMON_BASE_DIR}"
  exit 1
fi
# 強制処理
echo "[Phase 1] Processing force-move paths (preserving structure)..."
(
  cd "${COMMON_BASE_DIR}" && \
  
  for path in "${FORCE_MOVE_PATHS[@]}"; do
    if [ -d "${path}" ]; then
      echo "Scanning (force): ${path}"
      
      find "${path}" -type f -print0 | \
      
      rsync -ahvR --progress --remove-source-files --files-from=- --from0 . "${ARCHIVE_TARGET_DIR}/"
    else
      echo "Warning: Force-move path not found: ${path}"
    fi
  done
)

# 条件付き処理
echo "[Phase 2] Processing conditional-scan paths (preserving structure)..."
(
  cd "${COMMON_BASE_DIR}" && \

  for path in "${CONDITIONAL_SCAN_PATHS[@]}"; do
    if [ -d "${path}" ]; then
      echo "Scanning (conditional): ${path}"
      
      find "${path}" -type f \( \
        \( -size +${IMMEDIATE_MOVE_SIZE_MB}M \) \
        -o \
        \( -mtime +${RULE_DAYS_OLD} -size +${RULE_SIZE_MB}M \) \
      \) -print0 | \
      
      rsync -ahvR --progress --remove-source-files --files-from=- --from0 . "${ARCHIVE_TARGET_DIR}/"
    else
      echo "Warning: Conditional-scan path not found: ${path}"
    fi
  done
)

echo "Archive process finished."