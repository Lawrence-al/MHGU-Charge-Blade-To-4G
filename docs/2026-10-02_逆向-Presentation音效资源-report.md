# Presentation 音效资源修改报告

分析日期：2026-10-02。范围仅为用户指定的 SRQ / PTS 修改迁入 Presentation NRO。工具为现有 `main.c`、DTI 字段表、原始 NSO 解压、devkitARM objdump、Rust 编译器和 Node。

2026-10-03 更新：参数快照已直接保存到 `resource.rs`，构建也不依赖 asset；参考改版按用户当前目录放在 `weapon-nro` / `sound-nro`。同一天新增的盾突闪电说明见 `docs/Presentation.md`。

## 结论与使用

两份资源的当前 asset 改动可以由 Forge 的加载钩子复现。修改函数处理 dump 原版后，SRQ 全部 6640 字节与 asset 一致；PTS 的 63 条对象记录映射回文件后，全部 4671 字节与 asset 一致。两份 asset 未修改或删除。

新产物为 `presentation_to_4g.nro`，对应资源可使用原版。只替代这两份文件的当前差异，音库文件及其它 PSL / PEL / 动作资源仍按现有配置使用。游戏内听感及真实加载过程尚未运行验证。

## 当前字段差异

SRQR version 5 的记录起点为 `0x34`，每条记录 `0x84` 字节，共 49 条。下表偏移均相对 request 记录起点。

| request | 偏移 | 原版 → asset / NRO |
|---|---|---|
| `0x4A` | `+0x24 / +0x28` | 两个 f32 音量：`-3.9 → -1.9` |
| `0x4A` | `+0x34` | u16 重播间隔：`600 → 0` |
| `0x54` | `+0x02` | cue：`0x7FFF → 0x7F4A`（低字节 cue `0x4A`） |
| `0x54` | `+0x08` | 请求操作：`2 → 1`，改为播放 |
| `0x54` | `+0x12` | u16 分组第三项：`10 → 200` |
| `0x54` | `+0x14` | u8 优先级：`0 → 110`；下一字节 `+0x15` 为替换方式 1 |
| `0x54` | `+0x18` | u32：`0 → 1` |
| `0x54` | `+0x24 / +0x28` | 两个 f32 音量：`0 → -2.4` |
| `0x54` | `+0x2C` | u16：`0 → 600` |
| `0x54` | `+0x38 / +0x3C / +0x40` | 三个 i32：`-1/-1/-1 → 16/16/0` |
| `0x54` | `+0x54` | 音库索引：`-1 → 0` |

`0x54` 直接复制当前 asset 的整条记录，未列出的字节也保持 asset 原值。2026-10-03 依据 Start_sfx 再核对并纠正旧字段标注：`+0x14` 是优先级，不是启动延迟；`+0x30` 才是启动延迟，当前为 0；`+0x34` 是重播间隔。`+0x12` 是分组第三项，不是淡出时间。参数快照字节没有因本次文档纠正而变化。

PTS 按以下匹配键修改，不根据旧对话中的其它改动推断规则。

| group | effectId | 原 request | 原事件 | 改动 |
|---|---|---|---|---|
| SEQUENCE（1） | `0x226` | `0x4A` | 创建（1） | 事件置 0 |
| SEQUENCE（1） | `0x226` | `0x56` | 结束（3） | 事件置 0 |
| SEQUENCE（1） | `0x258/0x259/0x262/0x263` | `0x48` | 创建（1） | request 改 `0x47` |
| UNIQUE（2） | `0x1F4` | `0x54` | 结束（3） | request 改 0，事件保留 3 |
| UNIQUE（2） | `0x258/0x259/0x25D/0x25E/0x262/0x263` | `0x48` | 创建（1） | request 改 `0x47` |

## Evidence

| ID | 观察与来源 | 证据 |
|---|---|---|
| E-001 | dump 原版与当前 asset 的完整文件对比；下方命令可复现 | SRQ 41 个差异字节；PTS 13 条、13 个差异字节；修改函数输出逐字节等于 asset |
| E-002 | `D:/Work/dump-classicMH/MHGU dump/exefs/main.c` 的 `sub_60B67A0C / sub_60B67AE4`；原始 NSO ARM 指令 | 原 loader 验证 SRQR magic / version 5 后调用绑定函数；绑定函数读取 resource `+0x64` 的原始缓冲区，将每条 `+0x54` 音库索引解析并写入 `+0x58` 引用 |
| E-003 | 同一 `main.c` 的 `sub_604D923C / sub_604D8FB0`，`mhgu_dti_dump.h` 的 `rFestaPelTiedSe / cFestaPelTiedSe` | PTS load 先反序列化；resource `+0x68` 为数量，`+0x74` 为对象指针数组；对象 `+0xC` request、`+0xE` 事件 |
| E-004 | `sub_608875F0` 原始 ARM 指令 | `ADD r0,r0,#8; BX lr`，路径在 resource 内嵌 `+8`，不是路径指针 |
| E-005 | `forge-main/source/forge/hook.c` 与目标函数前 8 字节 | Forge ARM trampoline 原样搬运两条指令；两个目标入口均为无 PC 相对寻址的压栈／寄存器操作，可搬运 |
| E-006 | `cargo forge build --release`；`node -e 'require("./tests/verify").verifyTables()'` | Presentation release 编译成功，NRO 32768 字节；三个插件的写入范围无重叠 |

输入文件 SHA-256（完整内容）：

| 文件 | SHA-256 |
|---|---|
| dump 原版 SRQ | `985eabb597cafccc6a97a989faffe2fc9bbfc772648fac0b532c40a566025ab7` |
| asset SRQ | `66eb2dcff5df12c97823ef58c2e83a41bbb453ecf96f6da42c73cfe1d0dddd4b` |
| dump 原版 PTS | `1a71b4026ee7ca7e0564297899a2a9db3794de1f525287fb32cac642d819934a` |
| asset PTS | `f9e39349639a329be1fbcd4d3db122496e160d910e2d695a14d18697371d0b23` |

原始入口指令，以模块偏移表示：

```text
0x00B67AE4: E92D47F0  E1A04000  ; push {r4-r10,lr}; mov r4,r0
0x004D923C: E92D41F0  E24DDD0D  ; push {r4-r8,lr}; sub sp,sp,#0x340
```

## Findings

| ID | 结论 | evidence_ids | 验证程度 |
|---|---|---|---|
| F-001 | SRQ 应在绑定前修改；否则把 `0x54` 改为播放操作后，音库引用仍未建立 | E-001、E-002 | 二进制与原函数调用链已验证 |
| F-002 | PTS 必须按反序列化后对象字段修改，XFS 文件偏移不等于对象偏移 | E-001、E-003 | 全部 63 条记录与文件差异已验证 |
| F-003 | 两个钩子只在资源加载期间工作；匹配路径后才修改对应盾斧资源 | E-002、E-003、E-004、E-005、E-006 | 源码、编译产物及挂钩入口已验证；游戏运行待用户测试 |

## Path

P-001（callflow，关联 E-001 / E-002，F-001）：读取 common SRQR → 原 loader 验证格式 → NRO 在 `0x00B67AE4` 入口按 resource 路径匹配 → 修改 `0x4A` 字段和 `0x54` 记录 → 原函数建立音库引用和内部索引 → 原 loader 建立 request 查找表 → 游戏按新数据调用 cue。

P-002（callflow，关联 E-001 / E-003 / E-004，F-002）：PTS loader `0x004D923C` → 原 XFS 反序列化 → 成功后 NRO 匹配 `sound/pts/pl/w14_000` → 遍历对象指针数组 → 依据 group / effectId / request / 事件修改字段 → 保留原返回值 → 后续特效事件读取新 request。

两条路径均沿用原 loader 成功后缓冲区及条目指针有效的前提；未新增空指针判断。quest 和 shell 的原版同名 PTS 内容一致，使用同一内部资源路径时适用相同规则。

## 复现

在本项目根目录的 MSYS2 Bash 中执行，依赖本机 Rust 和已有 dump 文件：

```bash
"/c/Users/Lawrence/.cargo/bin/rustc.exe" --edition=2024 "tests/verify-resources.rs" -O -o "tests/verify-resources.exe"
"./tests/verify-resources.exe" \
  "/d/Work/dump-classicMH/MHGU dump/romfs/nativeNX/arc/weapon/w14/snd武器音效/mat-arc/sound/se/pl/wp/gaxe/com/snd_wp_gaxe_com.srq" \
  "assets/romfs/nativeNX/arc/weapon-nro/w14/snd/mat.arc/sound/se/pl/wp/gaxe/com/snd_wp_gaxe_com.srq" \
  "/d/Work/dump-classicMH/MHGU dump/romfs/nativeNX/arc/player/quest/w14-arc/sound/pts/pl/w14_000.4541367C" \
  "assets/romfs/nativeNX/arc/player/quest/w14.arc/sound-nro/pts/pl/w14_000.4541367C"
node -e 'require("./tests/verify").verifyTables()'
```

独立 Rust 程序直接编入本次 `resource.rs`，检查完整文件一致、重复应用一致、其它对象字段未变和资源路径过滤。它不模拟 XFS 解码器本身、Forge trampoline、游戏资源管理器或声音播放。

本次检查完成：范围与当前 asset 差异已核对；两个真实 ARM 入口及 DTI 字段已核对；Rust 资源验证通过；Presentation release 构建成功；插件写入范围验证通过。未提交 Git，也未修改 Moveset / SAED 源码。
