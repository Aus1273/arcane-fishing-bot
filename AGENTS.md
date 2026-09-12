# Project instructions

The user superseded the old Tauri v1 restriction on 2026-09-08.

- Active stack: Tauri 2, Rust, Svelte, TypeScript and Tailwind.
- Use frontendDist/devUrl and @tauri-apps/api/core.
- Keep the deterministic controller independent of native input and UI.
- Preserve saved configuration compatibility and offline replay coverage.
- Keep UI functional and explicit about unverified live behavior.
- Keep this checkout focused on the production app. Historical implementations and experiments are recoverable from Git history.
- Frontend feature code and styles live together under src/features; src/lib contains shared UI and the IPC boundary.
- Native app lifecycle belongs in main.rs; frontend command adapters belong in commands.rs.
