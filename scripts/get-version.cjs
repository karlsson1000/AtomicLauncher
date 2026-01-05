const { execSync } = require('child_process');
const fs = require('fs');

const getCommitHash = () => {
  try {
    return execSync('git rev-parse --short HEAD').toString().trim();
  } catch {
    return 'dev';
  }
};

const getDateVersion = () => {
  const now = new Date();
  const year = now.getFullYear().toString().slice(-2);
  const month = now.getMonth() + 1;
  const day = now.getDate();
  return `${year}.${month}.${day}`;
};

const dateVersion = getDateVersion();
const commitHash = getCommitHash();

// Update Cargo.toml
const cargoPath = './src-tauri/Cargo.toml';
let cargo = fs.readFileSync(cargoPath, 'utf-8');
cargo = cargo.replace(/^version = ".*"$/m, `version = "${dateVersion}"`);
fs.writeFileSync(cargoPath, cargo);

// Update tauri.conf.json
const tauriConfPath = './src-tauri/tauri.conf.json';
let tauriConf = JSON.parse(fs.readFileSync(tauriConfPath, 'utf-8'));
tauriConf.version = dateVersion;
fs.writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2));

const commitHashPath = './src-tauri/commit_hash.txt';
fs.writeFileSync(commitHashPath, commitHash);

console.log(`Build version: ${dateVersion}`);
console.log(`Commit hash: ${commitHash}`);