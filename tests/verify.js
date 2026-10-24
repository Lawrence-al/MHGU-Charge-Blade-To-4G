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
  require('./verify-style-guard');
  verifyCache(loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g'));
}
function verifySaed() {
  const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
  require('./verify-saed-abi').verifySaedAbi();
  verifyCache(plugin);
}
function verifyTables() {
  const all = [];
  for (const [file, expected, branches] of [['moveset.rs', fixture.moveset.slice(7), [[0x011DDE74, 8], [0x002EA87C, 8]]], ['guard.rs', [], [[0x011DFB0C, 8], [0x002F0800, 8]]], ['saed/patches.rs', fixture.saed, []], ['overpowered.rs', fixture.overpowered, []]]) {
    const source = fs.readFileSync(path.join(root, 'Moveset-to-4G/src', file), 'utf8');
    const rows = [...source.matchAll(/\((0x[0-9A-F]+), (0x[0-9A-F]+), (0x[0-9A-F]+)\)/g)].map(m => m.slice(1).map(Number));
    assert.deepEqual(rows.map(([offset, , value]) => [offset, value]), expected);
    all.push(...rows.map(([offset]) => [offset, 4]), ...branches);
  }
  for (const project of ['Presentation-to-4G', 'Moveset-to-4G']) {
    const activeSource = fs.readFileSync(path.join(root, project, 'src/hooks.rs'), 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    const hooks = [...activeSource.matchAll(/cb_hook!\s*\{\s*(0x[0-9A-F]+)/g)].map(m => [Number(m[1]), 8]);
    if(project==='Presentation-to-4G'){
      assert.deepEqual(hooks.map(([offset])=>offset),[0x011D3F04,0x00B67AE4,0x004D923C,0x004D74F8,0x011DABC8,0x011DE7E4,0x00282548,0x0048874C,0x004A1E50,0x011DA204,0x011DB280,0x00043224,0x0004201C]);
    }else{assert.equal(hooks.length,5);}
    all.push(...hooks);
  }
  // 新增的三个窄视觉桥也必须与Moveset、菜单及函数入口钩子完全不重叠。
  const phial = fs.readFileSync(path.join(root, 'Presentation-to-4G/src/phial_bridges.rs'), 'utf8');
  const phialOffsets = [...phial.matchAll(/offset:\s*(0x[0-9A-F]+)/g)].map(m=>Number(m[1]));
  assert.deepEqual(phialOffsets,[0x00488358,0x004886EC,0x00488654]);
  all.push(...phialOffsets.map(offset=>[offset,8]));
  const styleMenu = require('./style-menu-fixtures');
  all.push(...styleMenu.branches.map(([offset]) => [offset, 8]), [0x011D5C64, 8]);
  for (const file of ['actions.rs', 'menu.rs']) {
    const source = fs.readFileSync(path.join(root, 'Moveset-to-4G/src/style', file), 'utf8');
    const hooks = [...source.matchAll(/cb_hook!\s*\{\s*(0x[0-9A-F]+)/g)].map(m => [Number(m[1]), 8]);
    assert.equal(hooks.length, file === 'actions.rs' ? 1 : 6);
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
  for (const removed of [0x00043FF4,0x009BA5B0,0x00A5B558,0x011DA634,0x011DA5EC,0x011DA610,0x011DA5C0,0x011DA43C,0x011DA4B8,0x011DA4C0,0x011DB014, 0x00488F5C, 0x00488F74, 0x011DE158, 0x011DB498, 0x011DACF0, 0x011DAF74, 0x011DB0B4, 0x011DB0FC, 0x002EA864, 0x011DEA04]) assert.ok(all.every(([address]) => address !== removed));
  console.log('PASS: 24 direct values, HDP 0x50/0x23 calls, two cb_ NRO formats; no overlapping hooks/branches, codecave writes or retired yellow-shield patches');
  loadPlugin(path.join(root, 'Presentation-to-4G'), 'presentation_to_4g');
  loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
  assert.deepEqual(fs.readdirSync(path.join(root, 'dist')).filter(name => name.endsWith('.nro')).sort(), ['cb_moveset_to_4g.nro', 'cb_presentation_to_4g.nro']);
}
module.exports = { verifyMoveset, verifySaed, verifyTables };
if (require.main === module) { verifyTables(); verifyMoveset(); verifySaed(); }
