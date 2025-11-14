#!/usr/bin/env node

/**
 * Installation script for LLM Incident Manager
 * Checks prerequisites and optionally builds Rust binaries
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('');
console.log('🚀 Installing LLM Incident Manager...');
console.log('');

// Check if cargo is installed
function hasCommand(cmd) {
  try {
    execSync(`${cmd} --version`, { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

const hasCargo = hasCommand('cargo');

if (!hasCargo) {
  console.log('⚠️  Rust/Cargo not found on this system.');
  console.log('');
  console.log('The LLM Incident Manager is written in Rust and requires cargo to build.');
  console.log('');
  console.log('To install Rust:');
  console.log('  curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh');
  console.log('');
  console.log('Or visit: https://rustup.rs/');
  console.log('');
  console.log('After installing Rust, run: npm install');
  console.log('');
  process.exit(0); // Don't fail installation, just warn
}

console.log('✓ Rust/Cargo found');

// Check Rust version
try {
  const rustVersion = execSync('rustc --version', { encoding: 'utf8' });
  console.log(`✓ ${rustVersion.trim()}`);
} catch (err) {
  console.error('Warning: Could not check Rust version');
}

console.log('');
console.log('To build the Rust binaries, run:');
console.log('  npm run build');
console.log('');
console.log('To start the server:');
console.log('  npm start');
console.log('');
console.log('For development:');
console.log('  npm run start:dev');
console.log('');
console.log('Installation complete! 🎉');
console.log('');
