const fs = require('fs');
const path = require('path');
const assert = require('assert/strict');
const { Arm, loadPlugin, code, floatBits } = require('./arm');
const fixture = require('./fixtures');
const root = path.resolve(__dirname, '..');
const loadBases = [[0x60000000, 0x70000000], [0x34000000, 0x4C000000]];
function registers() {
  const r = Array.from({ length: 16 }, (_, i) => (0x11000000 + i * 0x10101) >>> 0);
  r[13] = 0x22001000;
  return r;
}
function preserved(cpu, regs, changed) {
  for (let i = 0; i <= 14; i++) if (!changed.includes(i)) assert.equal(cpu.r[i], regs[i], '寄存器 r' + i);
}
function verifyCache(plugin) {
  for (const [game, base] of loadBases) for (const size of [4, 8, 0x1000]) for (const result of [0, 0x1234]) {
    const r = registers(); r[0] = game + 0x00488F5C; r[1] = size; r[14] = 0xFFFFF000;
    const cpu = new Arm(r, [{ base, data: plugin.nro }]); cpu.svcResult = result;
    cpu.run(base + plugin.symbol('flush_code').value, r[14]);
    assert.deepEqual(cpu.svcs, [{ number: 0x5F, regs: [0xFFFF8001, size, r[0], 0, 0] }]);
    assert.equal(cpu.r[0], result);
    preserved(cpu, r, [0, 2, 3]);
  }
}
function verifyMoveset() {
  const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
  const bridge = plugin.symbol('gp_bridge'), slot = plugin.symbol('MAIN_CONTINUATION');
  const cave = code(fixture.gp);
  for (let i = 0; i < 15; i++) assert.equal(plugin.nro.readUInt32LE(bridge.value + i * 4), fixture.gp[i]);
  assert.equal(plugin.nro.readUInt32LE(bridge.value + 18 * 4), 0xE5810000);
  let cases = 0;
  for (const [gameBase, nroBase] of loadBases) {
    const site = gameBase + 0x011DDE74, end = site + 8, caveBase = gameBase + 0x013F1F58;
    const actor = 0x20000000, equip = 0x21000000, flagsAddress = actor + 0x3340;
    const ranges = [{ base: nroBase, data: plugin.nro }, { base: caveBase, data: cave }];
    const words = [[site, 0xE51FF004], [site + 4, nroBase + bridge.value], [nroBase + slot.value, end], [actor + 0x13C4, equip]];
    for (let mode = 0; mode < 256; mode++) for (let action = 0; action < 256; action++) for (const flag of [0, 0x20, 0x80000045]) {
      const r = registers(); r[0] = flag; r[1] = flagsAddress; r[4] = actor;
      const bytes = [[equip + 0x4D9, mode], [actor + 0x250A, action]];
      const baseline = [...r]; baseline[14] = site + 4;
      const original = new Arm(baseline, ranges, [[actor + 0x13C4, equip], [site + 4, 0xE5810000]], bytes);
      original.run(caveBase, end);
      const actual = new Arm(r, ranges, words, bytes);
      actual.run(site, end);
      assert.equal(actual.r[0], original.r[0]);
      assert.deepEqual(actual.flags, original.flags);
      assert.equal(actual.words.get(flagsAddress), original.words.get(flagsAddress));
      preserved(actual, r, [0]);
      cases++;
    }
  }
  verifyCache(plugin);
  console.log('PASS: ' + cases + ' compiled GP cases match original cave; flags/registers/stack/writeback/ASLR/cache ABI');
}
function verifySaed() {
  const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
  require('./verify-saed-abi').verifySaedAbi();
  verifyCache(plugin);
}
function verifyTables() {
  const all = [];
  for (const [file, expected, branches] of [['moveset.rs', fixture.moveset, [[0x011DDE74, 8], [0x002EA87C, 8]]], ['saed/patches.rs', fixture.saed, []], ['overpowered.rs', fixture.overpowered, []]]) {
    const source = fs.readFileSync(path.join(root, 'Moveset-to-4G/src', file), 'utf8');
    const rows = [...source.matchAll(/\((0x[0-9A-F]+), (0x[0-9A-F]+), (0x[0-9A-F]+)\)/g)].map(m => m.slice(1).map(Number));
    assert.deepEqual(rows.map(([offset, , value]) => [offset, value]), expected);
    all.push(...rows.map(([offset]) => [offset, 4]), ...branches);
  }
  for (const project of ['Presentation-to-4G', 'Moveset-to-4G']) {
    const activeSource = fs.readFileSync(path.join(root, project, 'src/hooks.rs'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    const hooks = [...activeSource.matchAll(/cb_hook!\s*\{\s*(0x[0-9A-F]+)/g)].map(m => [Number(m[1]), 8]);
    assert.equal(hooks.length, project === 'Presentation-to-4G' ? 8 : 5);
    all.push(...hooks);
  }
  for (let i = 0; i < all.length; i++) {
    const [a, length] = all[i];
    assert.ok(a + length <= 0x013F1EEC || a >= 0x013F1FA0, '写入旧代码洞');
    for (let j = i + 1; j < all.length; j++) assert.ok(a + length <= all[j][0] || a >= all[j][0] + all[j][1], '插件写入范围重叠');
  }
  const target = (site, word) => (site + 8 + ((word << 8) >> 6)) >>> 0;
  assert.equal(target(0x011DADE4, 0xEBC37115), 0x002B7240);
  assert.equal(target(0x011DBDB0, 0xEBC36D22), 0x002B7240);
  for (const removed of [0x011DB014, 0x00488F5C, 0x00488F74, 0x011DE158, 0x011DB498, 0x011DACF0, 0x011DAF74, 0x011DB0B4, 0x011DB0FC, 0x002EA864]) assert.ok(all.every(([address]) => address !== removed));
  console.log('PASS: all 31 direct values, HDP 0x50/0x23 calls, two cb_ NRO formats; no overlaps/codecave writes or obsolete Overpowered chain/query writes');
  loadPlugin(path.join(root, 'Presentation-to-4G'), 'presentation_to_4g');
  loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
  assert.deepEqual(fs.readdirSync(path.join(root, 'dist')).filter(name => name.endsWith('.nro')).sort(), ['cb_moveset_to_4g.nro', 'cb_presentation_to_4g.nro']);
}
module.exports = { verifyMoveset, verifySaed, verifyTables };
if (require.main === module) { verifyTables(); verifyMoveset(); verifySaed(); }
