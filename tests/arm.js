// 解释执行本项目桥接函数使用的 ARM/VFP 指令，读取实际 NRO 的代码与 ELF 符号。
// 不模拟游戏、内核缓存效果或 FPSCR 浮点异常位。
const fs = require('fs');
const path = require('path');
const assert = require('assert/strict');

function loadPlugin(root, name) {
  const nro = fs.readFileSync(path.join(root, '../dist/cb_' + name + '.nro'));
  const elf = fs.readFileSync(path.join(root, 'target/armv7a-none-eabi/release/lib' + name + '.so'));
  assert.equal(nro.toString('ascii', 0x10, 0x14), 'NRO0');
  assert.equal(elf.readUInt16LE(18), 40);
  const offset = elf.readUInt32LE(32), size = elf.readUInt16LE(46), count = elf.readUInt16LE(48);
  const sections = Array.from({ length: count }, (_, i) => {
    const p = offset + i * size;
    return { type: elf.readUInt32LE(p + 4), file: elf.readUInt32LE(p + 16), size: elf.readUInt32LE(p + 20), link: elf.readUInt32LE(p + 24), entry: elf.readUInt32LE(p + 36) };
  });
  const tab = sections.find(s => s.type === 2), strings = sections[tab.link], symbols = [];
  for (let p = tab.file; p < tab.file + tab.size; p += tab.entry) {
    const start = strings.file + elf.readUInt32LE(p);
    symbols.push({ name: elf.toString('utf8', start, elf.indexOf(0, start)), value: elf.readUInt32LE(p + 4), size: elf.readUInt32LE(p + 8) });
  }
  return { nro, symbol(part) {
    const matches = symbols.filter(s => s.name.includes(part));
    assert.equal(matches.length, 1, part);
    return matches[0];
  } };
}
function code(words) {
  const b = Buffer.alloc(words.length * 4);
  words.forEach((w, i) => b.writeUInt32LE(w, i * 4));
  return b;
}
const scratch = Buffer.alloc(4);
function floatBits(n) { scratch.writeFloatLE(n); return scratch.readUInt32LE(); }
function bitsFloat(n) { scratch.writeUInt32LE(n >>> 0); return scratch.readFloatLE(); }
function rotate(value, shift) { return shift ? ((value >>> shift) | (value << (32 - shift))) >>> 0 : value >>> 0; }

class Arm {
  constructor(regs, ranges, words = [], bytes = []) {
    this.r = Uint32Array.from(regs);
    this.s = Uint32Array.from({ length: 32 }, (_, i) => floatBits(i + 0.125));
    this.ranges = ranges;
    this.words = new Map(words);
    this.bytes = new Map(bytes);
    this.flags = { n: false, z: false, c: false, v: true };
    this.svcs = [];
    this.svcResult = 0;
    this.externalCalls = new Map();
  }
  read(addr, byte = false) {
    addr >>>= 0;
    if (byte && this.bytes.has(addr)) return this.bytes.get(addr);
    if (this.words.has(addr)) return byte ? this.words.get(addr) & 255 : this.words.get(addr);
    const range = this.ranges.find(x => addr >= x.base && addr + (byte ? 1 : 4) <= x.base + x.data.length);
    if (!range) throw Error('未映射地址 0x' + addr.toString(16));
    return byte ? range.data.readUInt8(addr - range.base) : range.data.readUInt32LE(addr - range.base);
  }
  run(start, end, maxSteps = 100) {
    this.r[15] = start;
    for (let steps = 0; this.r[15] !== end; steps++) {
      assert.ok(steps < maxSteps, '未返回预定地址');
      const pc = this.r[15], ins = this.read(pc), cond = ins >>> 28;
      this.r[15] = (pc + 4) >>> 0;
      if (ins === 0xF57FF04F || ins === 0xF57FF06F) continue;
      if (cond === 0 && !this.flags.z) continue;
      if (cond === 1 && this.flags.z) continue;
      assert.ok(cond === 0 || cond === 1 || cond === 14, '不支持的条件');
      const reg = i => i === 15 ? (pc + 8) >>> 0 : this.r[i];
      const sd = ((ins >>> 12) & 15) * 2 + ((ins >>> 22) & 1);
      const sn = ((ins >>> 16) & 15) * 2 + ((ins >>> 7) & 1);
      const sm = (ins & 15) * 2 + ((ins >>> 5) & 1);
      if ((ins & 0x0FFF0FFF) === 0x010F0000) {
        // MRS APSR/CPSR：本解释器跟踪 NZCV，其它状态位不参与这些桥接。
        this.r[(ins >>> 12) & 15] = ((+this.flags.n << 31) | (+this.flags.z << 30) | (+this.flags.c << 29) | (+this.flags.v << 28)) >>> 0;
      } else if ((ins & 0x0FFFFFF0) === 0x0128F000) {
        const value = reg(ins & 15);
        this.flags = { n: !!(value & 0x80000000), z: !!(value & 0x40000000), c: !!(value & 0x20000000), v: !!(value & 0x10000000) };
      } else if ((ins & 0x0FFFFFF0) === 0x012FFF10 || (ins & 0x0FFFFFF0) === 0x012FFF30) {
        const target = reg(ins & 15), link = (ins & 0x20) !== 0;
        if (link) this.r[14] = pc + 4;
        if (link && this.externalCalls.has(target)) this.externalCalls.get(target)(this);
        else this.r[15] = target;
      } else if ((ins & 0x0FE00F7F) === 0x0E000A10) {
        const rt = (ins >>> 12) & 15;
        if (ins & (1 << 20)) this.r[rt] = this.s[sn];
        else this.s[sn] = reg(rt);
      } else if ((ins & 0x0FFF0F00) === 0x0D2D0A00 || (ins & 0x0FFF0F00) === 0x0CBD0A00) {
        const count = ins & 255;
        if ((ins & 0x0FFF0F00) === 0x0D2D0A00) {
          this.r[13] -= count * 4;
          for (let i = 0; i < count; i++) this.words.set((this.r[13] + i * 4) >>> 0, this.s[sd + i]);
        } else {
          for (let i = 0; i < count; i++) this.s[sd + i] = this.read(this.r[13] + i * 4);
          this.r[13] += count * 4;
        }
      } else if ((ins & 0x0F300F00) === 0x0D100A00 || (ins & 0x0F300F00) === 0x0D000A00) {
        const addr = (reg((ins >>> 16) & 15) + (ins & (1 << 23) ? 1 : -1) * (ins & 255) * 4) >>> 0;
        if (ins & (1 << 20)) this.s[sd] = this.read(addr);
        else this.words.set(addr, this.s[sd]);
      } else if ((ins & 0x0E000F00) === 0x0C000A00) {
        assert.ok(ins & (1 << 23), '只支持递增 VFP 多寄存器传送');
        const rn = (ins >>> 16) & 15, count = ins & 255, start = reg(rn);
        for (let i = 0; i < count; i++) {
          const addr = (start + i * 4) >>> 0;
          if (ins & (1 << 20)) this.s[sd + i] = this.read(addr);
          else this.words.set(addr, this.s[sd + i]);
        }
        if (ins & (1 << 21)) this.r[rn] = start + count * 4;
      } else if ((ins & 0x0FB00FF0) === 0x0EB00A00) {
        const imm = (((ins >>> 16) & 15) << 4) | (ins & 15), b = (imm >>> 6) & 1;
        this.s[sd] = (((imm >>> 7) << 31) | ((1 - b) << 30) | (b ? 0x3E000000 : 0) | ((imm & 63) << 19)) >>> 0;
      } else if ((ins & 0x0FB00F50) === 0x0E200A00 || (ins & 0x0FB00F50) === 0x0E300A00) {
        const a = bitsFloat(this.s[sn]), b = bitsFloat(this.s[sm]);
        this.s[sd] = floatBits((ins & 0x0FB00F50) === 0x0E200A00 ? a * b : a + b);
      } else if ((ins & 0x0FB00F50) === 0x0E000A00 || (ins & 0x0FB00F50) === 0x0E000A40) {
        // VMLA / VMLS 为两个 f32 运算阶段，保留乘法的独立取整。
        const product = Math.fround(bitsFloat(this.s[sn]) * bitsFloat(this.s[sm]));
        this.s[sd] = floatBits(bitsFloat(this.s[sd]) + ((ins & 0x40) ? -product : product));
      } else if ((ins & 0xFFFF0000) >>> 0 === 0xE92D0000) {
        const list = Array.from({ length: 16 }, (_, i) => i).filter(i => ins & (1 << i));
        this.r[13] -= list.length * 4;
        list.forEach((i, n) => this.words.set((this.r[13] + n * 4) >>> 0, reg(i)));
      } else if ((ins & 0xFFFF0000) >>> 0 === 0xE8BD0000) {
        const list = Array.from({ length: 16 }, (_, i) => i).filter(i => ins & (1 << i)), sp = this.r[13];
        list.forEach((i, n) => { this.r[i] = this.read(sp + n * 4); });
        this.r[13] = sp + list.length * 4;
      } else if (((ins >>> 25) & 7) === 4) {
        assert.ok(ins & (1 << 23), '只支持递增 ARM 多寄存器传送');
        const rn = (ins >>> 16) & 15, start = reg(rn), pre = ins & (1 << 24) ? 4 : 0;
        const list = Array.from({ length: 16 }, (_, i) => i).filter(i => ins & (1 << i));
        list.forEach((i, n) => {
          const addr = (start + pre + n * 4) >>> 0;
          if (ins & (1 << 20)) this.r[i] = this.read(addr);
          else this.words.set(addr, reg(i));
        });
        if (ins & (1 << 21)) this.r[rn] = start + list.length * 4;
      } else if ((ins & 0x0FF00000) === 0x03000000 || (ins & 0x0FF00000) === 0x03400000) {
        const rd = (ins >>> 12) & 15, imm = (ins & 0xFFF) | ((ins >>> 4) & 0xF000);
        this.r[rd] = ins & 0x00400000 ? ((this.r[rd] & 0xFFFF) | (imm << 16)) >>> 0 : imm;
      } else if (((ins >>> 25) & 7) === 5) {
        const target = (pc + 8 + ((ins << 8) >> 6)) >>> 0, link = !!(ins & (1 << 24));
        if (link) this.r[14] = pc + 4;
        if (link && this.externalCalls.has(target)) this.externalCalls.get(target)(this);
        else this.r[15] = target;
      } else if (((ins >>> 24) & 15) === 15) {
        this.svcs.push({ number: ins & 0xFFFFFF, regs: Array.from(this.r.slice(0, 5)) });
        this.r[0] = this.svcResult;
      } else if (((ins >>> 26) & 3) === 1) {
        const rn = (ins >>> 16) & 15, rd = (ins >>> 12) & 15;
        assert.ok(ins & (1 << 24));
        const offset = ins & (1 << 25) ? reg(ins & 15) : ins & 0xFFF;
        const addr = (reg(rn) + (ins & (1 << 23) ? offset : -offset)) >>> 0;
        if (ins & (1 << 20)) this.r[rd] = this.read(addr, !!(ins & (1 << 22)));
        else if (ins & (1 << 22)) this.bytes.set(addr, reg(rd) & 255);
        else this.words.set(addr, reg(rd));
        if (ins & (1 << 21)) this.r[rn] = addr;
      } else if (((ins >>> 26) & 3) === 0) {
        const op = (ins >>> 21) & 15, rn = (ins >>> 16) & 15, rd = (ins >>> 12) & 15;
        const value = ins & (1 << 25) ? rotate(ins & 255, ((ins >>> 8) & 15) * 2) : reg(ins & 15), a = reg(rn);
        if (op === 10 || op === 2) {
          const result = (a - value) >>> 0;
          if (op === 10 || (ins & (1 << 20))) this.flags = { n: !!(result >>> 31), z: result === 0, c: a >= value, v: !!(((a ^ value) & (a ^ result)) >>> 31) };
          if (op === 2) this.r[rd] = result;
        } else if (op === 0) {
          assert.equal(ins & (1 << 20), 0, 'ANDS flag update is not implemented');
          this.r[rd] = a & value;
        } else if (op === 12) this.r[rd] = a | value;
        else if (op === 4) this.r[rd] = a + value;
        else if (op === 13) this.r[rd] = value;
        else throw Error('不支持的 ALU 指令 0x' + ins.toString(16));
      } else throw Error('不支持的指令 0x' + ins.toString(16));
    }
  }
}
module.exports = { Arm, loadPlugin, code, floatBits, bitsFloat };
