# Loop and Functionality Review

This page summarizes the current control loops driving the Arcane Fishing Bot and suggests improvements or pruning opportunities.

## Observations
- The main run loop initializes the rod, optionally captures a startup screenshot, and then iterates while the bot is marked as running, pausing when the session is flagged as paused and sleeping 50 ms between cycles. Error streaks beyond five consecutive failures break the loop for safety. 【F:src/main.rs†L1311-L1391】
- Each fishing attempt follows a three-phase routine: cast the rod, wait for a bite until a lure-dependent timeout, and reel when red/yellow markers are detected; detection cadence is set by `detection_interval_ms`. 【F:src/main.rs†L1394-L1450】
- Performance metrics (success rate, average operation time, error count) are recorded for each loop iteration but are not surfaced to the UI, limiting operator feedback. 【F:src/main.rs†L1129-L1158】【F:src/main.rs†L1375-L1378】
- OCR handlers are constructed twice during bot creation—once for the primary instance and again inside the worker thread—duplicating setup cost. 【F:src/main.rs†L1179-L1235】

## Suggestions for Additions
- Expose the `PerformanceMonitor` metrics to the front-end (e.g., via IPC) so operators can track real-time success rates and latency trends without inspecting logs. 【F:src/main.rs†L1129-L1158】【F:src/main.rs†L1375-L1378】
- Make the consecutive-error threshold and cycle sleep duration configurable to allow tuning for different latency or stability conditions instead of relying on hardcoded values (currently five errors and 50 ms sleeps). 【F:src/main.rs†L1342-L1388】
- Provide visual cues in the UI for the lure-derived bite timeout to help users align in-game gear with detector expectations (using the existing `calculate_max_bite_time` logic). 【F:src/main.rs†L74-L98】【F:src/main.rs†L1424-L1436】

## Suggestions for Pruning or Cleanup
- Consolidate OCR handler construction so the worker thread reuses the instance created at startup, avoiding double initialization and reducing resource usage. 【F:src/main.rs†L1179-L1235】
- Evaluate whether the fixed 50 ms idle sleep in the run loop is necessary once detection and timing are configurable; trimming redundant sleeps can shorten recovery between casts. 【F:src/main.rs†L1383-L1388】
