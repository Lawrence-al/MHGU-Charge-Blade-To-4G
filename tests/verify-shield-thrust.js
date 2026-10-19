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
const getConfig = plugin.symbol('config_file3get');
const configStorage = plugin.symbol('config_file6CONFIG');
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
        [base + configStorage.value + 24, floatBits(duration)], // 当前 LTO 直接引用 OnceLock 内的配置
      ], [[r[0] + 5, state]]);
      const events = [];
      cpu.externalCalls.set(base + getConfig.value, c => { c.r[0] = 0x25000000; });
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
      assert.deepEqual(events, state === 0 && shield === 2 ? ['original', 'effect'] : ['original']);
      assert.equal(cpu.r[0], result);
      for (let i = 4; i <= 11; i++) assert.equal(cpu.r[i], r[i]);
      assert.equal(cpu.r[13], r[13]);
      count++;
    }
}
console.log('PASS: ' + count + ' compiled shield-thrust entry cases; red-shield state 0 only, configured duration 17.5/30, no sound-object access or SE call, result/registers/stack preserved');
require('./verify-shield-thrust-once')();
