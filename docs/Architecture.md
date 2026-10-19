# 当前代码结构

游戏字段、原生调用和功能规则分为三层：`game-api` 定义对象及调用方法；Presentation／Moveset 保存各自规则与运行状态；`hooks.rs` 将原始 ARM 参数转换成对象后转发。两个插件继续各自产出一份 NRO，没有第三个插件，也没有跨 NRO 的状态或调用顺序依赖。

## 文件职责

| 目录／文件 | 职责 |
|---|---|
| `game-api/src/player.rs` | `ChargeBladePlayer`、`Equipment`、`Shield`，玩家字段及盾牌引用虚函数 |
| `game-api/src/command.rs` | `CommandResource`、`CommandTable`、28 字节 `TableHeader`，当前派生资源及记录指针 |
| `game-api/src/sound.rs` | `SoundSequence`、`SeOwner`、`SeRequest`、`MssEvent`，音序与播放池调用 |
| `game-api/src/effect.rs` | `EffectNode`、`WeaponEffectShell`、命名特效参数、原生构造／析构／创建 |
| `game-api/src/shell.rs` | `SaedShell`、`ShellParams`、`GroundProbe`，瓶爆进程字段、贴地及原瓶伤调用 |
| `game-api/src/resource.rs` | 资源对象、SRQR 记录视图、PTS 运行时视图、HDP 布局、原游戏分配器 |
| `game-api/src/memory.rs` | 非对齐读、volatile 读写、内部虚函数获取 |
| `game-api/src/abi.rs` | ARM32 浮点寄存器及栈参数转换 |
| `Presentation/src/action.rs` / `tasks.rs` | `ActionRunner` 管理两份任务状态，静态任务表保存帧与规则 |
| `Presentation/src/state.rs` | `PresentationState` 管理玩家／音序／SE 缓存与原子计数 |
| `Presentation/src/effect.rs` | 特效持续时间登记、到期停止、盾突红盾条件 |
| `Presentation/src/sound.rs` / `resource.rs` | MSS 拦截与播放规则、盾斧 SRQ／PTS 修改规则 |
| `Presentation/src/debug.rs` | 特效 ID 和任务表日志，默认不编译、不安装日志钩子 |
| `Moveset/src/moveset.rs` | 按功能命名的直接补丁、保持寄存器的 ARM 桥接 |
| `Moveset/src/overpowered.rs` | 原派生初始化后为每玩家合并表，保留原大解与红盾记录，追加当前风格后摇 |
| `Moveset/src/saed/schedule.rs` / `phial.rs` | 纯时间／轨迹计算、瓶爆更新和记录管理 |
| `Moveset/src/hitdata/mod.rs` / `records.rs` | 单一加载扩容入口、asset／原版 0x4F／新增 0x50 快照 |
| 两项目 `config.rs` / `config_file.rs` / `ui.rs` | 纯 INI 解析与输出、启动读取一次、Forge 菜单编辑草稿 |
| `shared/config_io.rs` / `ini.rs` / `settings_menu.rs` | 通过共用存储模块读写当前用户存档根目录 INI、保留中文注释的输出、保存／放弃／默认值 |
| `shared/runtime_hook.rs` | 项目内无 context 钩子，提供清零的 76 字节返回区，容纳当前 Forge 的 24 字节 Hook |
| `shared/patch.rs` / `Moveset/src/patches.rs` | 补丁核对与安装、启动配置选择 |

表中 Presentation 和 Moveset 分别指 `Presentation-to-4G`、`Moveset-to-4G`。

## 游戏对象的写法

大型对象用 `#[repr(transparent)] struct` 保存一个 32 位地址，`impl` 提供字段方法。类型本身不复制玩家或创建新盾牌，也不持有完整游戏对象的独占 Rust 引用。

```rust
let player = ChargeBladePlayer::from_address(player_actor);
let frame = unsafe { player.motion_frame() };
let shield = unsafe { player.shield() };
let reference = unsafe { shield.reference() };
let kind = unsafe { player.phial_kind() };
```

字段偏移只在对应对象文件维护。`player.sound_sequence()` 返回内嵌对象地址 `player+0x111C`；`sequence.owner()` 读取 `sequence+0x164` 指针值。`player.shield()` 读取 `player+0x23A4`；盾牌 `reference()` 通过虚表 `+0x130` 取得引用。装备参数 27 与瓶爆参数的资源子类型分别由 `Equipment::phial_kind()` 和 `ShellParams::kind()` 解码，两者使用不同原始字段。

对象方法沿用原钩子中玩家、装备、盾牌及资源已经有效的前提；没有添加空指针兜底。游戏拥有这些对象，地址对象只在对应生命周期内使用。节点和 shell 的析构钩子继续撤销各自计时记录。

## 已知小型布局

`EffectCreateParams` 使用 `#[repr(C, align(16))]`。外层参数为 0x10 字节，内部 `ProofEffectParams` 为 0xC8 字节；有效参数区共 0xD8，16 字节对齐后的 `sizeof` 为 0xE0，与旧数组结构一致。未知区域用保留字节表示。

```rust
let mut params = EffectCreateParams::zeroed();
unsafe { params.construct(game) };
params.set_priority(3);
params.attach_to(reference, 0);
// 创建后仍调用游戏析构；构造／使用／析构期间参数保留在调用点栈上。
unsafe { params.destruct(game) };
```

优先级、mask、骨骼、附加模式、父引用均有字段名和编译期偏移断言。测试以 0／0xA5／0xFF 填充完整参数，与旧数组写法逐字节对照，检查未知字节保留。

SRQR／HDP 是文件记录，PTS 是 XFS 解析后的 24 字节对象。它们使用各自记录长度和字节视图，不将文件缓冲区当成完整 Rust 游戏类。HDP 扩容仍使用原游戏 MtAllocator，并更新两个资源指针。

## 原生调用及性能

表现字段保留原来的 `read_unaligned`，瓶爆字段保留 `read_volatile`／`write_volatile`。共同内存模块用 `read_*` 和 `load_*` 区分这两种语义。字段方法不分配对象，没有 trait 对象动态分派。release 继续使用 LTO。

ARM/VFP 转换指令和裸汇编桥接保留；桥接的玩家／装备偏移改为关联常量，在汇编时解析为原立即数。Forge `hook(offset=...)` 当前要求字面量，因此钩子入口偏移仍就地标注。补丁指令及特效／音效帧由功能模块保存。

Presentation 的原子缓存、ActionRunner 的原子标记、特效与瓶爆记录的锁保持原有职责。调用游戏函数前释放记录锁，避免重入析构死锁。Moveset 在瓶爆初始化时保存 `Game` 基址，后续原生调用不重复查询 Forge 基址。

## 调试及验证

普通构建关闭 `debug-logs`。该 Cargo feature 仅用于开发调试，不属于默认发布构建；原来的 `cargo forge build --features ...` 命令不受固定版本打包工具支持，不能作为可执行的构建入口。发布版本统一从仓库根目录运行 `bash build.sh`。

根目录 `test.sh` 保留已有开关／回滚、HDP、4G 时序／轨迹、ARM 寄存器／栈、NRO 导入检查，新增了新旧实际 ActionRunner 的 7920 次调用对照和共用特效参数字节检查。当前生产对象方法也在主机测试中执行，仅读写地址空间及原生入口替换为测试替身。主机共用库缓存记录 rustc 版本，避免 stable／nightly 的 rlib 混用。

动作回归只需的旧执行器与常量已收进 `tests/data/presentation-reference.rs`，任务表使用固定摘要对照。构建和测试不再读取 `legacy-builds/`、`work/` 或父目录的临时工具。默认配置、资源修改规则、音效帧、特效持续帧、瓶爆表和补丁值均保持原设置；已停用的动作／命中／受击清理实现仍保留在特效模块注释中。


## 独立构建检查（2026-10-19）

Forge SDK 与 cargo-forge 固定为同一 Git 提交 `f2af0622a2a0aa291ffbd3d2f7628c0e2d96d220`，两个 Cargo.lock 一并保留。当前用户存档读写实现在仓库内，只内嵌 `conf/` 下的两个盾斧 INI；不会为编译其它插件读取模板。HDP 的唯一输入为 `assets/romfs/nativeNX/arc/player/quest/w14.arc/hit/pl_we14_hitdata.4E2EF008`。

| Evidence | Finding | Path |
|---|---|---|
| `bash build.sh`，两个 release ELF 与 `dist/*.nro` | Forge 来源为固定 Git 提交，构建不访问本机工作区 SDK | 项目内准备打包工具 → 编译两项目 → 输出到 dist |
| `bash test.sh` | 默认回归不需要历史目录、私有工具或游戏文件 | ELF/NRO 检查 → 配置和菜单 → 实际 ARM 与动作回归 |
| `bash test.sh "MHGU dump" "4G ELF" "Forge NSO"`（需自行准备原版文件） | 可额外检查实际导入提供者、GU 原指令、HDP、4G 时序和轨迹 | 只读本地原版文件 → 比对生成的两个 NRO |

测试不代替实机或模拟器中的战斗验证。普通测试不自动部署文件，不修改游戏存档。测试报告输出到各项目的 `target/build-verification.json`。

隔离复验使用了无相邻 SDK、目录名含空格的源码副本：`bash build.sh --offline` 完成两个 release 构建。最终工作副本也完成 PowerShell `-Offline` 构建、基础与原版数据比对。`build.ps1` 使用带 BOM 的 UTF-8，确保 Windows PowerShell 5 能识别中文注释后的参数声明。部署测试仅在临时插件目录复制两个 NRO，并核对其它插件和 INI 未变。

