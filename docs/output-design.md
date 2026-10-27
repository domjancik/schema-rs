# Output design

Open **Output** in Studio or schémaboy. Settings remain engine-owned authored state; accepting a timeline/model/patch transaction adds one undo entry. The current preview is capture-only. No physical UV fixture behavior is claimed from software tests.

## Daily global dimming

A master 0–1 gain multiplies final light intensity after scene evaluation, fixture dimming and color alpha. Enabling the daily curve adds cyclic, linear time points. Last-to-first interpolation crosses midnight without requiring duplicate 00:00 and 24:00 points; for example 18:00 at 100% and 06:00 at 20% yields 60% at midnight. The curve repeats every 24 hours; one point holds a constant level. Distinct time points, levels and master are validated before commit. Drag points or edit their HH:MM/percentage rows, then Save. Closing the panel discards unsaved edits.

Times use UTC clock input plus the saved UTC offset in minutes. **Use device time zone** copies the current browser offset. This is an explicit fixed offset, not an IANA time-zone/DST calendar; update the offset when daylight-saving rules change. The host clock adapter publishes once per second, independently of UI rendering and animation pause. Disabling the curve preserves the global master gain. Program constants and movement controls are not dimmed. Blackout still disables intensity channels.

## Extensible native slot models

Models are project-scoped named definitions with ordered visual DMX slot cards. The included `uv-cob` has two slots: Value/Dimmer and Constant 0/Program. Assign it through the fixture Model selector or Universe patch tab. Every fixture using it occupies two channels per pixel, even if slot 2 is always zero. A fixture at address 1 therefore occupies 1–2; the next must begin at 3 or later.

Add/edit models through **DMX models**. Every slot chooses Value, Red, Green, Blue, White, Pan, Tilt, Speed, Duration, named Channel or Constant. Nonconstant sources are normalized 0–1 and mapped to minimum/maximum bytes, including reversed ranges. Value/RGB/White apply master and fixture intensity gain. Constants are raw bytes and are preserved under dimming/blackout; use Constant 0 for program/strobe-off channels when specified by the device manual. Fine byte extracts the low byte of the normalized 16-bit value; use a matching coarse slot next to it where the fixture requires coarse/fine pairs. Pan/Tilt use rotation X/Y and normalized values. Named Channel reads a numeric bank channel. Models repeat the whole slot sequence for each fixture pixel.

This design takes ordered constants/channel sources from [Expression DMX Patching](https://docs.scenic.tools/plugins/artnet/expression-dmx-patching), while using bounded native controls rather than evaluating arbitrary mapping expressions. General recursive expression graphs, fixture-manufacturer libraries, RDM discovery and automatic address configuration on physical devices are not included.

Models contain 1–512 slots; all fixture footprints must remain inside 512 channels. Unknown models, colliding addresses, invalid slot bytes and expansion that overlaps another fixture are rejected atomically. Models in use cannot be deleted. Changing a shared model updates its instances together if the resulting patch validates. Built-in Dimmer/RGB/RGBW remain compatible.

## Universe usage and reindexing

**Universe patch** shows 512 channels, occupied ranges and all fixture cards with universe/address/footprint. Drag a fixture's title onto a free cell to use that address. Drop onto an occupied fixture to insert before it and repack the destination universe from address 1 in existing address order. Repacking is a single Batch, so intermediate overlaps never publish; Undo restores all addresses together. The source universe can retain a gap. Numeric address + Patch offers an alternative to dragging. Overflow or any overlap is rejected. Repatching software does not change physical DIP switches or fixture menus.

Model definitions and dimming are stored with the saved layout; content-only saves leave them unchanged. Both are serialized in sessions and undo/redo projects. Old layouts without these fields use the stock model set and disabled daily curve with master 1.
