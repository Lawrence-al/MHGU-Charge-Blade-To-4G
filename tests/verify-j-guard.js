// 路由内容运行实际Rust；编译后ARM另验证桥接调用、寄存器与候选链筛选。
require('./verify-yellow-route-host');
const fs = require('fs');
const path = require('path');
const assert = require('assert/strict');
const { Arm, loadPlugin, code, floatBits } = require('./arm');
const root = path.resolve(__dirname, '..');
const plugin = loadPlugin(path.join(root, 'Moveset-to-4G'), 'moveset_to_4g');
const bridge = plugin.symbol('j_guard_bridge'), slot = plugin.symbol('J_GUARD_CONTINUATION');
const helper = plugin.symbol('rewrite_yellow_route');
// 独立快照：原 AED 命令的前 24 字节，不依赖未发布的完整 XFS。
const expected = code([4456464,524288,27,0,0,0]);
assert.equal(expected.readUInt16LE(2), 0x44);
assert.equal(expected.readUInt16LE(4), 0);
assert.equal(expected.readUInt32LE(8), 0x1B);

let cases = 0;
for (const [game, base] of [[0x60000000, 0x70000000], [0x34000000, 0x4C000000]]) {
  const site = game + 0x002EA87C, end = site + 8, player = 0x21000000, equip = 0x22000000, command = 0x23000000;
  for (const chain of [0x819, 0x81A, 0x81B, 0x7F9, 0x7FA, 0x7FB,0x801,0x802,0x803,0x804]) for (const index of [0, 1, 2, 3, 4, 7, 8, 12]) {
    for (let flags=0;flags<16;flags++) for (const enabled of [false,true]) {
      const original = chain === 0x7FA
        ? code([0x00240010, 0x00088000, 0x1B, 0, 0, 0, floatBits(34), floatBits(10)])
        : code([0x00480010, 0x00088000, 0x1A, 0, 0, 0, floatBits(30), floatBits(20)]);
      const regs = Array.from({ length: 16 }, (_, i) => (0x11000000 + i * 0x10101) >>> 0);
      regs[4] = player; regs[5] = command; regs[8] = index; regs[13] = 0x24001000;
      const machine = new Arm(regs, [{ base, data: plugin.nro }, { base: command, data: original }], [
        [site, 0xE51FF004], [site + 4, base + bridge.value], [base + slot.value, end],
        [player + 0x24AC, chain], [player + 0x13C4, equip],
      ]);
      const read=machine.read.bind(machine);
      machine.read=(address,byte=false)=>{const w=read(address,byte);return !byte&&(w===0xED2D0B20||w===0xECBD0B20)?w-0x100:w;};
      let calls=0;
      machine.externalCalls.set(base+helper.value,m=>{
        assert.deepEqual([...m.r.slice(0,3)],[player,command,index]);assert.equal(m.r[13]%8,0);calls++;
        if(enabled)for(let i=0;i<24;i+=4)m.words.set(command+i,expected.readUInt32LE(i));
        for(const r of [0,1,2,3,12])m.r[r]=0xAABB0000+r;
        for(let i=0;i<32;i++)m.s[i]=0xBB000000+i;
        m.flags={n:true,z:false,c:true,v:false};
      });
      machine.flags = { n: !!(flags & 8), z: !!(flags & 4), c: !!(flags & 2), v: !!(flags & 1) };
      const originalFlags = { ...machine.flags }, originalVfp = [...machine.s];
      machine.run(site, end, 100);
      const actual = Buffer.alloc(32);
      for (let field = 0; field < 32; field += 4) actual.writeUInt32LE(machine.read(command + field), field);
      const wanted = Buffer.from(original);
      const candidate=[0x81A,0x7FA,0x802,0x803].includes(chain);
      assert.equal(calls,+candidate);
      if (candidate&&enabled) expected.copy(wanted);
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
if (process.argv[2]) {
  const styles = require('./command-resources')(process.argv[2]);
  for (const style of styles) {
    const commands = style.tables.find(t => t.chain === 0x7FA).commands;
    if (style.style === 1) { assert.equal(commands.length,0,'Striker has no shield-thrust chain'); continue; }
    const entry = commands[1];
    assert.equal(entry.path, 'player\\cmd\\w14\\plcmd_w14_0x0010_036a');
    assert.deepEqual([entry.pre,entry.start,entry.end],[10,34,0]);
    assert.deepEqual(entry.fields,{st:16,no:36,param:0,type:32776,args:[2,0,0,94]});
  }
  for(const style of styles)for(const chain of [0x802,0x803]){
    const commands=style.tables.find(t=>t.chain===chain).commands;
    assert.ok(commands.length>=2);assert.equal(commands[1].start,chain===0x802?30:42);assert.equal(commands[1].end,0);
  }
  console.log('PASS: every original style with shield thrust has the same 0x7FA entry 1 and windows; Striker chain is empty');
}
console.log('PASS: ' + cases + ' compiled yellow-route bridge cases; only J guard/shield thrust/small-medium recoil chains call Rust, 8-byte stack, NZCV/registers/all VFP/ASLR and windows retained');
