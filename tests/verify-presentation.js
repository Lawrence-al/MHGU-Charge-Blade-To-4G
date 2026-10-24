// 表拆分必须保持原音效／粒子参数；辅助函数必须取内嵌对象而非其 vtable。
const {fs, path, root, modulePath, run} = require('./host');
const assert = require('assert/strict');
const gameModules = require('./game-host');
const crypto = require('crypto');
// 来自 2026-10-11 已验证任务表的固定摘要，不依赖已删除的历史目录。
const expectedHashes = {
  "ACTION_TASKS": "0074f2600bd03ebd9419db31addf9920080073cca98bff3942abfa7926f7255c",
  "BURST_ACTION_TASKS": "43e0c2f596c920deaa6dad598e82e6838386a32b9c909f902f949f11da204070",
  "BLOCK_RULES": "83c7814daaeb297a7a937d3a912e4113509ccf0a0a3ec4f6f1291b5f8ccd6c49"
};
const current = fs.readFileSync(path.join(root, 'Presentation-to-4G/src/tasks.rs'), 'utf8');
const normalize = text => text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '').replace(/\s/g, '');
for (const name of ['ACTION_TASKS', 'BURST_ACTION_TASKS', 'BLOCK_RULES']) {
  const table = text => {
    const start = text.indexOf('pub const ' + name);
    return normalize(text.slice(start, text.indexOf('];', start) + 2));
  };
  assert.equal(crypto.createHash('sha256').update(table(current)).digest('hex'), expectedHashes[name], name + ' 被结构调整改变');
}
run('verify-presentation-owner', `
#![allow(dead_code)]
use std::cell::RefCell;
${gameModules()}
thread_local! { static FIELDS:RefCell<(u32,u32,usize)>=const {RefCell::new((0,0,0))}; }
mod memory {
    pub unsafe fn read_u32(addr:u32)->u32 { super::FIELDS.with(|f|{let mut f=f.borrow_mut();assert_eq!(addr,f.0+0x1280);f.2+=1;f.1}) }
    pub unsafe fn read_u16(_:u32)->u16 { unreachable!() }
    pub unsafe fn read_f32(_:u32)->f32 { unreachable!() }
    pub unsafe fn read_u8(_:u32)->u8 { unreachable!() }
    pub use read_u8 as load_u8;
    pub use read_u32 as load_u32;
    pub use read_f32 as load_f32;
    pub unsafe fn write_u8(_:u32,_:u8) { unreachable!() }
    pub unsafe fn write_u32(_:u32,_:u32) { unreachable!() }
    pub(crate) unsafe fn method<T:Copy>(_:u32,_:u32)->T { unreachable!() }
}
mod native { pub unsafe fn function<T:Copy>(_:u32)->T { unreachable!() } }
fn main() {
    for actor in [0x21000000,0x47E33190,0x34000000] {
        FIELDS.with(|f|*f.borrow_mut()=(actor,actor+0x80,0));
        let player=player::ChargeBladePlayer::from_address(actor);
        assert_eq!(player.sound_sequence().address(),actor+0x111C);
        FIELDS.with(|f|assert_eq!(f.borrow().2,0));
        assert_eq!(unsafe{player.sound_sequence().owner().address()},actor+0x80);
        FIELDS.with(|f|assert_eq!(f.borrow().2,1));
    }
    println!("PASS: actual player.rs uses embedded sequence address +0x111C and reads owner once at +0x1280");
}
`);
console.log('PASS: original GU task frames/types/durations/sounds/MSS exactly preserve baseline');
