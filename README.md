# MHGU Charge Blade to 4G

2026-11-10已按用户要求把原分析说明移入Obsidian统一归档。

- [现行知识库总览](<D:/Work/obisidian-note/逆向/MHGU/docs/overview.md>)
- [本专题](<D:/Work/obisidian-note/逆向/MHGU/docs/modules/methodology.md>)
- [原文 S0352](<D:/Work/obisidian-note/逆向/MHGU/backup/2026-11-10/projects/MHGU-Charge-Blade-to-4G/README.md>)

原文SHA256为 `6fd6653b7f916a24110a8546a8ad7254133e2dff2dec13def9a5b0f7251cc5d2`，版本和验证范围以归档原文及现行专题说明为准。

## 工程使用段

以下保留当前工程原README的安装、配置和构建段，便于原有入口及打包使用。详细分析在上方统一归档中。

## 安装与使用

1. 安装配套 Forge 前置。
2. 将需要的 NRO 放入 MHGU Mod 的 `romfs/nativeNX/plugins/`。
3. 重编译版本还需把对应的[默认INI](conf/)放在NRO旁，启动时会将已有INI加入资源索引。
4. 完全退出并重启游戏，在 Forge 菜单中修改配置。

```text
romfs/nativeNX/plugins/cb_presentation_to_4g.nro
romfs/nativeNX/plugins/cb_moveset_to_4g.nro
```

使用发布 NRO 时，无需额外复制项目里的参考资源。

旧的 `cb_style_to_4g.nro`、`cb_saed_to_4g.nro`、`cb_hitdata_to_4g.nro`、`cb_overpowered.nro` 已整合进 Moveset，不要重复安装。部署脚本会备份并停用旧Style NRO。同一功能的旧插件或金手指也不要同时启用。

覆盖同名 NRO 后重启即可更新，已有配置保留。移出对应 NRO 并重启即可停用该插件。

## 修改配置

打开 **Forge → Plugin UI → CB Presentation to 4G / CB Moveset to 4G**。修改后点击 **Save INI**，重启游戏应用。

- **Discard edits**：恢复上次保存值。
- **Defaults**：恢复默认草稿，仍需点击 Save INI。
- 黄盾变正常AED默认启用武士道3和4G6，GP后退/防性默认仅6；均只由风格列表判断，`none`全关。
- 盾突后接黄盾、防御后接黄盾、大解额外派生的默认列表均为none；J防接黄盾默认开启。超解及数值调整使用各自全局开关。
- 设置按Moveset、SAED、数值排列。已有列表保留；要应用新默认，点击Defaults后Save INI并重启。

两个INI分别为 `cb_presentation_to_4g.ini` 和 `cb_moveset_to_4g.ini`。Switch和Ryujinx自动使用当前MHGU用户存档根目录，gu-native、MHGURecomp及Enhanced自动使用 `romfs/nativeNX/plugins/`。菜单显示当前位置，Save INI写回同一文件。原版风格菜单选择即时应用并保存，功能设置保存后重启。重编译默认INI需随NRO安装，部署时保留已有配置。

| 配置区域 | 可以调整的内容 |
| --- | --- |
| Presentation | 音效、盾牌特效，各动作时机、持续时间，以及独立的 4G 大解命中／超解砸地特效 |
| Moveset | 按风格的黄盾变正常AED、GP后退、防性、盾突/防御后黄盾派生、大解额外派生，以及独立J防接黄盾 |
| SAED | 超解改动总开关、瓶爆特效档位；时序与轨迹随总开关启停 |
| Values / Hitdata | 收斧时长、盾补正、固定震动、调整后的攻击数据开关，排在最下方 |
| Styles | 七项列表仅0..7，默认黄盾变AED为3,6，GP后退/防性为6，其他四项none |

默认配置模板放在 `conf/`：[Presentation](conf/cb_presentation_to_4g.ini)、[Moveset](conf/cb_moveset_to_4g.ini)。4G选择保存在Moveset的 `[style] selected`，不写入装备套装；在原版菜单选择公会可以取消。旧Style源码仅保留作历史参考，不再构建或安装。

## 编译

本仓库可以单独编译，不需要本机的 MHGU-forge 工作区。构建时从公开仓库获取固定提交的 Forge SDK 和对应打包工具；工具安装在项目的 `target/` 内。

准备以下环境：

- Git、Rust nightly 和 `rust-src`。
- devkitPro / devkitARM。
- Windows 使用 devkitPro 的 MSYS2 Bash；Rust MSVC 工具链需要本机 C++ Build Tools。默认 devkitPro 安装位置为 `C:/devkitPro`，其它位置通过 `DEVKITPRO` 指定。

从本仓库根目录编译：

```bash
rustup toolchain install nightly --component rust-src
bash "build.sh"
```

Windows 也可以在仓库根目录使用 PowerShell；以下选项只影响当前进程的脚本执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1
```

首次构建需要联网下载依赖。依赖已缓存后可执行 `bash "build.sh" --offline`，PowerShell 在同一命令末尾加 `-Offline`。

最终文件统一放在：

```text
dist/
  cb_presentation_to_4g.nro
  cb_moveset_to_4g.nro
```

构建输出统一在 `dist/`；编译缓存与工具在 `target/` 中，`dist/` 和 `target/` 都不提交到 Git。构建不需要游戏本体文件。

可手动安装 `dist/` 中的文件，也可指定插件目录：

```bash
bash "deploy.sh" "/path/to/mod/romfs/nativeNX/plugins"
```

脚本复制两个NRO，并在plugins目录缺少INI时复制默认配置。
