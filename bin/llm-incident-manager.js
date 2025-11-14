#!/usr/bin/env node

/**
 * LLM Incident Manager Server
 * Start the incident management server
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Find the Rust binary
const binaryName = process.platform === 'win32' ? 'llm-incident-manager.exe' : 'llm-incident-manager';
const possiblePaths = [
  path.join(__dirname, '..', 'target', 'release', binaryName),
  path.join(__dirname, '..', 'target', 'debug', binaryName),
  `/usr/local/bin/${binaryName}`,
  path.join(process.env.HOME || '', '.cargo', 'bin', binaryName)
];

let binaryPath = possiblePaths.find(p => fs.existsSync(p));

if (!binaryPath) {
  console.error('Error: LLM Incident Manager binary not found.');
  console.error('Please run: npm run build');
  console.error('Or install via: cargo install llm-incident-manager');
  process.exit(1);
}

console.log('Starting LLM Incident Manager...');
console.log(`Binary: ${binaryPath}`);

// Pass all arguments to the Rust binary
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env
});

child.on('exit', (code) => {
  process.exit(code || 0);
});

child.on('error', (err) => {
  console.error('Error executing LLM Incident Manager:', err);
  process.exit(1);
});
