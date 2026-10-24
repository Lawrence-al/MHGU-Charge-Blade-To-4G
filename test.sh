#!/usr/bin/env bash
# 默认验证不需要游戏文件；传入 dump、4G ELF、Forge NSO 可追加原版数据比对。
set -euo pipefail
task_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$task_root"
node "tests/verify-build.js"
node "tests/verify-install.js"
node "tests/verify-config-file.js"
node "tests/verify-runtime-hook.js"
node "tests/verify.js"
node "tests/verify-j-guard.js"
node "tests/verify-shield-thrust.js"
node "tests/verify-presentation.js"
node "tests/verify-resource-loader.js"
node "tests/verify-gaxe-preview.js"
node "tests/verify-gaxe-ground.js" "${1:-}"
node "tests/verify-saed-ground-oracle.js" "${1:-}"
node "tests/verify-impact-api.js" "${1:-}"
node "tests/verify-gaxe-extraction.js"
node "tests/verify-phial-surfaces.js"
node "tests/verify-phial-bridges.js" "${1:-}"
node "tests/verify-phial-visuals.js"
node "tests/verify-action.js"
node "tests/verify-guard.js" "${1:-}" "${2:-}"
node "tests/style-policy-host.js"
node "tests/verify-style-guard-host.js"
node "tests/style-menu-choices.js"
node "tests/style-menu-abi.js"
node "tests/style-menu-ui.js"
node "tests/style-actions-host.js"
node "tests/style-actions-arm.js" "${1:-}"
rustc --edition=2024 --extern cb_game="Moveset-to-4G/target/integration-check/libcb_game.rlib" \
    "tests/verify-game-api.rs" -O -o "Moveset-to-4G/target/integration-check/verify-game-api.exe"
"Moveset-to-4G/target/integration-check/verify-game-api.exe"
if [ "$#" -eq 0 ]; then
    printf '基础验证通过。原版数据比对：bash test.sh "MHGU dump" "4G ExeFS.elf" "Forge NSO"\n'
elif [ "$#" -eq 3 ]; then
    node "tests/verify-build.js" "$3" "$1/exefs"
    node "tests/verify-overpowered.js" "$1"
    node "tests/verify-hitdata.js" "$1"
    node "tests/verify-saed.js" "$1" "$2"
else
    printf '用法：bash test.sh [MHGU-dump目录 4G-ELF Forge-NSO]\n' >&2
    exit 2
fi
