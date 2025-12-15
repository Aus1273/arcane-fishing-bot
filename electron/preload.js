import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('bot', {
  getState: () => ipcRenderer.invoke('get-state'),
  getConfig: () => ipcRenderer.invoke('get-config'),
  getStats: () => ipcRenderer.invoke('get-stats'),
  saveConfig: (config) => ipcRenderer.invoke('save-config', config),
  startSession: () => ipcRenderer.invoke('start-session'),
  stopSession: () => ipcRenderer.invoke('stop-session'),
});
