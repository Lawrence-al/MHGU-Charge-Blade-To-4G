# HDP 与 J 防派生实现报告

本次完成：新增 `Hitdata-to-4G`，加载时复现当前 asset 的 `0x00～0x4E`，按用户后续更正将 `0x4F` 写回原版完整记录；`Moveset-to-4G` 仅修改武士盾斧 J 防成功后的 X+A 临时派生命令，目标为 `actionid=0x44`。两个功能都支持相应游戏资源使用原版。

## 本地证据

| 编号 | 输入／位置 | 观察 |
|---|---|---|
| E-01 | GU 原版与 asset `pl_we14_hitdata.4E2EF008` | 同为 4496 字节、80 条、每条 56；21 条记录共 28 个字节不同，含 0x4F |
| E-02 | `main+0x00270C18` / IDA `sub_60270C18` | load 完成后 `resource+0x64` 为完整 HDP，`+0x68` 为条目区；卸载使用原 HDP 分配器 |
| E-03 | 原版 main `.data` 中 `0x01747CA0` 虚表 | `+0x2C` 的 load 指针为 `main+0x00270C18`；槽地址为 `0x01747CCC` |
| E-04 | 原版 `w14_m3.arc` 与 asset `plcmd_w14_bushi` | 第 74 张表（链 2074 / 0x81A）的第 3 条由 `072a` 换为 `071a`；Pre=20 / Start=30 / End=0 未变 |
| E-05 | 原版与 asset `plcmd_w14_0x0010_071a` | 文件 +0x1FE：act_no 0x47→0x44；+0x210：cmd_param0 3→27；act_param 仍为 0 |
| E-06 | `Hit_func` / `main+0x002EA864～0x002EA884` | 调用 `sub_6026D42C` 读出 32 字节栈命令，确认返回 1 且 Active 后，r4=玩家、r5=命令、r8=序号 |
| E-07 | `sub_602731B4`、`sub_6026D018`、`sub_602ECEFC` | 命令 +0/+2/+4/+6 为 act_st/no/param 与 cmd_type；+8 为 cmd_param0；cmd_type=8 通过 `sub_60285E78` 检查玩家输入位表 |
| E-08 | 原版／asset 的 0x4F 与 SAED `NEW_50` | asset 0x4F 与 NEW_50 全部 56 字节相同；原版记录开头／持续／动作值为 106／6／55，asset 与 NEW_50 为 0／8／100 |

原版 ARC 记录为 12 字节头和 80 字节目录项，条目由 zlib 解压。XFS 定义及对象长度被解析为 116 张派生表，逐项确认表 74 的命令。asset 在表 12、42 中还存在其它更改，本次未把这些改动加入 NRO。

## 发现到实现

F-01（E-01～E-03、E-08）：HDP 可以在原生 load 之后原地复制记录。直接重复挂钩 `0x00270C18` 会覆盖 SAED 的入口补丁，因此 P-01 使用加载虚表槽，先调用同一 main 入口，再应用 79 条 asset 和原版 0x4F 完整快照。仅跳过 asset 0x4F 无法恢复已修改文件中的值，因此必须主动写回原版快照；原版 load、SAED 扩容和新记录 0x50 都保留。写入的是 main 数据中的函数指针，NRO 安装顺序不影响原入口的挂钩。

F-02（E-04～E-07）：用户资源通过替换共享 `071a` 并修改按键字段实现派生。这会改变其它引用该资源的表。P-02 在 `main+0x002EA87C` 跳入 NRO，只对武士（装备 +0x4D9=3）盾斧（+0x4D4=14）、链 0x81A、序号 3 的本次栈命令改字段：`act_st=0x10, act_no=0x44, act_param=0, cmd_type=8, cmd_param0=0x1B`，其余三个命令参数为 0。资源引用和共享对象不改，原第 10 帧起预输入、第 30 帧执行的窗口继续使用。

`0x1B` 是输入位编号 27：原 `sub_60285E78` 按 `player+5156+4*(编号>>5)` 取一个 32 位值，测试 `1<<(编号&31)`。它不是动作 variant。桥接保存／恢复 APSR，补回被覆盖的 `mov r0,#0; movw r1,#999` 后返回 `main+0x002EA884`。没有新增逐帧 Rust 函数或内存分配。

## 逐条数据差异

| ID | 原动作值 → asset 动作值 | 其它更改（记录内偏移） |
|---|---|---|
| 04、0C、0D、0F、1E、3F | 42→47 | 无 |
| 05、40 | 33→40 | 无 |
| 14 | 45→60 | 无 |
| 15、1F | 75→90 | +0x0E：0→1 |
| 18 | 90→99 | 无 |
| 21 | 90→100 | +0x0E：0→1 |
| 24 | 100→99 | 无 |
| 3A | 70→95 | 无 |
| 43 | 35→60 | 无 |
| 44、4A | 70→90 | +0x0E：0→1 |
| 47 | 80→89 | 无 |
| 4D | 80→99 | 无 |
| 4F | asset 为 55→100；NRO 恢复 55 | asset 的 +0：106→0、+2 持续：6→8；NRO 写回原版完整记录 |

动作值字段为记录 +4 的 u16。新 NRO 复制 79 条完整 asset 记录，另写回原版 0x4F 完整记录，因此无需逐字段重写或修改本地文件。0x50 的持续 8、动作值 100 不改。

## 复现与验证

```bash
bash "build.sh"
node "tests/verify.js"
node "tests/verify-install.js"
node "tests/verify-j-guard.js"
node "tests/verify-hitdata.js" "D:/Work/dump-classicMH/MHGU dump"
node "tests/verify-saed.js" "D:/Work/dump-classicMH/MHGU dump" "D:/Work/dump-classicMH/MH4G dump/4gv1.2.cia/ExeFS.elf"
node "tests/verify-shield-thrust.js"
```

- J 防：13,824 组实际 NRO 指令执行；目标字段从 asset XFS 独立提取，验证风格、武器、链、序号、两组加载基址与全部 16 种 NZCV。修改范围、窗口、其它寄存器及 VFP 均符合要求。
- HDP：原版与 asset 两种输入经真实 `data.rs` 处理后，0x00～0x4E 与 asset、0x4F 与原版逐字节一致；两个应用／扩容顺序、重复应用、计数、0x50 和尾部哨兵通过。实际 `load.rs` 仅对指针宽度与对齐做主机适配，96 组检查保留返回值、调用一次原函数、筛选路径、重载和原扩容。
- 共存：四个 NRO 的写入范围不重叠，main 代码洞不写入；31 处原有直接补丁值不变。Moveset 缓存失败恢复为 24 组，SAED 为 42 组。
- 原有功能：GP 393,216 组、盾突入口 576 组／选择器 16 组、4G 首爆／时间／坐标及 SAED 浮点 ABI 验证全部通过。

实现保持加载时处理 HDP、按现成临时记录修改 J 防的简单结构；HDP 路径匹配提取为两插件共用函数。资源及玩家／装备指针沿用原版有效对象的前提，未添加例行空指针判断。验证覆盖离线代码和二进制，尚未进行游戏战斗测试。
