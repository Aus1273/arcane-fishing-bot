import { app, BrowserWindow, ipcMain, nativeTheme } from 'electron';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';
import fs from 'node:fs/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const statePath = path.join(app.getPath('userData'), 'bot-state.json');
let cachedState = null;

const defaultState = () => ({
  config: {
    color_tolerance: 10,
    autoclick_interval_ms: 70,
    fish_per_feed: 5,
    webhook_url: '',
    screenshot_interval_mins: 60,
    screenshot_enabled: true,
    red_region: { x: 1321, y: 99, width: 768, height: 546 },
    yellow_region: { x: 3097, y: 1234, width: 342, height: 205 },
    hunger_region: { x: 274, y: 1301, width: 43, height: 36 },
    region_preset: '3440x1440',
    startup_delay_ms: 3000,
    detection_interval_ms: 50,
    max_fishing_timeout_ms: 25000,
    rod_lure_value: 1.0,
    always_on_top: false,
    auto_save_enabled: true,
    failsafe_enabled: true,
    advanced_detection: false,
  },
  stats: {
    total_fish_caught: 0,
    total_runtime_seconds: 0,
    sessions_completed: 0,
    last_updated: new Date().toISOString(),
    best_session_fish: 0,
    average_fish_per_hour: 0,
    total_feeds: 0,
    uptime_percentage: 100,
  },
  session: {
    running: false,
    last_action: 'Idle',
    fish_caught: 0,
    hunger_level: 100,
    errors_count: 0,
    uptime_minutes: 0,
    started_at: null,
  },
});

async function loadState() {
  if (cachedState) return cachedState;
  if (!existsSync(statePath)) {
    cachedState = defaultState();
    await saveState();
    return cachedState;
  }

  const raw = await fs.readFile(statePath, 'utf-8');
  cachedState = JSON.parse(raw);
  return cachedState;
}

async function saveState() {
  if (!cachedState) return;
  await fs.mkdir(path.dirname(statePath), { recursive: true });
  await fs.writeFile(statePath, JSON.stringify(cachedState, null, 2), 'utf-8');
}

function updateUptime() {
  if (!cachedState?.session.running || !cachedState.session.started_at) return;
  const now = Date.now();
  const elapsedMinutes = Math.floor((now - cachedState.session.started_at) / 60000);
  cachedState.session.uptime_minutes = elapsedMinutes;
}

async function getState() {
  await loadState();
  updateUptime();
  return cachedState;
}

async function startSession() {
  await loadState();
  cachedState.session.running = true;
  cachedState.session.started_at = Date.now();
  cachedState.session.last_action = 'Session started';
  await saveState();
}

async function stopSession() {
  await loadState();
  updateUptime();
  cachedState.session.running = false;
  cachedState.session.started_at = null;
  cachedState.session.last_action = 'Session stopped';
  cachedState.stats.sessions_completed += 1;
  cachedState.stats.last_updated = new Date().toISOString();
  await saveState();
}

async function saveConfig(config) {
  await loadState();
  cachedState.config = config;
  cachedState.session.last_action = 'Config updated';
  await saveState();
}

function createWindow() {
  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    backgroundColor: nativeTheme.shouldUseDarkColors ? '#0c0f1d' : '#ffffff',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
    },
  });

  win.setMenuBarVisibility(false);
  const indexHtml = path.join(__dirname, '../dist/index.html');
  win.loadFile(indexHtml);
}

ipcMain.handle('get-state', async () => getState());
ipcMain.handle('get-config', async () => (await getState()).config);
ipcMain.handle('get-stats', async () => {
  const state = await getState();
  return { stats: state.stats, session: state.session };
});
ipcMain.handle('save-config', async (_event, config) => saveConfig(config));
ipcMain.handle('start-session', async () => startSession());
ipcMain.handle('stop-session', async () => stopSession());

app.whenReady().then(() => {
  createWindow();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
