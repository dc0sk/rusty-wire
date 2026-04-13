# Rusty Wire Copilot Memory Export

This file is a documentation snapshot of the current Copilot memory state for this repository.
Keep it synchronized with the contents of `/memories` and `/memories/repo` before pushing changes.

Export date: 2026-04-13

## User memory

No user memory entries are currently stored.

## Session memory

No session memory entries are currently stored.

## Repository memory

### Source: `/memories/repo/itu-regions-implementation.md`

# ITU Regions Implementation

## Changes Made
Added ITU region selection feature to rusty-wire band calculator.

### Files Modified:
1. **bands.rs** - Added ITU region support
   - Added `ITURegion` enum with Region1/2/3
   - Added `regions` field to Band struct
   - All bands marked for ALL_REGIONS
   - Functions: `get_bands_for_region()`, `get_band_by_index_for_region()`

2. **app.rs** - Integrated region into app config
   - Added `DEFAULT_ITU_REGION = Region1`
   - Added `itu_region` field to AppConfig
   - Updated `build_calculations()` to use region-aware band lookup

3. **cli.rs** - Added CLI and interactive region support
   - Added `--region 1|2|3` CLI flag
   - Added `prompt_itu_region()` function for interactive menu
   - Updated main menu: option 4 to change region, 5 to exit (was 4)
   - Functions: `parse_itu_region()`, region display in band listings
   - Updated help/usage message with region documentation

## Features:
- **CLI**: `--region 1|2|3` flag (defaults to 1)
- **Interactive**: Region selection at startup + "Change ITU Region" menu option
- **Defaults**: Region 1 (Europe, Africa, Middle East)
- **Region Info**: Shows full region names in menu and band listings

### Source: `/memories/repo/workflow-sync.md`

# Workflow Sync

- Before pushing, ensure documentation is up to date, docs/copilot-memories.md is synchronized with /memories and /memories/repo, cargo fmt and cargo test have run, the additional regression scripts have run, and all necessary files are staged.