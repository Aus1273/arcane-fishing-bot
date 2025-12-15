/// <reference types="svelte" />
/// <reference types="vite/client" />

import type { BotApi } from './lib/ipc';

declare global {
  interface Window {
    bot?: BotApi;
  }
}
