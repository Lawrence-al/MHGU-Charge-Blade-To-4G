#!/usr/bin/env bash
# 从dist安装两个NRO，plugins目录缺少INI时复制默认模板。
set -euo pipefail
task_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
task_plugins="${1:?用法：bash deploy.sh 插件目录}"
mkdir -p "$task_plugins"
if [ -f "$task_plugins/cb_style_to_4g.nro" ]; then
    task_archive="$task_root/legacy-builds/style-before-merge/installed-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$task_archive"
    mv -- "$task_plugins/cb_style_to_4g.nro" "$task_archive/cb_style_to_4g.nro"
fi
for task_stem in cb_presentation_to_4g cb_moveset_to_4g; do
    cp -- "$task_root/dist/$task_stem.nro" "$task_plugins/$task_stem.nro"
    if [ ! -f "$task_plugins/$task_stem.ini" ]; then
        cp -- "$task_root/conf/$task_stem.ini" "$task_plugins/$task_stem.ini"
    fi
done
printf '已安装两个NRO及缺少的默认INI到 %s\n' "$task_plugins"
