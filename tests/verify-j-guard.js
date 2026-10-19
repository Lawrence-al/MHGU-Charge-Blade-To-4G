// 执行实际 NRO 的两个独立动作派生；命令期望值从用户 asset 的 XFS 字段独立读取。
const fs = require('fs');
const path = require('path');
const assert = require('assert/strict');
const { Arm, loadPlugin, code, floatBits } = require('./arm');
const root = path.resolve(__dirname, '..');
const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
const bridge = plugin.symbol('j_guard_bridge'), slot = plugin.symbol('J_GUARD_CONTINUATION');
const options = plugin.symbol('AED_ROUTE_FLAGS');
// 独立快照：原 AED 命令的前 24 字节，不依赖未发布的完整 XFS。
const expected = code([4456464,524288,27,0,0,0]);
assert.equal(expected.readUInt16LE(2), 0x44);
assert.equal(expected.readUInt16LE(4), 0);
assert.equal(expected.readUInt32LE(8), 0x1B);

let cases = 0;
for (const [game, base] of [[0x60000000, 0x70000000], [0x34000000, 0x4C000000]]) {
  const site = game + 0x002EA87C, end = site + 8, player = 0x21000000, equip = 0x22000000, command = 0x23000000;
  for (const chain of [0x819, 0x81A, 0x81B, 0x7F9, 0x7FA, 0x7FB]) for (const index of [0, 1, 2, 3, 4, 7, 8, 12]) {
    for (const style of [0, 1, 2, 3, 4, 5]) for (const weapon of [0, 9, 14]) for (let flags = 0; flags < 16; flags++) for (const enabled of [0, 1, 2, 3]) {
      const original = chain === 0x7FA
        ? code([0x00240010, 0x00088000, 0x1B, 0, 0, 0, floatBits(34), floatBits(10)])
        : code([0x00480010, 0x00088000, 0x1A, 0, 0, 0, floatBits(30), floatBits(20)]);
      const regs = Array.from({ length: 16 }, (_, i) => (0x11000000 + i * 0x10101) >>> 0);
      regs[4] = player; regs[5] = command; regs[8] = index; regs[13] = 0x24001000;
      const machine = new Arm(regs, [{ base, data: plugin.nro }, { base: command, data: original }], [
        [site, 0xE51FF004], [site + 4, base + bridge.value], [base + slot.value, end],
        [base + options.value, enabled],
        [player + 0x24AC, chain], [player + 0x13C4, equip],
      ], [[equip + 0x4D4, weapon], [equip + 0x4D9, style]]);
      machine.flags = { n: !!(flags & 8), z: !!(flags & 4), c: !!(flags & 2), v: !!(flags & 1) };
      const originalFlags = { ...machine.flags }, originalVfp = [...machine.s];
      machine.run(site, end, 100);
      const actual = Buffer.alloc(32);
      for (let field = 0; field < 32; field += 4) actual.writeUInt32LE(machine.read(command + field), field);
      const wanted = Buffer.from(original);
      const route = (chain === 0x81A && index === 3 && (enabled & 1))
        || (chain === 0x7FA && index === 1 && (enabled & 2));
      if (route && style === 3 && weapon === 14) expected.copy(wanted);
      assert.deepEqual(actual, wanted, '派生字段/窗口');
      assert.equal(machine.r[0], 0); assert.equal(machine.r[1], 999);
      for (let reg = 2; reg < 15; reg++) assert.equal(machine.r[reg], regs[reg], 'r' + reg);
      assert.deepEqual(machine.flags, originalFlags);
      assert.deepEqual([...machine.s], originalVfp);
      assert.equal(machine.svcs.length, 0);
      cases++;
    }
  }
}
console.log('PASS: ' + cases + ' compiled AED route cases; independent J guard / Bushi shield-thrust switches; only chains 0x81A entry 3 / 0x7FA entry 1; action 0x44/input 0x1B, windows/NZCV/registers/VFP/ASLR retained');
