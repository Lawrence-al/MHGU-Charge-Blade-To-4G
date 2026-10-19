// 对照最终 ELF / NRO 与用户当前 subsdk9 的真实导出，检查装载所需入口及 ABI。
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const assert = require('assert/strict');
const root = path.resolve(__dirname, '..');
for (const [directory, stem] of [['Presentation-to-4G', 'presentation_to_4g'], ['Moveset-to-4G', 'moveset_to_4g']]) {
const elfPath = path.join(root, directory, 'target/armv7a-none-eabi/release/lib' + stem + '.so');
const nroPath = path.join(root, 'dist', 'cb_' + stem + '.nro');
const runtimePath = process.argv[2];
const elf = fs.readFileSync(elfPath), nro = fs.readFileSync(nroPath);
const u32 = (b, o) => b.readUInt32LE(o);
const hash = b => crypto.createHash('sha256').update(b).digest('hex');
const string = (b, o) => b.subarray(o, b.indexOf(0, o)).toString('utf8');

assert.equal(elf.subarray(0, 4).toString('hex'), '7f454c46');
assert.equal(elf[4], 1); // ELF32
assert.equal(elf[5], 1); // little endian
assert.equal(elf.readUInt16LE(18), 40); // EM_ARM
assert.equal(u32(elf, 36) & 0x600, 0x200); // soft-float C ABI
assert.equal(nro.subarray(0x10, 0x14).toString(), 'NRO0');
assert.equal(u32(nro, 0x18), nro.length);

const phOff = u32(elf, 28), phSize = elf.readUInt16LE(42), phCount = elf.readUInt16LE(44);
const loads = [];
for (let i = 0; i < phCount; i++) {
  const o = phOff + i * phSize;
  if (u32(elf, o) === 1) loads.push({ file: u32(elf, o + 4), addr: u32(elf, o + 8), size: u32(elf, o + 16), memory: u32(elf, o + 20), flags: u32(elf, o + 24) });
}
assert.equal(loads.length, 3);
const page = n => Math.ceil(n / 0x1000) * 0x1000;
for (let i = 0; i < 3; i++) {
  const load = loads[i], segment = 0x20 + i * 8;
  assert.equal(u32(nro, segment), load.addr);
  assert.equal(u32(nro, segment + 4), page(load.size));
  assert.equal(load.addr % 0x1000, 0);
  const skip = i === 0 ? 0x80 : 0; // NRO 头覆盖的预留区域。
  assert.deepEqual(nro.subarray(load.addr + skip, load.addr + load.size), elf.subarray(load.file + skip, load.file + load.size));
}
assert.equal(u32(nro, 0x38), page(Math.max(0, loads[2].memory - page(loads[2].size))));
const mod = u32(nro, 4);
assert.equal(nro.subarray(mod, mod + 4).toString(), 'MOD0');
const dynamic = mod + nro.readInt32LE(mod + 4);
assert.ok(dynamic >= loads[2].addr && dynamic < nro.length);
const bssStart = mod + nro.readInt32LE(mod + 8), bssEnd = mod + nro.readInt32LE(mod + 12);
assert.ok(bssEnd >= bssStart && bssEnd <= nro.length + u32(nro, 0x38));

const sectionOff = u32(elf, 32), sectionSize = elf.readUInt16LE(46), sectionCount = elf.readUInt16LE(48);
const sections = Array.from({ length: sectionCount }, (_, i) => {
  const o = sectionOff + i * sectionSize;
  return { type: u32(elf, o + 4), file: u32(elf, o + 16), size: u32(elf, o + 20), link: u32(elf, o + 24), stride: u32(elf, o + 36) };
});
// 沿用实机前置修复的装载约束：没有 DT_TEXTREL，重定位只写可写段。
for (let o = dynamic; u32(nro, o); o += 8) {
  assert.notEqual(u32(nro, o), 22, 'DT_TEXTREL is forbidden');
  if (u32(nro, o) === 30) assert.equal(u32(nro, o + 4) & 4, 0, 'DF_TEXTREL is forbidden');
}
let relocations = 0;
for (const section of sections.filter(s => s.type === 9)) {
  for (let i = 0; i < section.size; i += section.stride) {
    const target = u32(elf, section.file + i);
    assert.ok(loads.some(s => (s.flags & 2) && target >= s.addr && target + 4 <= s.addr + s.memory),
      'relocation targets a read-only segment: 0x' + target.toString(16));
    relocations++;
  }
}
const dynsym = sections.find(s => s.type === 11), dynstr = sections[dynsym.link];
const names = elf.subarray(dynstr.file, dynstr.file + dynstr.size), symbols = [];
for (let i = 0; i < dynsym.size; i += dynsym.stride) {
  const o = dynsym.file + i;
  symbols.push({ name: string(names, u32(elf, o)), value: u32(elf, o + 4), bind: elf[o + 12] >> 4, defined: elf.readUInt16LE(o + 14) !== 0 });
}
const exportedNames = symbols.filter(s => s.defined && s.bind !== 0).map(s => s.name);
const required = ['init', 'fini', 'forge_onLoad', 'forge_onInit', 'forge_onImGuiRender'];
for (const name of required) assert.ok(exportedNames.includes(name), 'missing callback: ' + name);

// 无游戏文件时仍检查完整 ELF/NRO；传入两个路径可额外核对真实导出和原指令。
if (!runtimePath) {
  const result = {nro: nroPath, bytes: nro.length, sha256: hash(nro), writable_relocation_targets: relocations, callbacks: required};
  fs.writeFileSync(path.join(root, directory, 'target/build-verification.json'), JSON.stringify(result, null, 2) + '\n');
  console.log('PASS: ' + directory + ': ARM32 soft-float, NRO/ELF segments, MOD0/BSS, writable relocations and callbacks');
  continue;
}
assert.ok(process.argv[3], '同时提供 Forge NSO 与 MHGU ExeFS 目录');
const context = require('./nso');
const runtime = fs.readFileSync(runtimePath), header = context.nsoHeader(runtime);
const memory = Buffer.alloc(header.dataMemOff + header.dataSize + header.bssSize);
for (const [i, name] of ['text', 'ro', 'data'].entries()) {
  const off = header[name + 'FileOff'], size = header[name + 'FileSize'];
  const segment = header.flags & (1 << i) ? context.decompressSegment(runtime, off, size, header[name + 'Size']) : runtime.subarray(off, off + size);
  segment.copy(memory, header[name + 'MemOff']);
}
const runtimeMod = u32(memory, 4);
assert.equal(memory.subarray(runtimeMod, runtimeMod + 4).toString(), 'MOD0');
const tags = {};
for (let o = runtimeMod + memory.readInt32LE(runtimeMod + 4); u32(memory, o) !== 0; o += 8) tags[u32(memory, o)] = u32(memory, o + 4);
const runtimeNames = memory.subarray(tags[5], tags[5] + tags[10]);
const runtimeSymbols = [];
for (let i = 0; i < u32(memory, tags[4] + 4); i++) {
  const o = tags[6] + 16 * i;
  if (memory.readUInt16LE(o + 14) !== 0) runtimeSymbols.push({ name: string(runtimeNames, u32(memory, o)), value: u32(memory, o + 4) });
}
const imports = symbols.filter(s => !s.defined && s.bind === 1).map(s => s.name);
// Forge API 由当前 subsdk9 导出；SDK / malloc 等由原游戏 main 和 sdk 导出。
// 伤害显示插件没有这些分配器依赖，CB 的 Vec / Mutex 必须按完整进程核对。
const providers = new Map(runtimeSymbols.map(s => [s.name, 'installed Forge']));
for (const moduleName of ['main', 'sdk', 'rtld', 'subsdk0']) {
  const file = path.join(process.argv[3], moduleName);
  const nso = fs.readFileSync(file), h = context.nsoHeader(nso);
  const mapped = Buffer.alloc(h.dataMemOff + h.dataSize + h.bssSize);
  for (const [i, part] of ['text', 'ro', 'data'].entries()) {
    const off = h[part + 'FileOff'], length = h[part + 'FileSize'];
    const data = h.flags & (1 << i) ? context.decompressSegment(nso, off, length, h[part + 'Size']) : nso.subarray(off, off + length);
    data.copy(mapped, h[part + 'MemOff']);
  }
  const mod = u32(mapped, 4), tags = {};
  assert.equal(mapped.toString('ascii', mod, mod + 4), 'MOD0');
  for (let o = mod + mapped.readInt32LE(mod + 4); u32(mapped, o); o += 8) tags[u32(mapped, o)] = u32(mapped, o + 4);
  const count = tags[4] ? u32(mapped, tags[4] + 4) : (tags[5] - tags[6]) / 16;
  for (let i = 0; i < count; i++) {
    const o = tags[6] + i * 16;
    if (!mapped.readUInt16LE(o + 14)) continue;
    const name = string(mapped, tags[5] + u32(mapped, o));
    if (!providers.has(name)) providers.set(name, 'GU ' + moduleName);
  }
  if (moduleName === 'main') {
    for (const file of ['moveset.rs', 'saed/patches.rs', 'overpowered.rs']) {
      const text = fs.readFileSync(path.join(root, 'Moveset-to-4G/src', file), 'utf8');
      for (const match of text.matchAll(/\((0x[0-9A-F]+), (0x[0-9A-F]+), (0x[0-9A-F]+)\)/g)) {
        assert.equal(u32(mapped, Number(match[1])), Number(match[2]), 'GU original word: ' + match[1]);
      }
    }
  }
}
for (const name of imports) assert.ok(providers.has(name), 'unresolved strong import in installed Forge + GU modules: ' + name);
const create = runtimeSymbols.find(s => s.name === 'forge_hook_create');
// 新前置用三次 STRD 写回 24 字节；旧版先传入 memcpy 长度 76。
// 项目显式提供 76 字节的清零返回区，两个已知实现都能完整容纳。
const hookBytes = u32(memory, create.value + 0x24) === 0xE3A0204C ? 76
  : [0xE1C200F0, 0xE1C200F8, 0xE1C201F0].every((word, i) =>
      u32(memory, create.value + 0xA4 + i * 8) === word) ? 24 : 0;
assert.ok(hookBytes, 'unrecognized Forge Hook return implementation; inspect ABI before deploying');

const result = {
  nro: nroPath, bytes: nro.length, sha256: hash(nro),
  runtime: runtimePath, runtime_sha256: hash(runtime),
  hook_return_bytes: hookBytes, hook_storage_bytes: 76,
  writable_relocation_targets: relocations,
  callbacks: required, strong_imports: imports.map(name => ({name, provider: providers.get(name)})),
  checks: ['ARM32 soft-float', 'NRO segments match ELF', 'MOD0/dynamic/BSS', 'no TEXTREL; every relocation target writable', 'all callbacks exported', 'all strong imports in installed Forge + GU modules', 'known Hook ABI fits project storage'],
};
fs.writeFileSync(path.join(root, directory, 'target/build-verification.json'), JSON.stringify(result, null, 2) + '\n');
console.log('PASS: ' + directory + ': ' + nro.length + ' bytes; ARM32/NRO/MOD0/BSS/callbacks/strong imports and installed Forge Hook ABI');
}
