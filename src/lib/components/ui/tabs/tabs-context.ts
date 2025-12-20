import type { Writable } from 'svelte/store';

export const TABS_CONTEXT = Symbol('tabs');

export type TabsContext = {
  value: Writable<string>;
  setValue: (value: string) => void;
};
