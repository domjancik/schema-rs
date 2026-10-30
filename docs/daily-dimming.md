# Daily dimming and time simulation

Output → Dimming shows the engine's current time in the schedule's configured UTC offset, the actual output multiplier, and a solid current-time marker on the daily curve. The final curve segment crosses midnight back to the first point. The master multiplies the curve; fixture dimmers still multiply independently and program/control DMX slots stay unchanged.

Use the time field or scrubber to preview another time. Play/Pause and 1×, 60× or 3600× speed allow testing the full day, including midnight. The dashed marker and Preview percentage follow the preview; ordinary scrubbing/playback does not change output. Unsaved curve edits affect the chart preview; Save dimming timeline commits those edits with normal undo.

**Apply simulated time to output** explicitly applies the simulated schedule time to the engine's saved daily curve. The output percentage and global Time sim indicator make this visible to other views. **Use live time**, the global Time sim → Live button, closing the dimming view, or changing tabs returns output to live time. Simulation also expires after 15 seconds without its view's heartbeat. An older view's cleanup cannot cancel a newer view's simulation; an explicit Use live time clears it globally.

Simulation controls only the daily dimming clock. It does not change the computer/RTC clock, graph animation time, DateTime inputs, projects, undo history or checkpoints. It is transient and is not resumed after restart. On a browser engine it affects computed output; native hosts apply it to their configured output path.

The interface uses existing scene publications for clock/status; there is no additional frame polling. Simulation settings are coalesced and serialized, and keepalives run every three seconds while active. Native and WASM hosts use the same monotonic, leased simulation clock. One dimming multiplier is computed for the whole evaluation pass.

`GET /api/dimming/clock` returns live UTC seconds, effective UTC seconds, simulation state/speed/owner, remaining lease, output level and the saved schedule offset. `POST` accepts one of:

```json
{"action":"simulate","owner":"unique-view-id","seconds":43200,"speed":60}
{"action":"renew","owner":"unique-view-id"}
{"action":"live","owner":"unique-view-id"}
{"action":"live"}
```

Seconds are UTC within a day; the UI converts from the configured schedule time. Speed is 0 for a fixed time, through 3600 for accelerated playback. Owner IDs are view coordination identifiers, not access credentials; the usual host authentication applies. Clock control runs on the engine thread without preparing an authored edit or incrementing the revision. The top-level `dimming_clock` publication field is transient telemetry, separate from the serialized session snapshot.

Tests cover real encoded dimming levels, timezone conversion, midnight wrap, ownership and expiry, native HTTP and actual WASM/UI operation, live return/close cleanup, narrow phone layout and unchanged project/history state.
