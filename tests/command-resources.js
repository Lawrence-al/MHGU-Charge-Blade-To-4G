// 从原版 ARC/XFS 读取风格派生表；期望数据不依赖插件里的合并算法。
const fs=require('fs'),path=require('path'),zlib=require('zlib'),assert=require('assert/strict');
function entries(file) {
  const b=fs.readFileSync(file),out=[];
  assert.equal(b.toString('ascii',0,4),'ARC\0');
  for(let i=0;i<b.readUInt16LE(6);i++){
    const at=12+i*80,offset=b.readUInt32LE(at+76),size=b.readUInt32LE(at+68);
    out.push({name:b.toString('utf8',at,b.indexOf(0,at)),type:b.readUInt32LE(at+64),bytes:zlib.inflateSync(b.subarray(offset,offset+size))});
  }
  return out;
}
function tables(bytes) {
  let at=bytes.indexOf(Buffer.from('05000200','hex'));const out=[];
  for(let i=0;i<116;i++){
    assert.equal(bytes.readUInt16LE(at),5);
    const end=at+4+bytes.readUInt32LE(at+4),arr=at+12,count=bytes.readUInt32LE(arr+13),commands=[];
    assert.equal(bytes.readUInt16LE(arr),3);let pos=arr+17;
    for(let j=0;j<count;j++){
      assert.equal(bytes.readUInt16LE(pos),7);const size=bytes.readUInt32LE(pos+4);
      let p=pos+18;p=bytes.indexOf(0,p)+1;
      const pathEnd=bytes.indexOf(0,p),name=bytes.toString('utf8',p,pathEnd);p=pathEnd+1;
      const frames=[];
      for(let k=0;k<3;k++){assert.equal(bytes.readUInt32LE(p),1);frames.push(bytes.readFloatLE(p+4));p+=8;}
      assert.equal(p,pos+4+size);commands.push({path:name,pre:frames[0],start:frames[1],end:frames[2]});pos=p;
    }
    assert.equal(pos,end);out.push({chain:2000+i,commands});at=end;
  }
  return out;
}
module.exports=function load(dump) {
  assert.ok(dump, '提供 MHGU dump 目录作为第一个参数');
  return Array.from({length:6},(_,style)=>{
    const files=entries(path.join(dump,'romfs/nativeNX/arc/player/quest/w14_m'+style+'.arc'));
    const parsed=tables(files.find(f=>f.type===0x4B4C2BD3).bytes);
    for(const t of parsed)for(const c of t.commands){
      const b=files.find(f=>f.name===c.path)?.bytes;
      if(b?.length===556)c.fields={st:b.readUInt16LE(0x1f8),no:b.readUInt16LE(0x1fe),param:b.readUInt16LE(0x204),type:b.readUInt16LE(0x20a),args:[0x210,0x218,0x220,0x228].map(p=>b.readUInt32LE(p))};
    }
    return {style,tables:parsed};
  });
};
