// 一次收集全部启用的补丁，统一核对与回滚，避免不同功能安装一半。
use crate::{
    config::Config,
    patch::{self, BranchPatch, WordPatch},
};
use alloc::vec::Vec;

pub struct Plan {
    pub words: Vec<WordPatch>,
    pub branches: Vec<BranchPatch>,
}

/// 仅初始化 NRO 的返回槽，不写 main；也供主机测试检查开关组合。
pub unsafe fn selected(_game_base: u32, config: &Config) -> Plan {
    let mut plan = Plan {
        words: Vec::new(),
        branches: Vec::new(),
    };
    // 资源/动作注册与风格菜单常驻，功能总开关不影响这些公共内容。
    crate::style::collect(&mut plan.branches);
    if config.enabled {
        unsafe { crate::moveset::collect(config, &mut plan.words, &mut plan.branches) };
        if !config.styles.guard_performance.is_empty() {
            crate::guard::collect(&mut plan.branches);
        }
        crate::saed::patches::collect(&config.saed, &mut plan.words);
    }
    plan
}

/// 返回 false 时停止后续入口钩子安装，第三 hit 不会缺少配套 HDP 加载器。
pub unsafe fn install(game_base: u32, config: &Config) -> bool {
    let plan = unsafe { selected(game_base, config) };
    unsafe { patch::install("moveset-to-4g", game_base, &plan.words, &plan.branches) }
}
