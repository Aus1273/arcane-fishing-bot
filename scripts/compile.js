#!/usr/bin/env node
import { spawn } from 'node:child_process';
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
  await runStep('Install JavaScript dependencies', 'npm', ['install']);
  await runStep('Build the Svelte frontend', 'npm', ['run', 'build']);
  await runStep('Compile the Rust core (release)', 'cargo', ['build', '--release']);

  console.log('\nBuild complete!');
  console.log('- UI build output: dist/');
  console.log('- Rust release binary: target/release/arcane-fishing-bot');
  console.log("Run 'npm run tauri dev' to open the desktop app using the built UI.");
}

main().catch((error) => {
  console.error(error.message);
  process.exit(1);
});
