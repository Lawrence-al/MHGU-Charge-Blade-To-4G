// 对实际 NRO 的 ABI 适配代码做寄存器／栈检查，尤其是地面高度的 s0 返回值。
const assert = require('assert/strict');
const path = require('path');
const { Arm, loadPlugin, code, floatBits } = require('./arm');

function verifySaedAbi() {
  const plugin = loadPlugin(path.resolve(__dirname, '../Moveset-to-4G'), 'moveset_to_4g');
  for (const base of [0x70000000, 0x4C000000]) for (const value of [-2000.5, 0, 130.25]) {
    const regs = Array.from({ length: 16 }, (_, i) => 0x1000 + i * 0x111);
    regs[0] = 0x32000000; regs[1] = 0x31000000; regs[13] = 0x22001000; regs[14] = 0xFFFFF000;
    const args = [0x212270E0, 0x10000100, 0x10000200, 7, 0x10000300, 3, 0x01000040, 0];
    const cpu = new Arm(regs, [{ base, data: plugin.nro }, { base: regs[1], data: code(args) }]);
    let calls = 0;
    cpu.externalCalls.set(regs[0], machine => {
      assert.equal(machine.r[13] % 8, 0);
      assert.deepEqual(Array.from(machine.r.slice(0, 4)), args.slice(0, 4));
      assert.deepEqual(Array.from({ length: 4 }, (_, i) => machine.read(machine.r[13] + i * 4)), args.slice(4));
      machine.s[0] = floatBits(value);
      // 返回整数寄存器故意写不同值，确认 Rust 收到的是浮点高度。
      machine.r[0] = 0xDEADC0DE;
      machine.r[1] = 0x11111111;
      machine.r[2] = 0x22222222;
      machine.r[3] = 0x33333333;
      machine.r[12] = 0xCCCCCCCC;
      calls++;
    });
    cpu.run(base + plugin.symbol('float_call8').value, regs[14]);
    assert.equal(calls, 1);
    assert.equal(cpu.r[0], floatBits(value));
    for (let i = 4; i <= 13; i++) if (i !== 12) assert.equal(cpu.r[i], regs[i]);

    const trigRegs = [...regs]; trigRegs[1] = floatBits(1.25);
    const trig = new Arm(trigRegs, [{ base, data: plugin.nro }]);
    trig.externalCalls.set(trigRegs[0], machine => {
      assert.equal(machine.s[0], floatBits(1.25));
      assert.equal(machine.r[13] % 8, 0);
      machine.s[0] = floatBits(value);
      machine.r[0] = 0xBAD0BAD0;
    });
    trig.run(base + plugin.symbol('trig_call').value, trigRegs[14]);
    assert.equal(trig.r[0], floatBits(value));
    for (let i = 4; i <= 13; i++) if (i !== 12) assert.equal(trig.r[i], trigRegs[i]);
  }
  console.log('PASS: compiled SAED native float adapters, all eight ground-query arguments, s0 results, callee-saved registers/stack and two NRO load bases');
}
module.exports = { verifySaedAbi };
if (require.main === module) verifySaedAbi();
