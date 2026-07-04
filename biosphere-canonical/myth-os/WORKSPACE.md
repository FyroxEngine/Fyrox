# myth-os Workspace

Rust workspace for the BioSpark Studios Quantum Ecosystem.

## Quick Start

```powershell
# Run the Library (vault browser + workspace)
cargo run -p library

# Run the Genesis engine (rack UI + world simulation)
cargo run -p genesis

# Run the Core supervisor (async orchestrator)
cargo run -p core-supervisor

# Launch Library + Genesis in separate windows
.\dev.ps1

# Smoke-test without opening a window
.\dev.ps1 dry-run

# Build all crates (no run)
cargo build --workspace
```

## Crate Map

| Directory | Package name | Binary | Role |
|-----------|-------------|--------|------|
| `library/` | `library` | `library.exe` | The Great Library — vault browser, workspace UI |
| `genesis/` | `genesis` | `genesis.exe` | Genesis Engine — rack UI, world simulation, Traktor S4 MIDI |
| `core/` | `core-supervisor` | `core.exe` | Core Supervisor — async orchestrator, bus router, heartbeat |
| `vault/` | `vault` | *(library)* | Vault atom — blob storage, mmap, persistence primitives |
| `mythos/` | `mythos` | *(library)* | Mythos types — MythId, SignalKind, QuantumModule definitions |
| `asset-forge/` | `asset-forge` | `qforge.exe` | qforge CLI — token-based asset prompt and manifest generator |

## Build Target

All build artifacts go to `C:\cargo-target\myth-os` (set in `.cargo/config.toml`).
Never committed. Do not point cargo at the workspace root for output.

## Key Paths

| Path | Contents |
|------|----------|
| `data/library/vaults.json` | Persisted vault store (auto-created on first run) |
| `data/core/log` | Core supervisor audit log |
| `assets/modules/` | Genesis module definitions (JSON) |
| `asset-forge/examples/` | Example `.toml` configs for qforge |

## Hardware

| Device | Connection | Crate | Notes |
|--------|-----------|-------|-------|
| Traktor S4 MK3 | USB MIDI | `genesis` | Channel faders 1–4 → Structure/Entities/Atmosphere/Dynamics bus. Absent = warn, not crash. |
| DMX controller | Serial (Arduino) | `genesis` (planned) | `serialport` crate wired, not yet active |

## App States (Library)

```
Splash → Landing → VaultView
                 → VaultSetup → Landing
       → Error   (any unrecoverable transition — ↩ returns to Landing)
```

## System Ordering (Library egui)

```
UiSet::Shell  (shell.rs — outer Mythos ring + inner Vault ring panels)
UiSet::Page   (landing / vault_view / vault_setup / error_screen — CentralPanel content)
```

**Rule:** Shell panels must be flushed to egui before Page content is added.
Never reverse this order or panels will flicker / overlap.

## Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `failed to remove library.exe` | Library window still open | Close the window, then `cargo run -p library` |
| `Res<T> conflicts with ResMut<T>` | Same resource requested twice in one system | Use only `ResMut<T>` — it derefs to `&T` for reads |
| `package ID not found for 'core'` | Cargo reserves the name `core` | Use `cargo run -p core-supervisor` |
| `No MIDI input ports found` | Traktor S4 not connected | Expected — logs a warning and continues |

## Conventions

- **Wire types:** DATA / CONTROL / AUDIO / NARRATIVE / TEMPORAL / AGENT / VISUAL / SPATIAL / BEHAVIORAL / SOCIAL / ENERGY / IDENTITY / EVENT / ASSET / META / LOGIC / RESONANCE
- **Bus channels:** Structure (Ch1) / Entities (Ch2) / Atmosphere (Ch3) / Dynamics (Ch4)
- **Octave Capacity Law:** 2^n children per container level. Default: Octave 4 = 16 children.
- **Color32 serde:** Store as `[u8; 4]` — egui's `Color32` has no serde without the egui feature flag.
- **B-DNA:** 64-position boolean lineage hash. Every Capsule requires one before seal.

## Environment Variables

```powershell
$env:RUST_LOG = "info,library=debug,genesis=debug,bevy_render=warn,wgpu=error,egui=warn"
```

Set this before `cargo run` for clean, readable log output. The `dev.ps1` script sets it automatically.
