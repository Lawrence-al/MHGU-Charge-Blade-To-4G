# MHGU SAED 瓶爆与 HDP 扩容分析报告

日期：2026-10-03。采用本地已有 IDA 反编译导出、原始 NSO／ELF、Node ARM 指令解释器和 Rust／devkitARM 构建；未启动游戏验证。

## 核心结果及 Scope

瓶爆由 NRO 管理：首爆采用 4G 普通地面超解榴弹 0／强属性 46 的倒计时；在实际首爆当帧锁定 GU 人物位置，第一爆沿初始化朝向前移 270／300。后续保留榴弹间隔 9、前进 100，以及强属性前六爆的相对时间和位置，并扩展第四组四个点。每一爆通过 GU 原生地面函数取 Y。地面超解第三下读新增 HDP `0x50`；瓶伤四条、空中第三下及其它第三下补丁保留原值。特效档位当前为 2。盾突在动作入口 `0x011DB280` 的起手更新有红盾就创建闪电，持续 30，不等待命中。

范围见 [scope.md](saed-runtime/scope.md)。SH／HDP 文件没有在本次修改；0x4F 文件内容由用户自行处理。完整参数见 [SAED.md](SAED.md)。

## Evidence

各地址为 main 相对偏移，4G 地址为 ELF 虚拟地址。C 导出中的 GU 地址带 `0x60000000` 加载基址。下列本地输入需要使用对应版本的用户 dump；命令在项目根目录的 Bash 中执行。

| E-id | source_ref | repro_command | content_hash | 观察 |
|---|---|---|---|---|
| E-001 | 4G `ExeFS.elf`，`0xD1A020～0xD1A204`、`0x100E800/0x100E878` | `node "tests/verify-saed.js" "D:/Work/dump-classicMH/MHGU dump" "D:/Work/dump-classicMH/MH4G dump/4gv1.2.cia/ExeFS.elf"` | n/a，原始输入位于项目外 | ARM 启动初始化后，榴弹位置 70/170/270/370/470/570，时间 0/9/18/27/36/45；强属性位置与时间见 F-001 |
| E-002 | 4G `ExeFS.elf.c`：SAED_phail `0xC60264`、构造 `0xC6155C`、地面函数 `0x2B3610` | `node "../.codex-tmp/saed-4g-runtime/analyze.js" function four 00C60264 00C6155C 002B3610` | n/a，已有本地导出 | 每次按总数减剩余数选表项，旋转局部 X/Z；出生位置前移 200；逐爆查地面 |
| E-003 | GU `main.c`：更新 `0x488DE4`、SH 查询 `0x4A3164`、贴地 `0x181440`、攻击 `0x4A2790` | `node "../.codex-tmp/saed-4g-runtime/analyze.js" function gu 60488DE4 604A3164 60181440 604A2790` | n/a，已有本地导出 | SH 第三个值用于完整生命周期收尾；GU 有同类地面查询；原攻击提交仍读 HDF |
| E-004 | GU `main.c`：HDP load `0x270C18`、unload `0x270BD0`、lookup `0x270D98` | `node "../.codex-tmp/saed-4g-runtime/analyze.js" function gu 60270C18 60270BD0 60270D98` | n/a，已有本地导出 | 数据指针 +0x64、条目指针 +0x68；计数在头 +8；条目步长 56；相同分配器负责释放 |
| E-005 | 实际 schedule.rs／hitdata.rs、原版／asset HDP、4G `0xC60350～0xC60390` | 与 E-001 相同的 `tests/verify-saed.js` 命令 | n/a，验证运行输出 | 100 组时序、720 次旋转、1536 组原指令坐标；两份 HDP 原条目保留，追加 0x50 与第三下快照逐字节一致 |
| E-006 | release NRO／ELF、安装器 | `node "tests/verify.js"`；`node "tests/verify-install.js"`；`node "tests/verify-shield-thrust.js"` | n/a，验证运行输出 | 原值表／写入范围、浮点 ABI、64 组缓存失败恢复、GP 指令及 576 组盾突起手入口验证通过 |
| E-007 | GU `main.c`、原 NSO：盾突 `0x011DB280`、盾状态读取 `0x002B6568` | `node "../.codex-tmp/saed-4g-runtime/analyze.js" function gu 611DB280 602B6568`；`node "tests/verify-shield-thrust.js"` | n/a，已有导出及本次 release | 盾突 state 0 初始化后推进状态；虚函数 +0x85C 直接读 player+0x3344，红盾为 2；入口不含 PC 相对指令。576 组编译入口和 16 组实际模块／瓶型选择通过 |
| E-008 | 4G `AED_hit` 0xC60644、字面量 0xC60980／0xC60984、倒计时 0x99440C | 与 E-001 相同的命令；`node "../.codex-tmp/saed-4g-runtime/analyze.js" function four 00C60644 0099440C` | n/a，原始输入位于项目外 | 普通地面 mode=2：type=5 首爆倒计时 0，type=4 为 46；另两个模式分别为 2／40，六个原 ARM 初始化分支验证通过 |
| E-009 | 实际 phial.rs／schedule.rs，tests/verify-phial-origin.rs | 与 E-001 相同的命令 | n/a，验证运行输出 | 模拟两种瓶型各十瓶，首爆前后移动人物；20 个爆点均使用首爆当帧锁定坐标，瓶数与镜像位置正确；此测试的粒子创建和地面查询为替身 |

## Findings

| F-id | severity | evidence_ids | confidence | location | status | 结论 |
|---|---|---|---|---|---|---|
| F-001 | n/a_re | E-001、E-002 | high | 4G 0xC60264；0x100E800/0x100E878 | validated | 时间和 X/Z 来自初始化后的表，不能直接把 ELF 中尚未填充的位置零值当成游戏数据 |
| F-002 | n/a_re | E-002、E-003 | high | GU 0x181440；native.rs／phial.rs | validated，离线 | GU 存在可复用的竖直地面查询；返回高度在 s0，成功标志写出到字节指针 |
| F-003 | n/a_re | E-003 | high | GU 0x488DE4 | validated | 原 6、6、21 修改不会形成准确的每爆间隔 21；旧重置方式下后续伤害间隔为 15 |
| F-004 | n/a_re | E-004、E-005 | high | GU 0x270C18；hitdata.rs | validated，离线 | 扩容为 81 条后原 lookup 能直接取得索引 0x50，无需改 lookup 或覆盖 0x4F |
| F-005 | n/a_re | E-001、E-005 | high | schedule.rs | validated，离线 | 前六个强属性点取原表；第四组 7～10 为用户十瓶需求的外推，不声称是 4G 原表 |
| F-006 | n/a_re | E-006、E-007 | high | Presentation shield_thrust.rs；GU 0x011DB280 | validated，离线 | 起手 state 0 的原初始化后，有红盾就创建 0x258／0x259，持续 30；用户反馈闪退后已撤除新增 request 0x47 调用及音序对象读取；不等待命中、CreateShell02 调用或其返回值 |
| F-007 | n/a_re | E-008、E-009 | high | schedule.rs／phial.rs | validated，离线 | 两种瓶型表首项虽为 0，但前置倒计时不同；使用 0／46 并在 GU 首爆当帧锁定位置，后续时间与位移相对实际首爆 |

### 4G 的实际位置与时间

`SAED_phail` 用总瓶数减剩余瓶数得到索引，每项 20 字节。4G 的进程类型 5 取榴弹表 `0x100E800`，类型 4 取属性表 `0x100E878`；这些类型值与 GU 资源子类型不是同一编号。

强属性六项 `(时间、横向、前进)` 为 `(0,0,100)`、`(18,80,220)`、`(20,-80,220)`、`(38,-120,340)`、`(40,120,340)`、`(42,0,340)`。进程创建时另外前移 200，因此换算到出生玩家位置为前方 300、420、540。

榴弹原表时间间隔 9。本版按用户最新纠正使用 9，第一爆位置保留玩家前方 270，再每次增加 100。时间累加 GU shell `+0x1C` 的 `mDeltaTime`，没有按毫秒换算。旧 SH 的完整 21 是 6+6+9，不作为两爆之间的间隔。

首爆不能只看 `SAED_phail` 表第一项的 0。`AED_hit` 在表开始前设置 `process+316` 的倒计时：普通地面 `process+660=2` 时，榴弹 type=5 用 0，强属性 type=4 用字面量 46；`0x99440C` 完成倒计时后才进入表。另两个原模式分别是 2／40，本版采用用户此次选择的普通地面首爆基准。旧版统一等待 12，使榴弹比该基准晚、强属性比该基准早。

4G 构造函数记录出生人物坐标并前移 200，再加表首项 70／100 得到 270／300。本版距离保持这两个原值，但在 GU 实际首爆当帧重新读取人物坐标，以适配等待期间人物前移；这是 GU 适配，记录坐标的时点与 4G 构造函数不同。之后固定该坐标，仅按原朝向和相对位移推进。首爆触发时把时间归零，后续间隔也相对实际首爆，暂停时不生成爆点。

强属性第七至十项继续按纵向 +120、横向半宽 +40 扩展，四点均匀分布于 ±160 与 ±160/3；累计时间取 58、60、62、64。少于十瓶时按表顺序提前结束。

新增核对直接执行 4G `0xC60350～0xC60390` 的 VFP 旋转／平移，对 256 个朝向、前六爆共 1536 个坐标逐项比较。NRO 横向绝对偏移与原指令一致，最大浮点误差 0.000122。第二组两点间距 160，第三组左右两端间距 240；没有把这些绝对偏移错误地累计为逐爆增量。游戏粒子本身的局部偏移尚未取得运行证据，档位改为 2 后由用户继续判断画面。

### GU 的旧 SH 计时

原状态 0 等待 SH[0]=6 后创建特效，状态 1 再等待 SH[1]=6 后创建伤害，最后收尾段为 SH[2]-SH[0]-SH[1]。第三个值不是“等待 200 后才生成 hitbox”。

旧桥接在生成伤害后把状态改回 0；修改后的 SH[2]=21 使这一段先等 `21-6-6=9`，再进入 6 的特效到伤害阶段，所以后续伤害间隔为 15。本版移除两处旧桥接，以累计时间独立决定每爆，首次倒计时榴弹 0／强属性 46、最后收尾 9。特效与伤害在同一爆点提交。

### 地面高度与攻击位置

4G 两组探测高度参数是 `(300,-2000)`、`(2000,300)`，基准为上一爆高度。GU 对应函数 `0x181440` 的参数依次为碰撞系统、下方位置、上方位置、区域号、成功字节、过滤标志、过滤 mask、可选接触结果。GU 查询从上方发起竖直向下射线；两个高度参数参与射线起点计算，不是线段长度限制。

碰撞系统取 `main+0x21270E0`，区域号取 shell `+0x1054`，过滤条件沿用 shell 虚表 `+0xA8/+0xAC`。ARM 适配函数将第五至八个参数写入栈，并从 s0 取结果。每爆更新 shell 主位置和 +0x1000 镜像，再复用原特效／伤害提交。原后处理 `0x4A23DC` 更新矩阵和攻击对象。

停止函数 `0x4A247C` 最终写 `shell+0x0C` 的存活标志为 3；`shell+4` 是动作状态字，不可用于判断对象已停止。本版保留原父对象离场检查，然后按 +0x0C 判断是否结束 NRO 序列。

### HDP 新条目

本次 asset `0x4F` 的 56 字节被内置为 NEW_50，持续 8、动作值 100。加载成功后，使用原 HDP 分配器重新分配 4552 字节，复制头和所有原条目，头计数从 80 改为 81，再写最后一条。更新两个资源指针后释放旧缓冲；原 unload 无需改变。

用户自己处理原文件 `0x4F`；新代码不改它。地面第三下调用 0x50，空中仍调用 0x23。`011DB014` 的旧替换补丁已经移除。

## Path

### P-001：瓶爆调用链

- path_type：callflow
- start：GU 创建 uShellWp14Shell03
- goal：按瓶型的时间／位置表逐爆提交特效及攻击
- steps：原 init `0x488C0C` 成功 → NRO 记录朝向和瓶数、按瓶型设置首爆倒计时 → update `0x488DE4` 累加 delta → 首爆当帧锁定人物坐标 → 后续按固定基准和相对位移计算 X/Z → GU 地面查询 `0x181440` 求 Y → 更新位置 → 原特效创建 `0x4A1E50` → 剩余瓶数减一 → 原瓶伤 `0x488FC0` → 最后收尾 → 原停止／销毁。（E-001、E-002、E-003、E-005、E-008、E-009；F-001、F-002、F-003、F-005、F-007）
- residual_risks：尚未运行实际战斗，斜坡／矮墙的碰撞材质命中和连续伤害需要游戏验证。

### P-002：地面第三下 0x50 调用链

- path_type：callflow
- start：rHitDataPlayer::load `0x270C18`
- goal：原 HDP 查找函数取得新增第三下条目
- steps：原 load 成功 → NRO 按资源名筛选盾斧 HDP → 同分配器扩容、计数 81 → `011DADDC` 提交 id=0x50 → 原创建函数 `0x2B7240` → 原 lookup `0x270D98` 返回新增条目。（E-004、E-005、E-006；F-004）
- residual_risks：默认原条目数量 80，分配及原加载成功；没有附加空指针保护。资源 unload 用原分配器释放新缓冲。

### P-003：红盾盾突起手

- path_type：callflow
- start：盾突动作 `sub_611DB280` 入口，调用前 actionState=0
- goal：有红盾就立即创建盾牌闪电，空挥和命中都播放
- steps：入口钩子保存起手标志 → 按原参数执行原函数初始化动画 → 读取 player+0x3344，红盾状态 2 → 按装备瓶型创建 0x258／0x259、记录持续 30 → 返回原函数返回值；后续更新不重复创建，入口不读取音序对象或新增音效调用。（E-006、E-007；F-006）
- residual_risks：盾牌对象和 PEL 沿用原动作有效指针前提，没有新增空指针判断；没有游戏画面验证。旧 011DB498 桥接及共用工厂 011DE158 均不写入。

## 构建与复现

在 `D:/Work/MHGU-forge/MHGU-Charge-Blade-to-4G/SAED-to-4G`：

```bash
export PATH="/c/Users/Lawrence/.cargo/bin:/c/devkitPro/msys2/usr/bin:/c/devkitPro/devkitARM/bin:$PATH"
export CARGO_TARGET_ARMV7A_NONE_EABI_LINKER="C:/devkitPro/devkitARM/bin/arm-none-eabi-gcc.exe"
cargo forge build --release
```

在项目根目录：

```bash
node "tests/verify-saed.js" "D:/Work/dump-classicMH/MHGU dump" "D:/Work/dump-classicMH/MH4G dump/4gv1.2.cia/ExeFS.elf"
node "tests/verify.js"
node "tests/verify-install.js"
node "tests/verify-shield-thrust.js"
```

两个 NRO 构建成功；SAED 只有现有链接器 `-z norelro ignored` 提示，Presentation 另有停用日志及常量的未使用提示。算法验证不等同于实际游戏测试。游戏测试关注盾突闪电、1／5／10 瓶、榴弹直线／属性分组、矮墙／斜坡贴地、地面第三下动作值。

已把以下两个新 NRO 复制到项目根目录及 Ryujinx `0100770008dd8000/romfs/nativeNX/plugins/`，各自的子项目、项目根目录、游戏目录三份 SHA-256 一致：

| 文件 | 字节数 | SHA-256 |
|---|---:|---|
| presentation_to_4g.nro | 32768 | `86ca4f564774757285f6bba5bc3445d15887bf2c873324cb3f398065ff43f678` |
| saed_to_4g.nro | 36864 | `c90e5698664559964b3bd4f149fa2f9a1c2021ad68322a1edcc87cb711aa0538` |

本次替换前的项目根目录／游戏目录产物备份在 `D:/Work/MHGU-forge/.codex-tmp/saed-4g-runtime/backups/`，文件前缀 `2026-10-03-first-blast-shield-start-`；前一版备份前缀 `2026-10-03-shield-caller-interval9-` 仍保留。备份不在插件加载目录。Moveset NRO 和本地 asset 均未改动。

后续参数调整：盾突持续时间由 20 改为 30；`actionState=2 variant=0|7 requestId=0x4C` 的现有触发帧 27 再后移 3，改为 30，无瓶也执行。只重新编译并部署 Presentation；576 组编译入口及 16 组瓶型选择验证通过，三份 NRO 的 SHA-256 一致。本次替换前的两份产物备份前缀为 `2026-10-03-shield30-se4c30-`。

随后加入盾突闪电同帧音效 request 0x47：在同一红盾起手分支调用 `play_se_on`，使用本次盾突玩家自己的音序 SE 对象，bank=0、attach_index=2、param_a5=0；特效仍持续 30。576 组实际 NRO 入口检查及 16 组模块检查确认只调用一次、七个池参数和 SE 归属对象正确。重新编译并部署 Presentation，三份 SHA-256 一致；本次替换前的两份产物备份前缀为 `2026-10-03-shield30-se47-`。

用户随后反馈加入 0x47 的版本持续闪退，要求删除新增音效。本次撤除盾突音效调用、SOUND_REQUEST 常量及音序对象读取，仅保留 30 帧闪电；重新编译的 NRO 与加入音效前的备份逐字节一致（SHA-256 回到上表数值）。576 组实际 NRO 入口及 16 组模块检查通过，项目与游戏目录三份 NRO 一致。撤回前版本的备份前缀为 `2026-10-03-remove-shield-se47-`；实际游戏表现待重启复测。

## Timeline 摘要

2026-10-03，先读取当前 SAED 补丁与原版 GU／4G 导出，执行 4G 表初始化，定位 GU 地面查询、HDP 分配／查找和原攻击提交，并按当时确认的 21 实现首版 NRO 时序和 HDP 扩容；0x4F 文件由用户自行处理。随后用户纠正榴弹间隔为 9，要求特效档位 2，盾突特效曾接到动作内 `0x011DB498`，以 1536 个原指令坐标核对属性左右位置并部署。用户此次反馈盾突空挥／命中都不播放、首爆时间和位置不对，并明确有红盾盾突就播放。最终撤去调用点及等待条件，改用盾突起手入口；新增 4G AED_hit 倒计时依据，采用 0／46 和首爆当帧位置基准，完成离线验证、release 编译及两个 NRO 的再次部署。
