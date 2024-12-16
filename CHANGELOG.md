# Changelog

## MDMA 0.9.0-test (2024/12/22)

### Added
- Functionality to save addon settings after each change.
- `Popup`: implemented login system via discord account.

### Changed
- `Better Group Invites`: validate the current candidate at every iteration of the invite loop, rather than only at the start of the loop.

### Fixed 

### Internal
- Updated error handling and messaging.
- Updated manager initialization mechanism.

## MDMA 0.8.0-test (2024/11/22)

### Added
- `Signed Custom Teleports`: new/updated mob positions.

### Changed
- Foreground content script is now instantiated through a function instead of a file.

### Fixed 
- Requests no longer interrupt logoff.
- `Better Group Invites`: hotkey checkboxes unchecking.

### Internal
- Added caching for most string slices.
- Peers map updates on every peer join/leave instantaneously.
- Added WASM module compression.
- Emotion removal futures get canceled after a reload task is received.
- Engine properties are now accessed using wasm-bindgen instead of serde, resulting in a significant improvement in overall performance.
- Removed the observe method from Emitter, due to function differences in Rust and Java Script.

## MDMA 0.7.0-test (2024/11/17)

### Added

### Changed
- `Console` Copying logs now provides up to 500 latest margonem server responses.

### Fixed
- `Better Group Invites`: inviting by nick is no longer case sensitive.
- `Accept Group`: updated window layout.
- `Accept Summon`: updated window layout.
- `Accept X`: updated window layout.
- `Better Group Invites`: updated window layout.
- `Better Messages`: updated window layout.
- `Better Who Is Here`: updated window layout.
- `Signed Custom Teleports`: updated window layout.

### Internal
- Refactored addon/setting windows creation methods.

## MDMA 0.6.0-test (2024/11/13)

### Added
- `UI`: in-game chat responsiveness.
- `UI`: signal handling for all components.
- `Better Messages`: added a config window.
- `Accept Summon`: automatically accept designated summon requests.
- `Accept Group`: incoming party invite handling.
- `Better Group Invites`: outgoing party invite handling. 
- `Popup`: for user authentication.

### Changed
- `Auto Group`: split into two addons, `Accept Group` and `Better Group Invites`.
- `Service Worker`: incoming events handling, which previously did not cause the worker to wake up.

### Fixed
- `UI`: window positioning on open from manager and within another window.
- `UI`: addon windows can no longer be moved by dragging any of the decor elements.
- `UI`: tip bounding box is now restricted to the viewport.

### Internal
- Updated error handling and messaging.
- Partially removed multithreading support, shrinking the WASM code size by ~60%. 
- Added (partial) string obfuscation to WASM.
- WASM tries to instantiate before the setting of `communication` and `Engine` modules.
- Peers map no longer retains data from previous location.
- WASM instantiation no longer stops similar game-init blocking scripts from working.
- Created a framework for managing DOM nodes.
- Created a library for WebExtension API bindings.
- Created a framework for communication between extension contexts.

## MDMA 0.5.0-test (2024/07/29)

### Added
- `Auto-X`: early release for testing the windows UI.
- `Better Messages`: configurable game messages.
- `Signed Custom Teleports`: adds a location alias over every custom teleport item.
- Console inside the UI for better bug reporting experience.

### Changed
- `Auto Group`: added a settings window.
- `Auto Group`: implemented a faster algorithm (<1ms) for handling group invites. 
- MDMA widget is now displayed beside player's bags.

### Fixed
- `Better Who Is Here`: emotions update if the server responds with the same emotion before the previous one stopped displaying. 
- `Better Who Is Here`: updated `noemo` handling.

### Internal
- Foreground and middleware scripts stop the game load prior to initialization.
- Every UI HTML element gets rendered inside a shadow DOM.
- Implemented proper communication between the game, and the extension's background script.

## MDMA 0.4.0-test (2024/06/16)

### Added

### Changed

### Fixed

### Internal
- MDMA now works server-side (partially)!

## MDMA 0.3.0-test (2024/06/03)

### Added

### Changed
- `Better Who Is Here`: added displacement logging upon calculation.

### Fixed

### Internal
- Added a graphical interface.

## MDMA 0.2.0-test (2024/05/27)

### Added

### Changed
- `Auto Group`: from now on the addon removes the ask property of parseJSON argument only when a party invite is sent to hero.

### Fixed
- `Better Who Is Here`: emotions now update their position upon removal.
- `Better Who Is Here`: emotions are now displayed for the proper amount of time.
- `Auto Group`: when hero is invited to party by a player from the same location the addon now correctly accepts the invite.

### Internal
- First official test release of MDMA in Rust 🦀

## MDMA 0.1 public test (2023/11/30)

### Added

### Changed

### Fixed

### Internal
- First official test release of MDMA in JS!
