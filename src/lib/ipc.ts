export type Region = { x: number; y: number; width: number; height: number };

export type BotConfig = {
  color_tolerance: number;
  autoclick_interval_ms: number;
  fish_per_feed: number;
  webhook_url: string;
  screenshot_interval_mins: number;
  screenshot_enabled: boolean;
  red_region: Region;
  yellow_region: Region;
  hunger_region: Region;
  region_preset: string;
  startup_delay_ms: number;
  detection_interval_ms: number;
  max_fishing_timeout_ms: number;
  rod_lure_value: number;
  always_on_top: boolean;
  auto_save_enabled: boolean;
  failsafe_enabled: boolean;
  advanced_detection: boolean;
};

export type LifetimeStats = {
  total_fish_caught: number;
  total_runtime_seconds: number;
  sessions_completed: number;
  last_updated: string;
  best_session_fish: number;
  average_fish_per_hour: number;
  total_feeds: number;
  uptime_percentage: number;
};

export type SessionState = {
  running: boolean;
  last_action: string;
  fish_caught: number;
  hunger_level: number;
  errors_count: number;
  uptime_minutes: number;
  started_at?: number | null;
};

export type BotState = {
  config: BotConfig;
  stats: LifetimeStats;
  session: SessionState;
};

export type BotApi = {
  getState: () => Promise<BotState>;
  getConfig: () => Promise<BotConfig>;
  getStats: () => Promise<{ stats: LifetimeStats; session: SessionState }>;
  saveConfig: (config: BotConfig) => Promise<void>;
  startSession: () => Promise<void>;
  stopSession: () => Promise<void>;
};

function fallbackState(): BotState {
  return {
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
  };
}

function getApi(): BotApi | undefined {
  if (typeof window === 'undefined') return undefined;
  return window.bot;
}

let inMemoryState: BotState | null = null;

function ensureFallbackState(): BotState {
  if (!inMemoryState) {
    inMemoryState = fallbackState();
  }
  return inMemoryState;
}

export async function getState(): Promise<BotState> {
  const api = getApi();
  if (api?.getState) return api.getState();
  return ensureFallbackState();
}

export async function getConfig(): Promise<BotConfig> {
  const api = getApi();
  if (api?.getConfig) return api.getConfig();
  return ensureFallbackState().config;
}

export async function getStats(): Promise<{ stats: LifetimeStats; session: SessionState }> {
  const api = getApi();
  if (api?.getStats) return api.getStats();
  const fallback = ensureFallbackState();
  return { stats: fallback.stats, session: fallback.session };
}

export async function saveConfig(config: BotConfig): Promise<void> {
  const api = getApi();
  if (api?.saveConfig) return api.saveConfig(config);

  const state = ensureFallbackState();
  state.config = config;
  state.session.last_action = 'Config updated';
}

export async function startSession(): Promise<void> {
  const api = getApi();
  if (api?.startSession) return api.startSession();

  const state = ensureFallbackState();
  state.session.running = true;
  state.session.started_at = Date.now();
  state.session.last_action = 'Session started';
}

export async function stopSession(): Promise<void> {
  const api = getApi();
  if (api?.stopSession) return api.stopSession();

  const state = ensureFallbackState();
  state.session.running = false;
  state.session.started_at = null;
  state.session.last_action = 'Session stopped';
  state.stats.sessions_completed += 1;
  state.stats.last_updated = new Date().toISOString();
}
