// 执行实际 Presentation NRO 的盾突入口，验证起手红盾触发与原函数返回。
// 原游戏动作及粒子创建使用替身；不依赖 CreateShell02 的命中或返回结果。
const path = require('path');
const assert = require('assert/strict');
const { Arm, loadPlugin, bitsFloat, floatBits } = require('./arm');
const root = path.resolve(__dirname, '..');
const plugin = loadPlugin(path.join(root, 'Presentation-to-4G'), 'presentation_to_4g');
const hook = plugin.symbol('hook_cb_shield_thrust_action8___detour');
const slot = plugin.symbol('hook_cb_shield_thrust_action10___ORIGINAL');
const effect = plugin.symbol('create_timed_on_shield');
let getConfig;
try { getConfig = plugin.symbol('config_file3get'); } catch (error) {
  // LTO可能完全内联get；这种构建必须从实际PIC指令找到配置值地址。
  if (!error.message.includes('config_file3get')) throw error;
}
const configStorage = plugin.symbol('config_file6CONFIG');
// Rust OnceLock 没有 repr(C)，新增配置字段后其 value 可被编译器重排。
// 从实际PIC取址和VLDR位移推导字段地址；不假定静态基址等于Config起点。
const durationLoad = [];
for (let p=hook.value; p<hook.value+hook.size; p+=4) {
  if ((plugin.nro.readUInt32LE(p)&0xFFF0FF00)>>>0===0xED900A00) durationLoad.push(p); // vldr s0,[rN,#imm]
}
assert.equal(durationLoad.length,1,'only the original GU path loads a duration; rebuild the current NRO if this is an old GU/4G-switch binary');
const durationInstruction=plugin.nro.readUInt32LE(durationLoad[0]);
const configRegister=(durationInstruction>>>16)&15;
const durationOffset=(durationInstruction&255)*4;
let configValue;
for(let p=hook.value+4;p<durationLoad[0];p+=4) {
  if(plugin.nro.readUInt32LE(p)===(0xE08F0000|(configRegister<<12)|configRegister)>>>0){
    // 指令调度可在LDR与ADD之间给其它参数寄存器赋值。
    for(let load=p-4;load>=hook.value;load-=4){
      const word=plugin.nro.readUInt32LE(load);
      if((word&0xFFFFF000)>>>0===(0xE59F0000|(configRegister<<12))>>>0){
        const staticBase=p+8+plugin.nro.readInt32LE(load+8+(word&0xFFF));
        configValue=staticBase+durationOffset-12;break;
      }
    }
    if(configValue!==undefined)break;
  }
}
if(configValue!==undefined)assert.ok(configValue>=configStorage.value && configValue+16<=configStorage.value+configStorage.size);
else assert(getConfig,'no config getter or PIC-derived config value for the actual duration load');
let count = 0;
for (const base of [0x70000000, 0x4C000000]) {
  for (let variant = 0; variant < 6; variant++) for (let state = 0; state < 4; state++)
    for (let shield = 0; shield < 4; shield++) for (const result of [0, 1, 0x12345678]) for (const duration of [17.5, 30]) {
      const r = Array.from({ length: 16 }, (_, i) => (0x11000000 + i * 0x10101) >>> 0);
      r[0] = 0x20000000; r[1] = variant; r[13] = 0x22001000; r[14] = 0xFFFFF000;
      const original = 0x23000000;
      const cpu = new Arm(r, [{ base, data: plugin.nro }], [
        [base + slot.value, original], [r[0] + 0x3344, shield],
        [0x25000000 + 12, floatBits(duration)], // repr(C) Config::shield_thrust_duration
        ...(configValue===undefined?[]:[[base + configValue + 12, floatBits(duration)]]), // 实际PIC配置起点
      ], [[r[0] + 5, state]]);
      const events = [];
      if(getConfig)cpu.externalCalls.set(base + getConfig.value, c => { c.r[0] = 0x25000000; });
      cpu.externalCalls.set(original, c => {
        assert.deepEqual(Array.from(c.r.slice(0, 2)), [r[0], variant]);
        assert.equal(c.r[13] % 8, 0);
        events.push('original');
        // 起手后原函数推进 state；入口必须保留调用前的 state 0。
        if (state === 0) c.bytes.set(r[0] + 5, variant < 3 ? 1 : 2);
        c.r[0] = result; c.r[1] = 0xBB; c.r[2] = 0xCC; c.r[3] = 0xDD; c.r[12] = 0xEE;
      });
      cpu.externalCalls.set(base + effect.value, c => {
        assert.deepEqual(Array.from(c.r.slice(0, 3)), [r[0], 0x258, 0x259]);
        assert.equal(bitsFloat(c.s[0]), duration);
        assert.equal(c.r[13] % 8, 0);
        events.push('effect');
        c.r[0] = 0x31000000; c.r[1] = 0xBA; c.r[2] = 0xCA; c.r[3] = 0xDA; c.r[12] = 0xEA;
      });
      cpu.run(base + hook.value, r[14]);
      assert.deepEqual(events,state===0&&shield===2?['original','effect']:['original']);
      assert.equal(cpu.r[0], result);
      for (let i = 4; i <= 11; i++) assert.equal(cpu.r[i], r[i]);
      assert.equal(cpu.r[13], r[13]);
      count++;
    }
}
assert.equal(count,1152);
console.log('PASS: '+count+' compiled GU shield-thrust cases; red-shield state 0 only, PIC-derived configured duration 17.5/30, no SE or converted sequence, result/registers/stack preserved');
require('./verify-shield-thrust-once')();
