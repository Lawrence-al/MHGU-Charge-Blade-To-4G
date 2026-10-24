// 主机运行真实对象方法，仅读写地址空间和原生函数入口由 harness 提供替身。
const {modulePath} = require('./host');
module.exports = function gameModules() {
  return `
extern crate self as cb_game;
#[derive(Clone, Copy)] pub struct Game(u32);
impl Game {
  pub const fn new(base:u32)->Self {Self(base)}
  pub const fn address(self,offset:u32)->u32 {self.0.wrapping_add(offset)}
  pub(crate) unsafe fn function<T:Copy>(self,offset:u32)->T {unsafe{native::function(self.address(offset))}}
}
${['player', 'command', 'sound', 'effect', 'shell', 'resource', 'abi', 'surface'].map(name =>
  modulePath('game-api/src/' + name + '.rs', name)).join('\n')}
`;
};
