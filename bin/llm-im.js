#!/usr/bin/env node

/**
 * LLM Incident Manager CLI
 * Enterprise-grade incident management system for LLM operations
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Find the Rust binary
const binaryName = process.platform === 'win32' ? 'llm-im-cli.exe' : 'llm-im-cli';
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
