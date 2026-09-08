#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.join(__dirname, '..');

function runStep(label, command, args) {
  console.log(`\n=== ${label} ===`);
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: projectRoot,
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });

    child.on('error', reject);
    child.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(`${label} failed with exit code ${code}`));
        return;
      }
      resolve();
    });
  });
}

async function main() {
  if (!existsSync(path.join(projectRoot, 'node_modules'))) {
    await runStep('Install locked JavaScript dependencies', 'npm', ['ci']);
  }
  await runStep('Check frontend types', 'npm', ['run', 'check']);
  await runStep('Build the Tauri desktop app', 'npm', ['run', 'tauri', 'build']);
  console.log('Build complete: target/release/');
}

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
