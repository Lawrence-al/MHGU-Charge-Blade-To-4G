#!/usr/bin/env bash
# 从 dist/ 安装两个 NRO；INI 由游戏在当前用户存档根目录生成。
set -euo pipefail
task_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
task_plugins="${1:?用法：bash deploy.sh 插件目录}"
mkdir -p "$task_plugins"
for task_stem in cb_presentation_to_4g cb_moveset_to_4g; do
    cp -- "$task_root/dist/$task_stem.nro" "$task_plugins/$task_stem.nro"
done
printf '已安装两个 NRO 到：%s\n' "$task_plugins"
