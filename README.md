# MHGU Charge Blade to 4G

让《怪物猎人 XX / Generations Ultimate》盾斧的音效、特效和部分战斗动作更接近 4G。适用于 **MHGU 1.4.0**，需要先安装与本项目配套的 Forge 前置。

项目分为两个插件，可以分别启用：

| 插件 | 文件 | 主要内容 |
| --- | --- | --- |
| CB Presentation to 4G | `cb_presentation_to_4g.nro` | 音效、盾牌及攻击特效 |
| CB Moveset to 4G | `cb_moveset_to_4g.nro` | 动作派生、盾补正、超解、攻击数据及可选超模 |

## 改了哪些地方

**Presentation**

- 调整各动作的音效、特效触发时间和持续时间。
- 调整盾突与瓶爆表现，并区分榴弹瓶和强属性瓶的效果。
- 可以分别设置音效、特效和各动作的开关、时机及持续时间。

**Moveset**

- 调整公会、空战、炼金的部分黄盾派生，使其可以派生普通大解；炼金仅调整 GP / 蓄力斩后的对应派生。
- 红盾、蓝盾补正改为 1.2，黄盾保持 1.0；调整部分风格的 GP 后退和大解瓶爆震动。
- 武士道 J 防成功后、盾突后，可以分别开启 **X+A 派生大解**。
- 超解追加第三次命中，调整瓶伤，按实际瓶数逐次爆炸；榴弹瓶采用直线，强属性瓶采用扇形，爆点适应地面高度，支持最多十瓶。
- 提供一套调整后的盾斧攻击数据，可以独立开关。
- 可选超模扩展大解后摇派生，保留原有红盾取消；**默认关闭**。

## 安装与使用

1. 安装配套 Forge 前置。
2. 将需要的 NRO 放入 MHGU Mod 的 `romfs/nativeNX/plugins/`。
3. 完全退出并重启游戏，在 Forge 菜单中修改配置。

```text
romfs/nativeNX/plugins/cb_presentation_to_4g.nro
romfs/nativeNX/plugins/cb_moveset_to_4g.nro
```

使用发布 NRO 时，无需额外复制项目里的参考资源。

升级时只保留这两个当前版本的盾斧插件。旧的 `cb_saed_to_4g.nro`、`cb_hitdata_to_4g.nro`、`cb_overpowered.nro` 已整合进 Moveset，不要重复安装。同一功能的旧插件或金手指也不要同时启用。

覆盖同名 NRO 后重启即可更新，已有配置保留。移出对应 NRO 并重启即可停用该插件。

## 修改配置

打开 **Forge → Plugin UI → CB Presentation to 4G / CB Moveset to 4G**。修改后点击 **Save INI**，重启游戏应用。

- **Discard edits**：恢复上次保存值。
- **Defaults**：恢复默认草稿，仍需点击 Save INI。
- 默认关闭超模，其余已列出的功能默认开启。

首次运行在 **MHGU 当前用户的存档根目录**生成 `cb_presentation_to_4g.ini` 和 `cb_moveset_to_4g.ini`，不建子目录。菜单与手动编辑使用同一位置，手动修改后也需重启。

| 配置区域 | 可以调整的内容 |
| --- | --- |
| Presentation | 音效、特效总开关，各动作时机、持续时间、盾突效果 |
| Moveset | 黄盾派生、盾补正、GP 后退、武士道派生和大解震动 |
| SAED | 超解改动总开关、瓶爆特效档位；时序与轨迹随总开关启停 |
| Hitdata | 调整后的攻击数据开关 |
| Overpowered | 超模开关，默认 `false` |

仓库的默认配置模板放在 `conf/`：[Presentation](conf/cb_presentation_to_4g.ini)、[Moveset](conf/cb_moveset_to_4g.ini)。编译时嵌入 NRO，首次运行仍生成到存档根目录。

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

根目录和两个源码目录不保留重复 NRO。编译缓存与工具在 `target/` 中，`dist/` 和 `target/` 都不提交到 Git。构建不需要游戏本体文件。

可手动安装 `dist/` 中的文件，也可指定插件目录：

```bash
bash "deploy.sh" "/path/to/mod/romfs/nativeNX/plugins"
```

脚本只复制这两个 NRO。INI 仍由游戏在当前用户存档根目录生成。

## 验证

安装 Node.js 后，在完成编译的仓库根目录执行：

```bash
bash "test.sh"
```

默认检查 NRO 格式、配置与菜单保存、补丁组合、ARM 调用及动作回归，不需要游戏文件。开发者持有本地 MHGU dump、4G ELF 和 Forge NSO 时，可追加原版数据及导出符号比对：

```bash
bash "test.sh" "/path/to/MHGU-dump" "/path/to/4G-ExeFS.elf" "/path/to/Forge-subSDK"
```

详细功能说明见 [插件说明](docs/Plugins.md)，配置含义见 [配置说明](docs/Configuration.md)。
