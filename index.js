/**
 * LLM Incident Manager - npm Package Entry Point
 *
 * This is a Rust-based application with npm tooling for convenience.
 * The actual implementation is in Rust. This module provides Node.js
 * utilities for integration and scripting.
 */

const { spawn, execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

/**
 * Get the path to the Rust binary
 */
function getBinaryPath(binaryName) {
  const platformBinary = process.platform === 'win32' ? `${binaryName}.exe` : binaryName;
  const possiblePaths = [
    path.join(__dirname, 'target', 'release', platformBinary),
    path.join(__dirname, 'target', 'debug', platformBinary),
    `/usr/local/bin/${platformBinary}`,
    path.join(process.env.HOME || '', '.cargo', 'bin', platformBinary)
  ];

  return possiblePaths.find(p => fs.existsSync(p));
}

/**
 * Execute the LLM Incident Manager CLI
 */
function cli(args = [], options = {}) {
  const binaryPath = getBinaryPath('llm-im-cli');

  if (!binaryPath) {
    throw new Error('LLM Incident Manager CLI not found. Run: npm run build');
  }

  return spawn(binaryPath, args, {
    stdio: 'inherit',
    ...options
  });
}

/**
 * Start the LLM Incident Manager server
 */
function startServer(args = [], options = {}) {
  const binaryPath = getBinaryPath('llm-incident-manager');

  if (!binaryPath) {
    throw new Error('LLM Incident Manager server not found. Run: npm run build');
  }

  return spawn(binaryPath, args, {
    stdio: 'inherit',
    ...options
  });
}

/**
 * Check if Rust binaries are built
 */
function isBuilt() {
  return getBinaryPath('llm-incident-manager') !== undefined;
}

/**
 * Build the Rust binaries
 */
function build(release = true) {
  const args = release ? ['build', '--release'] : ['build'];
  return execSync(`cargo ${args.join(' ')}`, { stdio: 'inherit' });
}

/**
 * Get version information
 */
function getVersion() {
  const packageJson = require('./package.json');
  const cargoToml = fs.readFileSync(path.join(__dirname, 'Cargo.toml'), 'utf8');
  const cargoVersion = cargoToml.match(/version\s*=\s*"([^"]+)"/)?.[1];

  return {
    npm: packageJson.version,
    cargo: cargoVersion,
    name: packageJson.name
  };
}

module.exports = {
  cli,
  startServer,
  isBuilt,
  build,
  getVersion,
  getBinaryPath
};
