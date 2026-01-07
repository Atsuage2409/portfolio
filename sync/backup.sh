#!/bin/bash
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
LOGFILE="/home/saito-atsushi/sync/logs/sync_list.log"
LOCK_FILE="/home/saito-atsushi/sync/lock/sync.lock"

# ロックファイル確認
if [ -e "$LOCK_FILE" ]; then
    echo "スクリプトは既に実行中です。ロックファイル $LOCK_FILE が存在します。"
    exit 1
fi
touch "$LOCK_FILE"
trap "rm -f \"$LOCK_FILE\"" EXIT
sleep 2

SOURCE_DIR="/mnt/storage/storage/"
DEST_DIR="/mnt/storage/WD_1/"
LOG_FILE="/home/saito-atsushi/sync/logs/sync_backup.log"

echo "Backup started at $(date)" | tee -a "${LOG_FILE}"
echo "Source: ${SOURCE_DIR}" | tee -a "${LOG_FILE}"
echo "Destination: ${DEST_DIR}" | tee -a "${LOG_FILE}"

rsync -ahvu --stats "${SOURCE_DIR}" "${DEST_DIR}" >> "${LOG_FILE}" 2>&1

if [ $? -eq 0 ]; then
  echo "Backup finished successfully at $(date)" | tee -a "${LOG_FILE}"
else
  echo "Backup FAILED at $(date)" | tee -a "${LOG_FILE}"
fi

echo "---------------------------------" | tee -a "${LOG_FILE}"