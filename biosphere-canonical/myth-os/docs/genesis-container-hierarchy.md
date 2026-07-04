# Genesis Container Hierarchy — Prime Seals
## BioSpheres OS Architecture

*Authored by Phantori — saved 2026-06-13*

---

## The Four Prime Seals

```
BioSpheres-OS v0.0.1          (Prime Seal 1 — The Operating System)
    └── Master Vault Protocol  (Prime Seal 2 — The Space in the Void)
            └── Core Engine    (Prime Seal 3 — The Headless Server)
                    └── Genesis Protocol (Prime Seal 4 — The Universe Creator)
```

Each layer is a Genesis Container (Prime Seal). The OS does not contain the
Vaults — it *interacts* with the Vault Genesis Container. Clean separation.

---

## Prime Seal 1 — BioSpheres-OS v0.0.1 (The Operating System)

**Purpose:** Foundational layer. Manages system resources, orchestrates loading/
unloading of the other major Genesis Containers.

**Key Responsibilities:** System startup, resource allocation, security,
inter-container communication channels.

### Prime Crest 1.1: System Boot & Orchestration

**Prime Glyph 1.1.1: OS Bootstrap**
- `System_Loader_ATOM` — Initiates the OS and its core services
- `Container_Manager_ATOM` — Discovers, validates, prepares other Genesis Containers
- `Inter_Container_Bus_ATOM` — Establishes communication channels between OS/Vault/Core/Genesis

**Prime Glyph 1.1.2: Resource Arbitration**
- `Global_Memory_ATOM` — Manages system-wide memory
- `CPU_Scheduler_ATOM` — Arbitrates CPU time across all running containers
- `Graphics_Context_ATOM` — Provides shared graphics context for rendering

---

## Prime Seal 2 — Master Vault Protocol (The Space in the Void)

**Purpose:** Establishes and manages the "space in the void" — the isolated
dimension where multiverses reside. Loaded by the OS.

**Key Responsibilities:** Managing concurrent vault instances (up to 16 on server),
providing persistence layer, interfacing with Core container.

### Prime Crest 2.1: Vault Lifecycle Manager

**Prime Glyph 2.1.1: Vault Instantiation**
- `Vault_Spawner_ATOM` — Creates and isolates new "vault spaces" (dimensions)
- `Vault_Authenticator_ATOM` — Manages access and security per vault
- `Vault_Resource_Allocator_ATOM` — Assigns OS resources to individual vaults

**Prime Glyph 2.1.2: Concurrent Vault Pool**
- `Active_Vault_Pool_ATOM` (up to 16 instances) — Manages concurrent vault pool
- `Vault_Persistence_ATOM` — Handles save/load of each vault's environment state
- `Vault_Telemetry_ATOM` — Monitors health and activity of each vault

### Prime Crest 2.2: Core Container Interface

**Prime Glyph 2.2.1: Core Handshake Protocol**
- `Core_Loader_ATOM` — Initiates and connects to Core Genesis Container per vault
- `Vault_Core_Channel_ATOM` — Dedicated communication between vault instance and Core

**Prime Glyph 2.2.2: Multiverse Access Proxy**
- `Client_Connection_Router_ATOM` — Routes incoming client connections to correct vault Core

---

## Prime Seal 3 — Core Engine (The Headless Server)

**Purpose:** Headless server managing active simulations within a vault.
Receives instructions from Vault Protocol, uses Genesis Protocol to generate worlds.

**Key Responsibilities:** Running simulations, managing game logic, client interaction.

### Prime Crest 3.1: Simulation Runtime

**Prime Glyph 3.1.1: Simulation Loop**
- `Game_Tick_ATOM` — **THE CLOCK** — manages server-side simulation loop
- `Physics_Processor_ATOM` — Executes narrative physics engine for all active worlds
- `AI_Director_ATOM` — Manages non-player entities and world events
- `World_State_Sync_ATOM` — Keeps client world states synchronized

**Prime Glyph 3.1.2: Multiverse Management**
- `Multiverse_Registry_ATOM` — Tracks 16 active multiverse worlds per Core instance
- `World_Load_Balancer_ATOM` — Distributes simulation load across resources

### Prime Crest 3.2: Genesis Protocol Interface

**Prime Glyph 3.2.1: World Generation Request**
- `Genesis_Requester_ATOM` — Sends requests to Genesis Protocol for new worlds
- `World_Seed_Manager_ATOM` — Manages unique seeds per generated world

**Prime Glyph 3.2.2: World Blueprint Importer**
- `Blueprint_Interpreter_ATOM` — Interprets Genesis blueprints into active world data

### Prime Crest 3.3: Client Interaction (Server-side)

**Prime Glyph 3.3.1: Client Connection Handler**
- `Client_Connect_ATOM` — Manages new client connections to specific multiverses
- `Client_Disconnect_ATOM` — Handles client disconnections
- `Input_Interpreter_ATOM` — Translates client influence signals into simulation params

**Prime Glyph 3.3.2: World State Broadcaster**
- `State_Replicator_ATOM` — Sends world state updates to connected clients
- `Delta_Encoder_ATOM` — Optimizes network traffic (send only changes)

---

## Prime Seal 4 — Genesis Protocol (The Universe Creator)

**Purpose:** Dedicated system for generating unique multiverses per vault,
based on narrative physics and procedural rules.

**Key Responsibilities:** Procedural generation of landscapes, weather, lore, initial conditions.

### Prime Crest 4.1: World Generation Pipeline

**Prime Glyph 4.1.1: Terrain & Biome Generation**
- `Noise_Generator_ATOM` — Noise patterns for terrain heightmaps, temperature, rainfall
- `Biome_Distributor_ATOM` — Places biomes based on environmental factors
- `Voxel_Engine_ATOM` — Generates underlying volumetric world data

**Prime Glyph 4.1.2: Weather & Atmospheric Systems**
- `Atmosphere_Model_ATOM` — Simulates atmospheric conditions (wind, clouds, pressure)
- `Weather_Pattern_ATOM` — Generates dynamic weather systems (rain, snow, storms)

**Prime Glyph 4.1.3: Narrative Physics Seed**
- `Physics_Rule_Injector_ATOM` — Embeds narrative physics rules for this universe
- `Initial_Condition_ATOM` — Sets starting state of the physics simulation

### Prime Crest 4.2: Content & Lore Generation

**Prime Glyph 4.2.1: Flora & Fauna Spawner**
- `Vegetation_Placer_ATOM` — Populates world with plants
- `Creature_Spawner_ATOM` — Generates and places initial wildlife

**Prime Glyph 4.2.2: Lore & Event Generator**
- `Historical_Event_ATOM` — Generates unique "history" for the world
- `Geological_Event_ATOM` — Simulates geological processes (volcanoes, earthquakes)

### Prime Crest 4.3: Blueprint Export

**Prime Glyph 4.3.1: World Data Serializer**
- `Blueprint_Exporter_ATOM` — Packages generated world data for Core consumption
- `Seed_Recorder_ATOM` — Logs seed and parameters used for generation

---

## Client-Side Mirror Structure

The client mirrors the server but flips from **creating** to **connecting**.

### Client-Side Vault Protocol (The Interface to a Multiverse)

**Prime Crest 2.1 (Client): Vault Connection Manager**
- `Vault_Connector_ATOM` — Initiates/maintains connections to server Vaults/Cores
- `World_Synchronizer_ATOM` — Receives and processes world state updates from server

**Prime Crest 2.2 (Client): MIDI & Influence Engine**
- `MIDI_Input_ATOM` — Reads data from MIDI controllers (synths, drum machines, DJs, S4)
- `Influence_Mapper_ATOM` — Translates MIDI into abstract "influence signals" for server
- `Feedback_Render_ATOM` — Visual/auditory feedback on influence being exerted

**Prime Crest 2.3 (Client): Local Rendering & Visualization**
- `Client_Renderer_ATOM` — Renders received world state locally
- `Physics_Visualizer_ATOM` — Displays narrative physics in real-time
- `UI_Overlay_ATOM` — Renders the client's control interface

---

## Rust Crate Mapping

| Prime Seal / ATOM | Rust Crate |
|-------------------|------------|
| BioSpheres-OS | `myth-core` (expanding) |
| `Inter_Container_Bus_ATOM` | `services/myth-bus` (planned) |
| `CPU_Scheduler_ATOM` / `Game_Tick_ATOM` | `services/myth-clock` (building now) |
| Master Vault Protocol | `myth-vault` (expanding) |
| `Vault_Persistence_ATOM` | `myth-vault` storage layer |
| Core Engine | `myth-core` server mode |
| `Delta_Encoder_ATOM` | `myth-wire` (WirePacket encoding) |
| Genesis Protocol | `myth-quill` + modules 01-16 |
| `Voxel_Engine_ATOM` | Module 10 (planned) |
| `MIDI_Input_ATOM` | `myth-controller` |
| `Client_Renderer_ATOM` | `biospark-theatre` + Bevy adapter |
| `Blueprint_Exporter_ATOM` | `myth-stencil` (`.stencil` files) |

---

## The Law of 16

Appears at every layer:
- Up to 16 concurrent vault instances per server
- 16 active multiverse worlds per Core instance  
- 16 modules (OctaveEnforcer)
- 16 channels in the narrative mixer

This is not coincidence. 16 is the structural constant of the system.
