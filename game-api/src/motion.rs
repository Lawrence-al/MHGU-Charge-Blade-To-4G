//! 玩家动画与派生查询的时间视图。只在派生查询期间临时提供旧大解时间。
use crate::{Game, abi, memory, player::ChargeBladePlayer};

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PlayerMotion(ChargeBladePlayer);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommandClock {
    blend_wait: f32,
    frame: f32,
    end: f32,
    loop_start: f32,
    tick_base: f32,
    tick_delta: f32,
}

impl CommandClock {
    /// 收斧发生在攻击动画结束后；保持终帧即可延续End=0的派生，有限窗口仍关闭。
    /// 不重新触发IS_ack的跨帧事件，不把中立动画的第0帧交给大解派生表。
    pub const fn finished(end: f32) -> Self {
        Self {
            blend_wait: 0.0,
            frame: end,
            end,
            loop_start: 0.0,
            tick_base: end,
            tick_delta: 0.0,
        }
    }
}

impl PlayerMotion {
    pub const AXE_IDLE: u32 = 5020;

    pub const fn new(player: ChargeBladePlayer) -> Self {
        Self(player)
    }

    pub unsafe fn end_frame(self) -> f32 {
        unsafe { memory::read_f32(self.0.address() + 0x504) }
    }
    pub unsafe fn blend_remaining(self) -> f32 {
        unsafe { memory::read_f32(self.0.address() + 0x4F4) }
    }
    pub unsafe fn set_action_state(self, state: u8) {
        unsafe { memory::write_u8(self.0.address() + 5, state) };
    }

    /// 从游戏入口进入原大解更新，保留其它NRO已经安装的表现挂钩。
    pub unsafe fn update_discharge(self, game: Game, variant: u32) -> u32 {
        let update: unsafe extern "C" fn(u32, u32) -> u32 = unsafe { game.function(0x011DABC8) };
        unsafe { update(self.0.address(), variant) }
    }

    pub unsafe fn clock(self) -> CommandClock {
        let at = |offset| unsafe { memory::read_f32(self.0.address() + offset) };
        CommandClock {
            blend_wait: at(0x4F4),
            frame: at(0x500),
            end: at(0x504),
            loop_start: at(0x508),
            tick_base: at(0x14D0),
            tick_delta: at(0x14D4),
        }
    }
    pub unsafe fn set_clock(self, clock: CommandClock) {
        for (offset, value) in [
            (0x4F4, clock.blend_wait),
            (0x500, clock.frame),
            (0x504, clock.end),
            (0x508, clock.loop_start),
            (0x14D0, clock.tick_base),
            (0x14D4, clock.tick_delta),
        ] {
            unsafe { memory::write_u32(self.0.address() + offset, value.to_bits()) };
        }
    }

    /// 原归中的姿态选择部分。保留斧模式和当前命令表，不提交待机动作。
    /// 6053A1DC对正混合参数加2；配置范围3..120，减2后交给原生倒计时。
    pub unsafe fn blend_to_axe_idle(self, game: Game, frames: u16) -> u32 {
        unsafe {
            abi::animation_call(
                game.address(0x00282120),
                self.0.address(),
                Self::AXE_IDLE,
                0,
                (f32::from(frames) - 2.0).to_bits(),
            )
        }
    }

    /// 归中完成后才调用原包装。它明确传入0/4/0及零栈参数，保留原待机/移动分流。
    pub unsafe fn return_to_neutral(self, game: Game) -> u32 {
        unsafe { abi::animation_call(game.address(0x002B6E5C), self.0.address(), 0, 0, 0) }
    }

    /// 与原生离开攻击的路径相同：将两个HDP控制器提交给原注销队列。
    pub unsafe fn stop_attack_controllers(self) {
        for offset in [0x3020, 0x30D8] {
            let controller = self.0.address() + offset;
            let stop: unsafe extern "C" fn(u32) -> u32 =
                unsafe { memory::method(controller, 0x1C) };
            unsafe { stop(controller) };
        }
    }
}
