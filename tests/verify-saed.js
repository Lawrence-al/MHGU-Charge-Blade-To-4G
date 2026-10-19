// 用原版 4G ELF 执行表初始化，再验证 NRO 的时序／HDP 扩容纯逻辑。
const fs = require('fs');
const path = require('path');
const cp = require('child_process');
const assert = require('assert/strict');
const { Arm, bitsFloat, floatBits } = require('./arm');

const root = path.resolve(__dirname, '..');
const [guDump, fourElf] = process.argv.slice(2);
if (!guDump || !fourElf) throw Error('用法：node tests/verify-saed.js "MHGU dump 目录" "4G ExeFS.elf"');
const elf = fs.readFileSync(fourElf);
function read(address, size) {
  for (let i = 0; i < elf.readUInt16LE(44); i++) {
    const p = elf.readUInt32LE(28) + i * elf.readUInt16LE(42);
    if (elf.readUInt32LE(p) !== 1) continue;
    const file = elf.readUInt32LE(p + 4), base = elf.readUInt32LE(p + 8), length = elf.readUInt32LE(p + 16);
    if (address >= base && address + size <= base + length) return elf.subarray(file + address - base, file + address - base + size);
  }
  throw Error('ELF 未映射地址 0x' + address.toString(16));
}
const registers = Array(16).fill(0); registers[13] = 0x20000000;
const cpu = new Arm(registers, [
  { base: 0xD1A020, data: read(0xD1A020, 0x268) },
  { base: 0x100E800, data: read(0x100E800, 240) },
]);
cpu.run(0xD1A020, 0xD1A204, 200);
const table = address => Array.from({ length: 6 }, (_, i) => [0, 4, 8].map(field => bitsFloat(cpu.read(address + i * 20 + field))));
assert.deepEqual(table(0x100E800), [
  [0, 0, 70], [9, 0, 170], [18, 0, 270], [27, 0, 370], [36, 0, 470], [45, 0, 570],
]);
assert.deepEqual(table(0x100E878), [
  [0, 0, 100], [18, 80, 220], [20, -80, 220], [38, -120, 340], [40, 120, 340], [42, 0, 340],
]);
console.log('PASS: original 4G ARM startup produces the measured impact and element tables, including runtime-initialized positions');

// 表第一项的时间为 0，但 AED_hit 在进入表之前另有首爆倒计时。
for (const mode of [0, 1, 2]) for (const type of [4, 5]) {
  const regs = Array(16).fill(0); regs[0] = 0x21000000; regs[13] = 0x22001000;
  const init = new Arm(regs, [
    { base: 0xC60644, data: read(0xC60644, 0x11C) },
    { base: 0xC6096C, data: read(0xC6096C, 0x1C) },
    { base: 0x100E7D0, data: read(0x100E7D0, 48) },
  ], [[regs[0] + 260, floatBits(35)]], [[regs[0] + 660, mode], [regs[0] + 253, type], [regs[0] + 661, 5]]);
  init.run(0xC60644, 0xC606A0);
  const expected = mode === 1 ? 2 : mode === 0 ? 40 : type === 4 ? 46 : 0;
  assert.equal(bitsFloat(init.read(regs[0] + 316)), expected);
  assert.equal(init.read(regs[0] + 622, true), 5);
}
console.log('PASS: original 4G AED_hit ARM initializes normal ground SAED first delays: impact=0, element=46; six initialization branches checked');

const dir = path.join(root, 'Moveset-to-4G/target/runtime-check');
fs.mkdirSync(dir, { recursive: true });
// 直接执行 4G 的旋转／平移指令，作为 Rust world_xz 的外部依据。
const nativePositions = [];
const element = table(0x100E878);
for (let yaw = 0; yaw < 65536; yaw += 256) {
  const trig = read(0xE18354 + (yaw >>> 8) * 16, 16);
  const sin = trig.readFloatLE(0), cos = trig.readFloatLE(4);
  const origin = [120, 35, -680];
  const processOrigin = [Math.fround(origin[0] + Math.fround(sin * 200)), origin[1], Math.fround(origin[2] + Math.fround(cos * 200))];
  for (let i = 0; i < element.length; i++) {
    const regs = Array(16).fill(0); regs[4] = 0x21000000; regs[13] = 0x22001000;
    const words = processOrigin.map((value, axis) => [regs[4] + 0x100 + axis * 4, floatBits(value)]);
    words.push([regs[13] + 20, 0]);
    const machine = new Arm(regs, [{ base: 0xC60350, data: read(0xC60350, 0x40) }], words);
    machine.s[0] = floatBits(cos); machine.s[17] = floatBits(element[i][2]);
    machine.s[18] = floatBits(element[i][1]); machine.s[19] = floatBits(sin);
    machine.run(0xC60350, 0xC60390);
    const record = Buffer.alloc(32);
    record.writeUInt32LE(i, 0);
    [sin, cos, ...origin].forEach((value, field) => record.writeFloatLE(value, 4 + field * 4));
    record.writeUInt32LE(machine.read(regs[13] + 16), 24);
    record.writeUInt32LE(machine.read(regs[13] + 24), 28);
    nativePositions.push(record);
  }
}
const positionsFile = path.join(dir, 'four-g-fan-positions.bin');
fs.writeFileSync(positionsFile, Buffer.concat(nativePositions));
const executable = path.join(dir, 'verify-saed.exe');
cp.execFileSync((process.env.RUSTC || 'rustc'), [
  '--edition=2024', '--extern', 'cb_game=' + require('./host').gameRlib,
  path.join(__dirname, 'verify-saed.rs'), '-o', executable,
], { stdio: 'inherit' });
cp.execFileSync(executable, [
  path.join(guDump, 'romfs/nativeNX/arc/player/quest/w14-arc/player/hit/pl_we14_hitdata.4E2EF008'),
  path.join(root, 'assets/romfs/nativeNX/arc/player/quest/w14.arc/hit/pl_we14_hitdata.4E2EF008'),
  path.join(__dirname, 'data/saed-hdp50.bin'),
  positionsFile,
], { stdio: 'inherit' });
const originExecutable = path.join(dir, 'verify-phial-origin.exe');
cp.execFileSync((process.env.RUSTC || 'rustc'), [
  '--edition=2024', path.join(__dirname, 'verify-phial-origin.rs'), '-o', originExecutable,
], { stdio: 'inherit' });
cp.execFileSync(originExecutable, [], { stdio: 'inherit' });
