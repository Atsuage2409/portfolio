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

cd /home/saito-atsushi/sync || exit 1
rm $LOGFILE
echo "===start time $(date '+%Y-%m-%d %H:%M:%S')===" >> "$LOGFILE"
sudo ./archive.sh >> "$LOGFILE"
echo "===finish time $(date '+%Y-%m-%d %H:%M:%S')===" >> "$LOGFILE"
exit 0