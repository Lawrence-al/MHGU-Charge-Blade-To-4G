// 主机测试只替换 ARM 汇编入口和游戏内存，配置／补丁选择逻辑仍编译真实 Rust。
const fs = require('fs');
const path = require('path');
const cp = require('child_process');
const root = path.resolve(__dirname, '..');
const rustc = process.env.RUSTC || 'rustc';
const dir = path.join(root, 'Moveset-to-4G/target/integration-check');
fs.mkdirSync(dir, {recursive: true});
const rustPath = p => JSON.stringify(p.replaceAll('\\', '/'));
// 纯逻辑测试直接链接生产共用库；主机不调用 ARM 原生函数。
const gameRlib = path.join(dir, 'libcb_game.rlib');
const gameVersionFile = gameRlib + '.rustc-version';
const gameVersion = cp.execFileSync(rustc, ['-vV'], {encoding:'utf8'});
const gameSource = path.join(root, 'game-api/src');
const gameNewest = Math.max(...fs.readdirSync(gameSource).filter(n => n.endsWith('.rs'))
  .map(n => fs.statSync(path.join(gameSource, n)).mtimeMs));
if (!fs.existsSync(gameRlib) || !fs.existsSync(gameVersionFile)
    || fs.readFileSync(gameVersionFile,'utf8') !== gameVersion
    || fs.statSync(gameRlib).mtimeMs < gameNewest) {
  cp.execFileSync(rustc, ['--edition=2024', '--crate-name', 'cb_game',
    '--crate-type', 'rlib', path.join(gameSource, 'lib.rs'), '-o', gameRlib], {stdio: 'inherit'});
  fs.writeFileSync(gameVersionFile,gameVersion);
}
function modulePath(file, name) {
  return `#[path = ${rustPath(path.join(root, file))}] pub mod ${name};`;
}
function hostBridge(file, name) {
  const source = fs.readFileSync(path.join(root, file), 'utf8')
    .replace(/use core::arch::\{[^;]+;\s*/g, '')
    .replace(/global_asm!\([^;]+;\s*/g, '')
    .replace(/#\[unsafe\(naked\)\]\s*unsafe extern "C" fn (\w+)\(([^)]*)\) \{[\s\S]*?\n\}/g, 'unsafe extern "C" fn $1($2) {}');
  if (source.includes('naked_asm!') || source.includes('global_asm!')) throw Error('ARM 适配不完整');
  const output = path.join(dir, name + '.rs');
  fs.writeFileSync(output, source);
  return `#[path = ${rustPath(output)}] mod ${name};`;
}
function run(name, source, args = [], test = false) {
  const file = path.join(dir, name + '.rs'), exe = path.join(dir, name + '.exe');
  fs.writeFileSync(file, source);
  cp.execFileSync(rustc, ['--edition=2024', ...(test ? ['--test'] : []),
    '--extern', 'cb_game=' + gameRlib, file, '-o', exe], {stdio: 'inherit'});
  cp.execFileSync(exe, args, {stdio: 'inherit'});
}
module.exports = {fs, path, root, dir, rustPath, modulePath, hostBridge, run, gameRlib, rustc};
