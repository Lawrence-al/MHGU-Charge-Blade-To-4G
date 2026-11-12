// 执行实际NRO的盾突和任务初始化入口；游戏创建器与音频队列使用替身。
const path = require('path');
const assert = require('assert/strict');
const { Arm, loadPlugin, bitsFloat, floatBits } = require('./arm');
const root = path.resolve(__dirname, '..');
const plugin = loadPlugin(path.join(root, 'Presentation-to-4G'), 'presentation_to_4g');
const hook = plugin.symbol('hook_cb_shield_thrust_action8___detour');
const slot = plugin.symbol('hook_cb_shield_thrust_action10___ORIGINAL');
const visual = plugin.symbol('gaxe13create_visual');
const lifetime = plugin.symbol('effect14track_lifetime');
const assets = plugin.symbol('shield_thrust12sound_assets');
const getConfig = plugin.symbol('config_file3get');
const configStorage = plugin.symbol('config_file6CONFIG');
const stateStorage = plugin.symbol('state5STATE');
const clear = plugin.symbol('__aeabi_memclr8');
// 从实际PIC取址和持续时间加载推导Config，不假设Rust OnceLock的私有布局。
const durationLoad = [];
for (let p=hook.value; p<hook.value+hook.size; p+=4) {
  if ((plugin.nro.readUInt32LE(p)&0xFFF0FF00)>>>0===0xED900A00) durationLoad.push(p);
}
assert.equal(durationLoad.length,2);
const durationInstruction=plugin.nro.readUInt32LE(durationLoad[0]);
assert.equal(plugin.nro.readUInt32LE(durationLoad[1]),durationInstruction);
const configRegister=(durationInstruction>>>16)&15;
const durationOffset=(durationInstruction&255)*4;
let configValue;
for(let p=hook.value+4;p<durationLoad[0];p+=4) {
  if(plugin.nro.readUInt32LE(p)===(0xE08F0000|(configRegister<<12)|configRegister)>>>0){
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
assert.ok(configValue>=configStorage.value && configValue+16<=configStorage.value+configStorage.size);
const selectorOffset=228;
assert.ok(Array.from({length:hook.size/4},(_,i)=>plugin.nro.readUInt32LE(hook.value+i*4))
  .includes((0xE5D00000|(configRegister<<16)|selectorOffset)>>>0),'4G selector load');
const modes = [
  { effects:true, sounds:true, visual:true, audio:true, manager:true },
  { effects:true, sounds:false, visual:true, audio:true, manager:true },
  { effects:false, sounds:true, visual:true, audio:true, manager:true },
  { effects:false, sounds:false, visual:true, audio:true, manager:true },
  { effects:true, sounds:true, visual:false, audio:true, manager:true },
  { effects:true, sounds:true, visual:true, audio:false, manager:true },
  { effects:true, sounds:true, visual:true, audio:true, manager:false },
];
let count = 0;
for (const [base, game] of [[0x70000000,0x61000000], [0x4C000000,0x21000000]]) {
  for (let variant = 0; variant < 6; variant++) for (let state = 0; state < 4; state++)
    for (let shield = 0; shield < 4; shield++) for (const result of [0, 1, 0x12345678])
      for (const duration of [17.5, 30]) for (const element of [false,true])
        for (const fourG of [false,true]) for (const mode of modes) {
      const r = Array.from({ length: 16 }, (_, i) => (0x11000000 + i * 0x10101) >>> 0);
      r[0] = 0x20000000; r[1] = variant; r[13] = 0x22001000; r[14] = 0xFFFFF000;
      const original = 0x23000000, reference=0x23000100, manager=0x31000000;
      const cpu = new Arm(r, [{ base, data: plugin.nro }], [
        [base + slot.value, original], [r[0] + 0x3344, shield],
        [r[0]+0x13C4,0x32000000], [0x32000356,Number(element)],
        [r[0]+0x23A4,0x33000000], [0x33000000,0x34000000], [0x34000130,reference],
        [r[0]+0x2634,0x35000000], [base+stateStorage.value,game],
        [0x35001388,0x39000000],
        [base+configValue+12,floatBits(duration)],
        [game+0x01896F5C,mode.manager?manager:0],
        ...[1.25,2.5,-3.75].map((v,i)=>[r[0]+0x40+i*4,floatBits(v)]),
      ], [[r[0]+5,state], [0x32000357,0], [base+configValue+8,Number(mode.effects)],
        [base+configValue+selectorOffset,Number(fourG)], [base+configValue+1,Number(mode.sounds)]]);
      const events = [];
      cpu.externalCalls.set(base+getConfig.value,c=>{c.r[0]=base+configValue;});
      cpu.externalCalls.set(original, c => {
        assert.deepEqual(Array.from(c.r.slice(0, 2)), [r[0], variant]);
        assert.equal(c.r[13] % 8, 0);
        events.push('original');
        if (state === 0) c.bytes.set(r[0] + 5, variant < 3 ? 1 : 2);
        c.r[0] = result; c.r[1] = 0xBB; c.r[2] = 0xCC; c.r[3] = 0xDD; c.r[12] = 0xEE;
      });
      cpu.externalCalls.set(base+clear.value,c=>{
        assert.equal(c.r[1],224);for(let i=0;i<224;i+=4)c.words.set(c.r[0]+i,0);
      });
      cpu.externalCalls.set(game+0x00040B10,c=>{assert.equal(c.r[0]%16,0);});
      cpu.externalCalls.set(game+0x00040B50,()=>{});
      cpu.externalCalls.set(reference,c=>{assert.equal(c.r[0],0x33000000);c.r[0]=0x36000000;});
      function checkAttachment(c,params){
        assert.equal(c.read(params+0xD0),0x36000000,'shield reference');
        assert.equal(c.read(params+0x52,true),0,'shield bone 0 low');
        assert.equal(c.read(params+0x53,true),0,'shield bone 0 high');
        assert.equal(c.read(params+0x14)&0x80,0,'initial rotation binding');
        assert.equal(c.read(params+0x18)&(1<<23),0,'initial PEL transform flags');
        assert.equal(c.read(params+0xA8)&1,0,'initial quaternion flag');
      }
      cpu.externalCalls.set(base+visual.value,c=>{
        assert.ok(fourG);
        assert.deepEqual(Array.from(c.r.slice(0,3)),[0x35000000,105,element?2:1]);
        checkAttachment(c,c.r[3]);
        events.push('4G-effect');c.r[0]=Number(mode.visual);c.r[1]=0x37000000;
      });
      cpu.externalCalls.set(game+0x004A1E08,c=>{
        assert.ok(!fourG);
        assert.deepEqual(Array.from(c.r.slice(0,4)),[0x35000000,0x39000000,2,element?0x259:0x258]);
        const params=c.read(c.r[13]);checkAttachment(c,params);
        assert.equal(c.read(params+8),3,'GU priority');
        assert.equal(c.read(c.r[13]+4),0,'GU PTS');
        assert.equal(c.r[13]%8,0);
        events.push('GU-effect');c.r[0]=mode.visual?0x37000000:0;
      });
      cpu.externalCalls.set(base+lifetime.value,c=>{
        assert.equal(c.r[0],mode.visual?0x37000000:0);assert.equal(bitsFloat(c.s[0]),duration);
        events.push('duration');
      });
      cpu.externalCalls.set(base+assets.value,c=>{c.r[0]=Number(mode.audio);c.r[1]=0x38000000;});
      cpu.externalCalls.set(game+0x004F0668,c=>{
        assert.deepEqual(Array.from(c.r.slice(0,4)),[manager,0x38000000,0x55,1]);
        const position=c.read(c.r[13]);
        assert.deepEqual([0,4,8].map(i=>bitsFloat(c.read(position+i))),[1.25,2.5,-3.75]);
        assert.equal(c.r[13] % 8, 0);
        events.push('sound');c.r[0]=0xDEADBEEF;
      });
      cpu.run(base+hook.value,r[14],350);
      const expected=['original'];
      if(state===0&&(shield===2||shield===3)){
        if(mode.effects){expected.push(fourG?'4G-effect':'GU-effect');if(mode.visual||!fourG)expected.push('duration');}
        if(mode.sounds&&mode.audio&&mode.manager)expected.push('sound');
      }
      assert.deepEqual(events,expected);
      assert.equal(cpu.r[0], result);
      for (let i = 4; i <= 11; i++) assert.equal(cpu.r[i], r[i]);
      assert.equal(cpu.r[13], r[13]);
      count++;
    }
}
assert.equal(count,32256);
const quest=plugin.symbol('hook_cb_quest_init8___detour');
const questSlot=plugin.symbol('hook_cb_quest_init10___ORIGINAL');
const pel=plugin.symbol('gaxe3pel');
for(const base of [0x70000000,0x4C000000])for(const fourG of [false,true])for(const mode of modes){
  const r=Array(16).fill(0);r[0]=0x20000000;r[13]=0x22001000;r[14]=0xFFFFF000;
  const cpu=new Arm(r,[{base,data:plugin.nro}],[[base+questSlot.value,0x23000000]],
    [[base+configValue+8,Number(mode.effects)],[base+configValue+selectorOffset,Number(fourG)],
      [base+configValue+1,Number(mode.sounds)]]);
  const events=[];
  cpu.externalCalls.set(0x23000000,c=>{assert.equal(c.r[0],r[0]);events.push('original');c.r[0]=0x12345678;});
  cpu.externalCalls.set(base+getConfig.value,c=>{c.r[0]=base+configValue;});
  cpu.externalCalls.set(base+pel.value,c=>{events.push('load-effect');c.r[0]=1;c.r[1]=0x30000000;});
  cpu.externalCalls.set(base+assets.value,c=>{events.push('load-sound');c.r[0]=1;c.r[1]=0x38000000;});
  cpu.run(base+quest.value,r[14],100);
  assert.deepEqual(events,['original',...(mode.effects&&fourG?['load-effect']:[]),...(mode.sounds?['load-sound']:[])]);
  assert.equal(cpu.r[0],0x12345678);assert.equal(cpu.r[13],r[13]);
}
console.log('PASS: '+count+' actual NRO shield-thrust cases and 28 quest-init cases; GU 0x258/0x259 or initial 4G 105, isolated request 0x55, red/blue start once, separate switches/failures, result/registers/stack preserved');
// GU 1.4.0原main的RVA 0x004F0668，SHA256 b3935415499549adaf4c0e89822831daa175cfd456a7990791c79be3997645ef。
// 实际执行原生显式SRQR队列入口，核对第5参数、坐标复制及排队成功后的资源加引用。
const nativeQueue=Buffer.from('30482de9000051e33088bd0810c09de5ece080e20000a0e30040dee5000054e30800000a2440dee5000054e30400000a020080e248e08ee2400050e3f5ffffba3088bde824e08ee20100a0e30040a0e30e50a0e30000cee50140cee50250cee510208ee514308ee50300cee51c408ee500009ce504008ee504009ce508008ee508009ce50c008ee520108ee50100a0e13048bde8d15b0eea','hex');
const retainStub=Buffer.from('10402de908c09fe53cff2fe11080bde80000a0e100000023','hex');
for(const game of [0x61000000,0x21000000])for(const available of [0,1,63,64])for(const resource of [0,0x32000000]){
  const manager=0x30000000,position=0x33000000,r=Array(16).fill(0);
  Object.assign(r,{0:manager,1:resource,2:0x55,3:1,13:0x22001000,14:0xFFFFF000});
  const slots=Array.from({length:64},(_,i)=>[manager+236+i*36,Number(i<available)]);
  const coords=[1.5,2.75,-3.0];
  const cpu=new Arm(r,[{base:game+0x004F0668,data:nativeQueue},{base:game+0x00887648,data:retainStub}],
    [[r[13],position],...coords.map((v,i)=>[position+4*i,floatBits(v)])],slots);
  let refs=0;
  cpu.externalCalls.set(0x23000000,c=>{assert.equal(c.r[0],resource);refs++;c.r[0]=3;});
  cpu.run(game+0x004F0668,r[14],1000);
  const queued=resource!==0&&available<64;
  assert.equal(refs,Number(queued));assert.equal(cpu.r[13],r[13]);
  if(queued){
    const slot=manager+236+available*36;
    assert.deepEqual([0,1,2,3].map(i=>cpu.read(slot+i,true)),[1,0,14,1]);
    assert.deepEqual([16,20,28,32].map(i=>cpu.read(slot+i)),[0x55,1,0,resource]);
    cpu.words.set(position,floatBits(100));
    assert.deepEqual([4,8,12].map(i=>bitsFloat(cpu.read(slot+i))),coords);
  }
}
console.log('PASS: 16 original GU queue cases; explicit resource, request 0x55, create event 1, copied position, retained queued resource, null/full queue handling');
if(process.argv[2]){
  const fs=require('fs'),{nsoHeader,decompressSegment}=require('./nso');
  const bytes=fs.readFileSync(path.join(process.argv[2],'exefs/main')),h=nsoHeader(bytes);
  const text=h.flags&1?decompressSegment(bytes,h.textFileOff,h.textFileSize,h.textSize):bytes.subarray(h.textFileOff,h.textFileOff+h.textSize);
  assert.deepEqual(text.subarray(0x004F0668,0x004F0668+nativeQueue.length),nativeQueue);
  assert.equal(text.subarray(0x011D415C,0x011D415C+8).toString('hex'),'30482de90040a0e1');
  console.log('PASS: original GU dump matches explicit sound queue and quest-init hook prologue');
}
require('./verify-shield-thrust-once')();
