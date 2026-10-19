// 按帧音效／特效任务和 MSS 拦截表；仅保存数据，运行状态在 action.rs。

/// 一条"到了某个阶段/某一帧要做什么"的任务
pub struct ActionTask {
    /// 动作阶段：0 = 起手那一帧（只跑一次），1 = 之后每帧
    pub action_state: u32,
    /// 变体位图：bit i 表示 variant == i 时命中（例如 1|7 → `MASK_1_7`）
    pub variant_mask: u32,
    /// 触发帧：和 `mMotion[0].mFrame` 比较；0.0 表示"进入这个阶段就执行"
    pub trigger_frame: f32,
    /// 触发后要做什么
    pub action: TaskAction,
}

/// 任务内容。以后要"到帧顺便干别的"就往这里加分支，不用改判定逻辑。
pub enum TaskAction {
    /// 播一条 SE，三个字段与 mss `0x66` 事件一一对应
    PlaySe {
        /// SE 请求号（mss 0x66 第 2 个 u16）
        request_id: u16,
        /// 库号（mss 0x66 第 1 个 u16；GU 实测 0/1，4G 那边 API 的 5 不是同一套编号）
        bank: u16,
        /// 发声挂点索引（mss 0x66 第 3 个字段）
        attach_index: i32,
    },
    /// 创建 UNIQUE 特效，父对象使用当前玩家的盾牌、骨骼使用 0
    CreateShieldEffect {
        /// 榴弹瓶使用的特效 ID
        effect_id: u16,
        /// 强属性瓶使用的特效 ID
        element_effect_id: u16,
        /// 从创建起累计的特效更新帧数，不因动作切换或命中提前停止
        duration_frames: f32,
    },
    // 以后扩展示例（现在还没用）：
    // CameraShake { kind: u32, flag: u32 },
    // ReadHitData  { slot: u32, id: u32, flags: u32 },
    // CreateShell  { shell: u32, flag: u32 },
}

/// variant 位图：bit i = variant i
const fn variants(a: u32, b: u32) -> u32 {
    (1 << a) | (1 << b)
}

/// variant 0|6
const MASK_0_6: u32 = variants(0, 6);
/// variant 1|7
const MASK_1_7: u32 = variants(1, 7);
/// variant 2|9
const MASK_2_9: u32 = variants(2, 9);
/// variant 2|4|9
const MASK_2_4_9: u32 = (1 << 2) | (1 << 4) | (1 << 9);
/// variant 3|8
const MASK_3_8: u32 = variants(3, 8);
/// variant 2|3|8|9
const MASK_2_3_8_9: u32 = (1 << 2) | (1 << 3) | (1 << 8) | (1 << 9);
/// variant 1|7|2|9（大姐 + 超解共用的起手音）
/// 注：只有起手 0x48 那条（已注掉）在用这个掩码，所以先允许它没有使用者
#[allow(dead_code)]
const MASK_1_7_2_9: u32 = (1 << 1) | (1 << 7) | (1 << 2) | (1 << 9);

/// 盾斧瓶爆动作的按帧任务表
///
/// 帧号含义与 4G 的 `IsActionFrameReached(player, 帧)` 一致（同一把"动作帧"尺子）；
/// `variant` 编号两作对应：0/1 = 大姐(AED)、2/3 = 超解(SAED)，GU 另有 4~9 的子变体。
pub const ACTION_TASKS: &[ActionTask] = &[
    // ---- 起手那一帧（actionState == 0）----
    // 起手音：variant 1|7|2|9 都要（attach_index 先统一用 2 试，原值 8）
    // 2026-09-17：按需求**暂时停用**，要恢复就把下面整段解注
    /*
    ActionTask {
        action_state: 0,
        variant_mask: MASK_1_7_2_9,
        trigger_frame: 0.0,
        action: TaskAction::PlaySe {
            request_id: 0x48,
            bank: 0,
            attach_index: 2,
        },
    },
    */
    // ---- 后续帧（actionState == 1）----
    // 大姐：第 1 帧创建盾牌闪电，持续 30 帧。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_1_7,
        trigger_frame: 1.0,
        action: TaskAction::CreateShieldEffect {
            effect_id: 0x258,
            element_effect_id: 0x259,
            duration_frames: 30.0,
        },
    },
    // variant 2|9：第 1 帧创建盾牌闪电，持续 30 帧。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_9,
        trigger_frame: 1.0,
        action: TaskAction::CreateShieldEffect {
            effect_id: 0x262,
            element_effect_id: 0x263,
            duration_frames: 30.0,
        },
    },
    // 大姐：第 10 帧播放 4G 特效 0x2B 的伴生音效 0x4A（走 sub_1B5C64 的表）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_1_7,
        trigger_frame: 10.0,
        action: TaskAction::PlaySe {
            request_id: 0x4A,
            bank: 0,
            attach_index: 2,
        },
    },
    // 大姐：第 40 帧提交停止请求，淡出时间由 SBKR 决定。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_1_7,
        trigger_frame: 40.0,
        action: TaskAction::PlaySe {
            request_id: 0x56,
            bank: 0,
            attach_index: 2,
        },
    },
    // variant 2|4|9：第 30 帧提交停止请求，结束上一动作的 0x4A。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_4_9,
        trigger_frame: 30.0,
        action: TaskAction::PlaySe {
            request_id: 0x56,
            bank: 0,
            attach_index: 2,
        },
    },
    // 大姐：第 66 帧（4G 在这一帧播 cue 0x4B）
    // attach_index 用 2：和 GU 自己那条大姐砸地音效（key 0x1412 第 62 行 a4=2）取同一个挂点，
    // 不然声音会从别的节点发出、听着"很怪"。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_1_7,
        trigger_frame: 66.0,
        action: TaskAction::PlaySe {
            request_id: 0x4B,
            bank: 0,
            attach_index: 2,
        },
    },
    // 大姐（另一路）：第 62 帧（和 GU 自己 mss 那条 0x43 的帧一致）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_0_6,
        trigger_frame: 62.0,
        action: TaskAction::PlaySe {
            request_id: 0x43,
            bank: 0,
            attach_index: 2,
        },
    },
    // 超解：第 44 帧（attach_index 先统一用 2 试，原值 12）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_3_8_9,
        trigger_frame: 44.0,
        action: TaskAction::PlaySe {
            request_id: 0x45,
            bank: 0,
            attach_index: 2,
        },
    },
    // 超解：第 52 帧（attach_index 先统一用 2 试，原值 12）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_9,
        trigger_frame: 52.0,
        action: TaskAction::PlaySe {
            request_id: 0x54,
            bank: 0,
            attach_index: 2,
        },
    },
    // 超解：第 108 帧（瓶爆/冲击，和 4G 的判定帧一致；attach_index 先统一用 2 试，原值 12）
    // 4G 这里原本是 0x57（cue 0x55 = gaxe_shield_atk），但 GU 既没有 0x57 记录、也没有这个样本，
    // 所以 2026-09-17 暂用 GU 现成的 0x55（cue 0x48 = gaxe_fx）顶替，先听感验证。
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_9,
        trigger_frame: 108.0,
        action: TaskAction::PlaySe {
            request_id: 0x55,
            bank: 0,
            attach_index: 2,
        },
    },
    // 超解：第 109 帧（第二条；attach_index 先统一用 2 试，原值 12）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_2_9,
        trigger_frame: 109.0,
        action: TaskAction::PlaySe {
            request_id: 0x4B,
            bank: 0,
            attach_index: 2,
        },
    },
    // 超解（另一路）：第 108 帧（attach_index 先统一用 2 试，原值 12）
    ActionTask {
        action_state: 1,
        variant_mask: MASK_3_8,
        trigger_frame: 108.0,
        action: TaskAction::PlaySe {
            request_id: 0x43,
            bank: 0,
            attach_index: 2,
        },
    },
];

/// variant 2|4（瓶爆子动作里这两个 variant 走"第 26 帧"那组）
const MASK_2_4: u32 = variants(2, 4);
/// 除 2|4 以外、0~15 的所有 variant（瓶爆子动作里走"第 50 帧"那组）
const MASK_NOT_2_4: u32 = 0xFFFF & !MASK_2_4;

/// 瓶爆子动作（sub_611DA204，= 4G 的 sub_CAC594）的按帧任务表
///
/// 这个动作的 variant **不区分有没有瓶**，除 0x4C 外的任务靠 [`constant::PLAYER_PHIAL_FLAG`]
/// （`*(u8*)(player+6)`，起手帧写入：有瓶 1 / 没瓶 0）门控；0x4C 无瓶时也执行。
/// 第 26/50 帧的两条音效都是 4G 特效 0x37（瓶爆特效）的伴生音效，使用 request 0x4A。
/// 配套 SRQ 将 0x4A 的同组起播间隔设为 0ms；声数上限仍为 1，后一次起播替换前一声。
/// 同样的阶段、variant 和帧数另外创建 0x258，挂到盾牌骨骼 0，持续 25 帧。
pub const BURST_ACTION_TASKS: &[ActionTask] = &[
    // variant 0|7：第 30 帧播放 0x4C，不要求有瓶。
    ActionTask {
        action_state: 2,
        variant_mask: variants(0, 7),
        trigger_frame: 30.0,
        action: TaskAction::PlaySe {
            request_id: 0x4C,
            bank: 0,
            attach_index: 2,
        },
    },
    // variant 2|4：第 26 帧
    ActionTask {
        action_state: 2,
        variant_mask: MASK_2_4,
        trigger_frame: 26.0,
        action: TaskAction::PlaySe {
            request_id: 0x4A,
            bank: 0,
            attach_index: 2,
        },
    },
    // 其它 variant（0~15 里除 2、4 以外）：第 50 帧
    ActionTask {
        action_state: 2,
        variant_mask: MASK_NOT_2_4,
        trigger_frame: 50.0,
        action: TaskAction::PlaySe {
            request_id: 0x4A,
            bank: 0,
            attach_index: 2,
        },
    },
    // variant 2|4：第 26 帧创建盾牌闪电，持续 25 帧。
    ActionTask {
        action_state: 2,
        variant_mask: MASK_2_4,
        trigger_frame: 26.0,
        action: TaskAction::CreateShieldEffect {
            effect_id: 0x258,
            element_effect_id: 0x259,
            duration_frames: 25.0,
        },
    },
    // 其它 variant（0~15 里除 2、4 以外）：第 50 帧创建盾牌闪电，持续 25 帧。
    ActionTask {
        action_state: 2,
        variant_mask: MASK_NOT_2_4,
        trigger_frame: 50.0,
        action: TaskAction::CreateShieldEffect {
            effect_id: 0x258,
            element_effect_id: 0x259,
            duration_frames: 25.0,
        },
    },
];

/// 一条"拦截"规则：命中就**不把这条 mss 事件交给游戏**（等效于删掉那一条音效）
///
/// 判定条件全部来自 mss 事件本身 + 音序控制器的帧，
/// 只要 `se_owner` 是盾斧玩家（`uPlayerQuest14`）就生效，不限本地/远端玩家。
pub struct BlockRule {
    /// 目标帧：满足 `上一帧 < frame <= 当前帧` 才算命中（和游戏自己的跨帧判定一致，掉帧也不会漏）
    pub frame: u32,
    /// mss 事件 opcode（0x66 = SE_Play）
    pub opcode: u8,
    /// 库号（mss 0x66 第 1 个 u16）
    pub bank: u16,
    /// SE 请求号（mss 0x66 第 2 个 u16）
    pub request_id: u16,
}

/// 要掐掉的音效：大姐（key 0x1412）第 62 帧的 0x43、超解（key 0x1405）第 112 帧的 0x43、
/// key 0x1413 第 24 帧的 0x4A。
///
/// 0x43 的这两个帧号在 pl_wp14 里是唯一的（其它动作上的帧号分别是 70/66/82）；
/// 0x4A 在这份 mss 里只有 0x1413 这一处。插件自主播放直接写 SE 池，不经过这里。
pub const BLOCK_RULES: &[BlockRule] = &[
    // 大姐：0x1412 第 62 帧 bank0 req0x43
    BlockRule {
        frame: 62,
        opcode: 0x66,
        bank: 0,
        request_id: 0x43,
    },
    // 超解：0x1405 第 112 帧 bank0 req0x43
    BlockRule {
        frame: 112,
        opcode: 0x66,
        bank: 0,
        request_id: 0x43,
    },
    // 0x1413：第 24 帧 bank0 req0x4A，改由 ACTION_TASKS 自主播放
    BlockRule {
        frame: 24,
        opcode: 0x66,
        bank: 0,
        request_id: 0x4A,
    },
];
