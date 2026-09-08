import { invoke, isTauri as inDesktop } from '@tauri-apps/api/core';
import macbookProfile from '../../configs/macbook-pro-14-3024x1964.json';
export const isTauri = inDesktop();
export type Region = { x:number; y:number; width:number; height:number };
export type Calibration = { frame_width:number; frame_height:number; hotbar_first_slot:number[]; hotbar_slot_stride:number; bite_rgb:number[]; bite_tolerance:number; bite_min_pixels:number; catch_rgb:number[]; catch_tolerance:number; catch_min_pixels:number };
export type BotConfig = {
  calibration:Calibration|null; region_preset:string; red_region:Region; yellow_region:Region; hunger_region:Region;
  rod_slot:number; food_slot:number; auto_feed_enabled:boolean; energy_capacity_feed_below:number;
  observation_max_age_ms:number; recovery_limit:number; fish_per_feed:number;
  startup_delay_ms:number; detection_interval_ms:number; autoclick_interval_ms:number; max_fishing_timeout_ms:number;
  rod_lure_value:number; always_on_top:boolean; color_tolerance:number;
  // Preserved for old settings files; the current UI does not present unimplemented features.
  feed_below_percent:number; webhook_url:string; screenshot_interval_mins:number; screenshot_enabled:boolean;
  auto_save_enabled:boolean; failsafe_enabled:boolean; advanced_detection:boolean;
};
export type ResolutionPreset = { red_region:Region; yellow_region:Region; hunger_region:Region; settings:Partial<BotConfig>|null };
export type EnergyReading = { usable:number; capacity:number };
export type Observation = { sequence:number; captured_at_ms:number; bite:boolean; bite_checked:boolean; caught:boolean; rod_selected:boolean; food_selected:boolean; energy:EnergyReading|null; error:string|null };
export type Controller = { phase:string; reason:string; fish_caught:number; feeds:number; errors:number; recovery_attempts:number; energy:EnergyReading|null };
export type SessionState = { running:boolean; controller:Controller; elapsed_ms:number; observation:Observation|null };
export type LifetimeStats = { total_fish_caught:number; total_runtime_seconds:number; sessions_completed:number; last_updated:string; best_session_fish:number; average_fish_per_hour:number; total_feeds:number; uptime_percentage:number };
export type Snapshot = { stats:LifetimeStats; session:SessionState };
export type Preview = { observation:Observation; elapsed_ms:number; regions:{name:string;data_url:string}[] };
const defaults:BotConfig = {
  calibration:null,region_preset:'3440x1440',red_region:{x:1321,y:99,width:768,height:546},yellow_region:{x:3097,y:1234,width:342,height:205},hunger_region:{x:274,y:1301,width:43,height:36},
  rod_slot:2,food_slot:1,auto_feed_enabled:false,energy_capacity_feed_below:0,observation_max_age_ms:1000,recovery_limit:2,
  fish_per_feed:5,startup_delay_ms:5000,detection_interval_ms:50,autoclick_interval_ms:70,max_fishing_timeout_ms:25000,rod_lure_value:1,always_on_top:false,color_tolerance:10,
  feed_below_percent:50,webhook_url:'',screenshot_interval_mins:60,screenshot_enabled:false,auto_save_enabled:false,failsafe_enabled:false,advanced_detection:false,
};
let previewConfig:BotConfig = {...defaults,...macbookProfile};
const empty:Snapshot = {
  stats:{total_fish_caught:0,total_runtime_seconds:0,sessions_completed:0,last_updated:'',best_session_fish:0,average_fish_per_hour:0,total_feeds:0,uptime_percentage:0},
  session:{running:false,elapsed_ms:0,observation:null,controller:{phase:'stopped',reason:'Idle',fish_caught:0,feeds:0,errors:0,recovery_attempts:0,energy:null}},
};
export async function getConfig():Promise<BotConfig> {return isTauri ? invoke('get_config') : structuredClone(previewConfig);}
export async function getStats():Promise<Snapshot> {return isTauri ? invoke('get_stats') : structuredClone(empty);}
export async function saveConfig(config:BotConfig):Promise<void> {if(isTauri) await invoke('save_config',{config});else previewConfig=structuredClone(config);}
export async function getResolutionPresets():Promise<Record<string,ResolutionPreset>> {
  if(isTauri) return invoke('get_resolution_presets');
  return {[macbookProfile.region_preset]:{red_region:macbookProfile.red_region,yellow_region:macbookProfile.yellow_region,hunger_region:macbookProfile.hunger_region,settings:macbookProfile}};
}
export async function startSession():Promise<void> {if(!isTauri) throw new Error('Open the desktop app to run automation');await invoke('start_session');}
export async function stopSession():Promise<void> {if(isTauri) await invoke('stop_session');}
export async function inspectScreenshot(config:BotConfig,imageBase64:string):Promise<Preview> {return invoke('inspect_screenshot',{config,imageBase64});}
export async function capturePreview(config:BotConfig):Promise<Preview> {return invoke('capture_preview',{config});}
export function biteTimeout(lure:number):number {return Math.round(Math.min(180,Math.max(10,(lure<=1?3-2*lure:1.25-lure/3)*60+5))*1000);}
