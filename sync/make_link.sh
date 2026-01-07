#!/bin/bash
set -euo pipefail

LISTFILE="/home/saito-atsushi/sync/logs/make_link_list"
DISK1="/mnt/storage/cash_1"
DISK2="/mnt/storage/WD_1"

while read -r SRC_FULL; do
    [[ -z "$SRC_FULL" || "$SRC_FULL" =~ ^# ]] && continue

    if [[ ! "$SRC_FULL" =~ ^$DISK1/ ]]; then
        echo "警告: 予期しないパス形式です: $SRC_FULL" >&2
        continue
    fi

    if [[ -e "$SRC_FULL" ]]; then
        echo "警告: 既にファイルまたはリンクが存在します: $SRC_FULL" >&2
        continue
    fi

    relpath="${SRC_FULL#$DISK1/}"

    DEST="$DISK2/$relpath"

    if [[ ! -e "$DEST" ]]; then
        echo "警告: 実ファイルが見つかりません: $DEST" >&2
        continue
    fi

    mkdir -p "$(dirname "$SRC_FULL")"

    ln -s "$DEST" "$SRC_FULL"
    echo "リンク作成: $SRC_FULL -> $DEST"

done < "$LISTFILE"
