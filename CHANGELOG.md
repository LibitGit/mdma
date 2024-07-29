# Changelog
## MDMA 0.1.4-test (2024/07/29)

### Added
- `Auto-X`: early release for testing the windows UI.
- `Custom Alert Message`: smaller and unclickable game messages.
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

## MDMA 0.1.3-test (2024/06/16)

### Added

### Changed

### Fixed

### Internal
- MDMA now works server-side (partially)!

## MDMA 0.1.2-test (2024/06/03)

### Added

### Changed
- `Better Who Is Here`: added displacement logging upon calculation.

### Fixed

### Internal
- Added a graphical interface.

## MDMA 0.1.1-test (2024/05/27)

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