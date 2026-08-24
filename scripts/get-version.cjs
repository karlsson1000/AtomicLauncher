const fs = require('fs');

if (!(process.env.CI && process.env.GITHUB_REF_NAME)) {
  console.log('Not a CI release build; skipping version pinning.');
  process.exit(0);
}

const version = process.env.GITHUB_REF_NAME;

const cargoPath = './src-tauri/Cargo.toml';
let cargo = fs.readFileSync(cargoPath, 'utf-8');
cargo = cargo.replace(/^version = ".*"$/m, `version = "${version}"`);
fs.writeFileSync(cargoPath, cargo);

const tauriConfPath = './src-tauri/tauri.conf.json';
let tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf-8'));
tauriConf.version = version;
fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2));

console.log(`Build version (from tag): ${version}`);
