# AI AGENT INSTRUCTIONS

## Project Overview
This is a Rust-powered desktop automation tool for the game "Arcane Odyssey". It uses Tauri (v1) for the desktop shell and Svelte/Tailwind for the UI.

## 🛑 STRICT RULES (DO NOT IGNORE)
1. **NO ELECTRON:** This project uses TAURI. Do not suggest or import Electron packages.
2. **NO TAURI V2:** This project uses Tauri v1 (stable). Do not use `frontendDist`, use `distDir`.
3. **NO FANTASY UI:** The design language is strictly "Industrial / Sci-Fi / Black Mesa".
   - NO rounded corners (`rounded-none`).
   - NO gradients or "glassmorphism".
   - Colors: `#ff9a00` (Orange) and `#1a1a1a` (Dark Grey).

## Technical Stack
- **Backend:** Rust (Tauri commands)
- **Frontend:** Svelte + TypeScript + Tailwind CSS
- **Communication:** Use `@tauri-apps/api/tauri` and `invoke` to call Rust.

## User Persona
The user ("Aus1273") prefers direct, technical solutions. Do not use "flavor text" (e.g., "summoning runes") in the code or UI. Keep it functional.
