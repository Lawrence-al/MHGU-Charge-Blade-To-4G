# Moveset-to-4G：派生变 4G

2026-10-22：4G风格并入本NRO；黄盾七处改字已移除，改为按风格将命令0x44/0x4B重定向到常驻0xAA/0xAB。GP后退及4G防性按0..255风格编号配置，4G为6；未选中执行原GU逻辑。盾突X+A支持武士道和勇气，J防保持仅武士道；AED瓶爆震动固定20仅启停。SAED继续全局启停，HDP 0x50始终加载。[当前配置](Configuration.md)。下表为旧补丁位置记录，黄盾七字不再写入。

产物：`cb_moveset_to_4g.nro`。本文说明派生模块 `Moveset-to-4G/src/moveset.rs`；其它已整合功能及开关见 [INI 配置](Configuration.md)。每组补丁按开关独立收集，由 `patches.rs` 和共用安装器一次核对／回滚。

| 修改 | main 偏移 | 备注 |
|---|---|---|
| 上黄盾变正常大解 | 011DA634、011DA5EC、011DA610、011DA5C0、011DA43C、011DA4B8、011DA4C0 | 包括公会、空战、炼金。炼金只能在 GP 后或蓄力斩后正常大解，其它派生仍上黄盾 |
| 红蓝盾补正修复 | 011DF8B0、011DF8D0 | 红盾、蓝盾 1.2；黄盾 1.0 |
| 大解瓶爆震动 | 0048859C | 按金手指从 15 改为 20 |
| GP 防御后后退 | 011DDE74 | 公会、空战、炼金；狩技和其它风格保持原行为 |
| 4G GP／红盾防性 | 011DFB0C、002F0800 | `guard.rs`：白盾／红盾 GP 独立减威力 10；红盾用 40/50 阈值。`guard_performance` 与后退开关独立，黄／蓝盾保留 GU 原规则 |
| J 防成功后 X+A → 大解前置 | 002EA87C | 武士盾斧 `0x81A` 派生表第 3 条，临时命令改为武器动作 `0x44`，不替换共享 `071a` |
| 武士道盾突后 X+A → 大解前置 | 002EA87C | `0x7FA` 第 1 条临时命令改为 `0x44`，由 `bushi_shield_thrust_aed` 独立控制；与 J 防共用同一个桥接 |

10 处直接补丁值原样保留，包括两处 `000001A8` 和 `011DF8D0` 的无条件 `E3A01002`。

`main+0x011DDE74` 写入 `E51FF004`，下一字保存 NRO 中 `gp_bridge` 的实际地址。覆盖 8 字节，返回 `main+0x011DDE7C`。

桥接读取 `*(u32*)(player+0x13C4)` 的 `+0x4D9` 字节；值为 1、3、5，或玩家 `+0x250A` 动作字节等于 0x54，执行 `r0 |= 0x20`，其它情况保持原 r0。补回 `str r0, [r1]` 后，通过 PC 相对寻址读取 NRO 数据段里的返回地址。保留整数寄存器、栈和该分支产生的 NZCV。

不写 `main+0x013F1F58～0x013F1F9C`。安装与缓存刷新在 `shared/patch.rs`；SVC 寄存器排列参照 [libnx AArch32 svc32.s](https://github.com/jeffi6375/libnx/blob/master/nx/source/kernel/svc32.s)，当前进程句柄和 ARM32 缓存方式参照 [cache.h](https://github.com/jeffi6375/libnx/blob/master/nx/include/switch/arm/cache.h)。

## J 防成功后的 X+A

`j_guard_bridge` 接在 `Hit_func` 已调用 `sub_6026D42C` 读出命令、确认 `Active` 之后；此时 `r4` 是玩家、`r5` 是 32 字节栈命令、`r8` 是表内序号。两条派生都限定装备 `+0x4D4=14`、风格 `+0x4D9=3`；J 防由 `j_guard_aed` 控制，仅处理玩家 `+0x24AC=0x81A`、序号 3。

- `act_st=0x10`，`act_no=0x44`，`act_param=0`。
- `cmd_type=8`，`cmd_param0=0x1B`，其它三个 `cmd_param` 为 0。
- 保留原表的 `PreFrame=20 / StartFrame=30 / EndFrame=0`：第 10 帧起可预输入，第 30 帧开始执行派生。

`0x1B` 是玩家输入位表中的第 27 位，不是动作 variant。与 asset 的修改版 `plcmd_w14_0x0010_071a` 一致。既不改共享 `rPlBaseCmd`，也不替换原表的资源引用；原版 `w14_m3.arc` 可以使用。

## 武士道盾突后的 X+A

盾突函数 `main+0x011DB280` 的初始化参数在 `main+0x0182CEC0` 等表中；参数 `a2[4]` 指向 `main+0x016B10A0`，其值为派生链 `0x7FA`。武士道资源对应第 42 张表，第 1 条原来引用 `036a`，窗口为 `Pre=10 / Start=34 / End=0`。

开启 `[moveset] bushi_shield_thrust_aed=true` 时，只将这条当前命令的前 24 字节写成上述 `act_no=0x44`、输入 `0x1B` 的命令，窗口保留：第 24 帧起可预输入、第 34 帧执行。原第 7 条 `071a` 保留，不复用用户 asset 修改后的共享命令。关闭本项时，J 防开关仍可单独生效。

两个开关在启动时编码为 `AED_ROUTE_FLAGS` 的 bit0 / bit1，在同一桥接里按链和序号选择。不增加全局 Rust 每帧回调，不重复覆盖 main 入口，不改其它风格。

覆盖的 `mov r0,#0 / movw r1,#999` 会补回，返回 `main+0x002EA884`。进入时的 NZCV、其它整数寄存器、栈和全部浮点寄存器保留，没有新增 Rust 每帧回调或对象分配。

393,216 组编译后 GP 指令执行与原洞输出一致；新增派生覆盖 110,592 组编译后 ARM 执行，验证四种独立开关组合、链／序号、六种风格、武器类型、窗口和寄存器。1,024 组配置选择及 70 组缓存失败恢复通过。4G 防性另验证 25,717 组威力、34,560 组分类，详见 [防性移植记录](2026-10-21_4G防性移植-report.md)。未进行实机战斗测试。
