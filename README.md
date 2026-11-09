# MHGU Charge Blade to 4G

让《怪物猎人 XX / Generations Ultimate》盾斧的音效、特效和部分战斗动作更接近 4G。适用于 **MHGU 1.4.0**，需要先安装与本项目配套的 Forge 前置。

原版Switch／模拟器使用Forge前置，重编译端使用其内建的Forge加载器。两个NRO的入口及内部桥均通过标准 `forge_hook_create` 安装。通用入口使用接口提供的original。Presentation在重编译端将主动作和超解发瓶事件接到序言后的内部桥，通过NRO重放原序言并续入原函数，原版使用入口钩子。已完成gu-native0.1.6解释器入口绕过复现和实际NRO桥的ARM对照，安卓游戏画面待复测。详见[主动作触发修正](docs/2026-11-10_重编译盾斧主动作触发修正.md)及[此前接口兼容范围](docs/2026-11-06_实现-NRO跨运行时Hook兼容-report.md)。

项目分为两个插件，可以分别启用；4G 风格已合并到 Moveset：

| 插件 | 文件 | 主要内容 |
| --- | --- | --- |
| CB Presentation to 4G | `cb_presentation_to_4g.nro` | 音效、盾牌及攻击特效 |
| CB Moveset to 4G | `cb_moveset_to_4g.nro` | 4G风格、按风格的派生/防性、盾补正、超解、攻击数据及大解追加派生 |

## 改了哪些地方

**Presentation**

- 调整各动作的音效、特效触发时间和持续时间。
- 剑能量达到黄色／红色时立即提示一次，取消GU的150ms延迟；独立开关默认开启，保留原充能光效。
- 调整盾突与瓶爆表现，并区分榴弹瓶和强属性瓶的效果。
- 可以分别设置音效、特效和各动作的开关、时机及持续时间。
- GU盾牌闪电和时长保留；只提取`gaxe_005/103`第二段爆炸，在大解砸下/超解砸地时追加。
- Presentation菜单按4G特效、GU lightning、Sound排序；4G超解瓶爆107默认关闭，其余五项4G视觉默认开启。
- 六个独立4G视觉开关：上述两段爆炸、002瓶附加、003/008一二解瓶爆、006/009大解三爆及107超解瓶爆。按瓶型和实际元素分组，保留原伤害和时序。
- 29项完整资源仍可逐个手动预览；可独立选Ground/Water/Auto。107的group0/1为榴弹普通/末瓶，地面/水面由另一筛选字段决定。资源内嵌NRO，无需替换ARC。

**Moveset**

- 按风格编号将 `0x44/0x4B` 转为新增前置 `0xAA/0xAB`；默认对武士道（3）和4G（6）启用，不再使用旧七处黄盾补丁。
- 红盾、蓝盾补正改为 1.2，黄盾保持 1.0；调整部分风格的 GP 后退和大解瓶爆震动。
- 白盾／红盾恢复 4G 防性：GP 在技能修正后独立减防御威力 10，红盾使用 40/50 硬直阈值；与GP后退分别配置风格，默认只对6启用。
- J防成功后可接黄盾；盾突后接黄盾、小退/中退后接黄盾、大解额外派生均可按风格选择。前两项需配合同风格“黄盾变正常AED”才会变成普通大解。
- 超解追加第三次命中，调整瓶伤，按实际瓶数逐次爆炸；榴弹瓶采用直线，强属性瓶采用扇形，爆点适应地面高度，支持最多十瓶。
- 提供一套调整后的盾斧攻击数据，可以独立开关。
- 可选超模扩展大解后摇派生，保留原有红盾取消；**默认关闭**。
- 可按风格启用大解平滑收斧，默认关闭。大解结束后在同一动作中进入恢复阶段，收斧期间延续已开启的额外派生，时长默认20。

**Moveset 内置的 4G 风格**

- 使用盾斧打开原版狩猎风格菜单，第七项选择 **4G**；选择会即时保存。
- 新动作 `0xAA / 0xAB` 使用逻辑 variant `16 / 17`，分别复用原 `0x44 / 0x4B` 的前置动画；结束后走修复版的大解／超解与派生链。
- 底层复用公会资源。音效、特效、伤害和防性等功能继续使用 Presentation／Moveset；新插件不复制这两份功能。
- 在配置中使用编号6，底层继续读公会资源。七项功能按0..7八个编号选择，7为预留；超解仍是全风格共用开关。
- 新动作和 HDP `0x50` 始终加载，即使插件战斗功能关闭也可由其它动作引用。

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

## 验证

安装 Node.js 后，在完成编译的仓库根目录执行：

```bash
bash "test.sh"
```

默认检查 NRO 格式、配置与菜单保存、补丁组合、ARM 调用及动作回归，不需要游戏文件。开发者持有本地 MHGU dump、4G ELF 和 Forge NSO 时，可追加原版数据及导出符号比对：

```bash
bash "test.sh" "/path/to/MHGU-dump" "/path/to/4G-ExeFS.elf" "/path/to/Forge-subSDK"
```

移植特效包保存在`assets/ported/`，常规构建直接嵌入已转换数据。使用Python 3可单独验证资源完整性：`python "tools/port_cb_gaxe.py" verify`。重新从4G原始资源转换才需要本地dump和指定转换工具；详见[005/103接入](docs/2026-10-24_4G盾斧005-103接入.md)。

详细功能说明见 [插件说明](docs/Plugins.md)，配置含义见 [配置说明](docs/Configuration.md)。
4G 菜单和新增动作的实现及验证见 [4G 风格](docs/Style.md)。
