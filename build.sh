#!/usr/bin/env bash
# 固定 Forge 版本，两个独立 NRO 统一保留在 dist/。
set -euo pipefail
task_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
case "${1:-}" in
    "") ;;
    --offline) export CARGO_NET_OFFLINE=true ;;
    *) printf '用法：bash build.sh [--offline]\n' >&2; exit 2 ;;
esac
if command -v cygpath >/dev/null 2>&1; then
    task_devkit_default="/c/devkitPro"
else
    task_devkit_default="/opt/devkitpro"
fi
export DEVKITPRO="${DEVKITPRO:-$task_devkit_default}"
export DEVKITARM="${DEVKITARM:-$DEVKITPRO/devkitARM}"
export PATH="$DEVKITARM/bin:$PATH"
if command -v cygpath >/dev/null 2>&1; then
    export CARGO_TARGET_ARMV7A_NONE_EABI_LINKER="$(cygpath -m "$DEVKITARM/bin/arm-none-eabi-gcc.exe")"
else
    export CARGO_TARGET_ARMV7A_NONE_EABI_LINKER="$DEVKITARM/bin/arm-none-eabi-gcc"
fi

# 与各项目 Cargo.toml 一致；工具仅安装到本项目 target/，不改全局安装。
task_sdk_url="https://github.com/Lawrence-al/forge-rs.git"
task_sdk_rev="f2af0622a2a0aa291ffbd3d2f7628c0e2d96d220"
task_tools="$task_root/target/forge-tools"
CARGO_TARGET_DIR="$task_root/target/forge-tool-build" cargo +nightly install \
    --git "$task_sdk_url" --rev "$task_sdk_rev" --locked \
    --root "$task_tools" cargo-mhgu-forge
export PATH="$task_tools/bin:$PATH"

mkdir -p "$task_root/target/nro" "$task_root/dist"
for task_pair in "Presentation-to-4G:presentation_to_4g" "Moveset-to-4G:moveset_to_4g"; do
    task_project="${task_pair%%:*}"
    task_stem="${task_pair#*:}"
    (
        cd "$task_root/$task_project"
        # cargo-forge 按项目自己的 target/ 查找 ELF。
        unset CARGO_TARGET_DIR
        cargo +nightly metadata --locked --format-version 1 >/dev/null
        cargo +nightly forge build --release
    )
    mv -f -- "$task_root/$task_project/$task_stem.nro" "$task_root/target/nro/cb_$task_stem.nro"
done
# 所有构建均成功后，再更新发布输出。
mv -f -- "$task_root/target/nro/cb_presentation_to_4g.nro" "$task_root/dist/cb_presentation_to_4g.nro"
mv -f -- "$task_root/target/nro/cb_moveset_to_4g.nro" "$task_root/dist/cb_moveset_to_4g.nro"
if [ -f "$task_root/dist/cb_style_to_4g.nro" ]; then
    mkdir -p "$task_root/legacy-builds/style-before-merge"
    mv -- "$task_root/dist/cb_style_to_4g.nro" "$task_root/legacy-builds/style-before-merge/cb_style_to_4g.nro"
fi
printf '编译完成：%s/dist/\n' "$task_root"
