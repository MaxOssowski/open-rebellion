// Pattern 1 — entity tables
pub mod capital_ships;
pub mod defense_facilities;
pub mod fighters;
pub mod major_characters;
pub mod manufacturing_facilities;
pub mod minor_characters;
pub mod production_facilities;
pub mod sectors;
pub mod special_forces;
pub mod systems;
pub mod troops;

// Pattern 1 — new entity tables
pub mod all_facilities;
pub mod entity_table;
pub mod fleets_seed;
pub mod missions;

// Pattern 2 — parameter tables
pub mod general_params;
pub mod side_params;

// Pattern 2 — int lookup tables (shared by all *MSTB and gameplay tables)
pub mod int_table;

// Pattern 2 — system facility seed tables (SYFCCRTB, SYFCRMTB)
pub mod syfc_table;

// Pattern 3 — seed tables (generic, shared by all CMUN*/FACL* files)
pub mod seed_table;

// The GID legends from STRATEGY.DLL (their loader is native only).
pub mod gid_legend;

// Win32 PE resource extraction (native only — uses memory-mapped file I/O)
#[cfg(not(target_arch = "wasm32"))]
pub mod textstra;
#[cfg(not(target_arch = "wasm32"))]
pub mod wave_resources;
