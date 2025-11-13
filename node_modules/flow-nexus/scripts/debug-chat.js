#!/usr/bin/env node

import { spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const cliPath = path.join(__dirname, '../cli.js');
const args = ['chat', 'Hello Queen Seraphina, what is Flow Nexus?'];

console.log('Running:', 'node', cliPath, ...args);
console.log('---\n');

const child = spawn('node', [cliPath, ...args], {
  cwd: path.join(__dirname, '..'),
  env: { ...process.env },
  stdio: 'inherit'
});

child.on('error', (error) => {
  console.error('Error:', error);
});

child.on('exit', (code) => {
  console.log(`\nProcess exited with code ${code}`);
});