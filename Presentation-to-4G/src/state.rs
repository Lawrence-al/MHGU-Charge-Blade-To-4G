//! 本 NRO 的对象缓存与计数；原子访问和 SE 归属回退顺序沿用原实现。
use cb_game::{
    Game,
    player::ChargeBladePlayer,
    sound::{SeOwner, SoundSequence},
};
use core::sync::atomic::{AtomicU32, Ordering};

pub struct PresentationState {
    game_base: AtomicU32,
    player: AtomicU32,
    sequence: AtomicU32,
    owner: AtomicU32,
    last_owner: AtomicU32,
    dispatch_count: AtomicU32,
    play_count: AtomicU32,
}

pub static STATE: PresentationState = PresentationState {
    game_base: AtomicU32::new(0),
    player: AtomicU32::new(0),
    sequence: AtomicU32::new(0),
    owner: AtomicU32::new(0),
    last_owner: AtomicU32::new(0),
    dispatch_count: AtomicU32::new(0),
    play_count: AtomicU32::new(0),
};

impl PresentationState {
    pub fn initialize(&self, game_base: u32) {
        self.game_base.store(game_base, Ordering::Relaxed);
    }
    pub fn game(&self) -> Game {
        Game::new(self.game_base.load(Ordering::Relaxed))
    }
    pub fn set_player(&self, player: ChargeBladePlayer) {
        self.player.store(player.address(), Ordering::Relaxed);
    }
    pub fn observe_sequence(&self, sequence: SoundSequence, owner: SeOwner) {
        self.sequence.store(sequence.address(), Ordering::Relaxed);
        self.last_owner.store(owner.address(), Ordering::Relaxed);
        self.dispatch_count.fetch_add(1, Ordering::Relaxed);
    }
    pub fn set_owner(&self, owner: SeOwner) {
        self.owner.swap(owner.address(), Ordering::Relaxed);
    }

    /// 保留原有“缓存归属 → 玩家音序归属 → 最近归属”的顺序及尚未抓到对象时的返回。
    pub unsafe fn resolve_owner(&self) -> SeOwner {
        let owner = self.owner.load(Ordering::Relaxed);
        if owner != 0 {
            return SeOwner::from_address(owner);
        }
        let player = self.player.load(Ordering::Relaxed);
        if player != 0 {
            let owner = unsafe {
                ChargeBladePlayer::from_address(player)
                    .sound_sequence()
                    .owner()
            };
            if owner.address() != 0 {
                return owner;
            }
        }
        SeOwner::from_address(self.last_owner.load(Ordering::Relaxed))
    }

    pub fn count_play(&self) {
        self.play_count.fetch_add(1, Ordering::Relaxed);
    }
    #[cfg(feature = "debug-logs")]
    pub fn dispatch_count(&self) -> u32 {
        self.dispatch_count.load(Ordering::Relaxed)
    }
    #[cfg(feature = "debug-logs")]
    pub fn play_count(&self) -> u32 {
        self.play_count.load(Ordering::Relaxed)
    }
}
