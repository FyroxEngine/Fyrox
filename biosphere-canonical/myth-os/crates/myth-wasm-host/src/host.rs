use std::path::Path;
use wasmtime::{Engine, Instance, Linker, Module, Store};
use myth_plugin::{MythPlugin, PluginResult};
use myth_plugin_registry::PluginRegistry;
use myth_wire::{WirePacket, WireType};

use crate::abi::*;
use crate::error::WasmHostError;
use crate::sandbox::sandboxed_engine;

/// A loaded WASM plugin, wrapped to implement MythPlugin.
/// The host process owns the Wasmtime Store — the plugin runs inside it.
pub struct WasmPluginHost {
    engine:    Engine,
    module:    Module,
    plugin_id: String,
    // Cached metadata read once at load time
    id_str:    String,
    name_str:  String,
    heraldry:  String,
    wire_in:   Vec<WireType>,
    wire_out:  Vec<WireType>,
}

impl WasmPluginHost {
    /// Load a certified plugin from the registry.
    /// Verifies the hash before instantiating — refuses revoked plugins.
    pub fn load(
        registry: &PluginRegistry,
        plugin_id: &str,
    ) -> Result<Self, WasmHostError> {
        let wasm_path = registry
            .wasm_path(plugin_id)
            .ok_or_else(|| WasmHostError::NotCertified(plugin_id.to_string()))?;

        Self::load_from_path(wasm_path, plugin_id)
    }

    /// Load directly from a .wasm file path (used by Plugin Foundry sandbox).
    /// Does NOT require registry certification — Foundry test environment only.
    pub fn load_uncertified(wasm_path: impl AsRef<Path>) -> Result<Self, WasmHostError> {
        Self::load_from_path(wasm_path.as_ref(), "uncertified")
    }

    fn load_from_path(wasm_path: impl AsRef<Path>, plugin_id: &str) -> Result<Self, WasmHostError> {
        let engine = sandboxed_engine()?;
        let module = Module::from_file(&engine, wasm_path)?;

        // Read static metadata with a short-lived store
        let (id_str, name_str, heraldry, wire_in, wire_out) =
            read_metadata(&engine, &module)?;

        Ok(Self {
            engine,
            module,
            plugin_id: plugin_id.to_string(),
            id_str,
            name_str,
            heraldry,
            wire_in,
            wire_out,
        })
    }

    /// Run process() inside a fresh Store with fuel limit.
    /// A fresh Store per call means a hung/panicking plugin can't corrupt host state.
    fn call_process(&self, packet: &WirePacket) -> Result<Vec<WirePacket>, WasmHostError> {
        let mut store: Store<()> = Store::new(&self.engine, ());
        store.set_fuel(PROCESS_FUEL_LIMIT)?;

        let linker: Linker<()> = Linker::new(&self.engine);
        let instance = linker.instantiate(&mut store, &self.module)?;

        // Serialize packet to JSON
        let packet_json = serde_json::to_vec(packet)?;

        // Write into WASM linear memory via myth_alloc
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, EXPORT_ALLOC)?;
        let ptr = alloc.call(&mut store, packet_json.len() as i32)?;

        let memory = instance.get_memory(&mut store, "memory")
            .ok_or_else(|| WasmHostError::MissingExport("memory".into()))?;
        memory.write(&mut store, ptr as usize, &packet_json)?;

        // Call myth_process
        let process = instance.get_typed_func::<(i32, i32), i32>(
            &mut store, EXPORT_PROCESS
        )?;
        let out_ptr = process.call(&mut store, (ptr, packet_json.len() as i32))?;

        // Read output — scan for null terminator to find length
        let mem_data = memory.data(&store);
        let out_bytes = read_cstr(mem_data, out_ptr as usize)?;
        let output: Vec<WirePacket> = serde_json::from_slice(out_bytes)?;

        // Free input allocation
        let free = instance.get_typed_func::<(i32, i32), ()>(&mut store, EXPORT_FREE)?;
        free.call(&mut store, (ptr, packet_json.len() as i32))?;

        Ok(output)
    }
}

impl MythPlugin for WasmPluginHost {
    fn id(&self)              -> &str          { &self.id_str }
    fn name(&self)            -> &str          { &self.name_str }
    fn version(&self)         -> (u32, u32, u32) { (0, 1, 0) }
    fn wire_in(&self)         -> &[WireType]   { &self.wire_in }
    fn wire_out(&self)        -> &[WireType]   { &self.wire_out }
    fn heraldry_symbol(&self) -> &str          { &self.heraldry }

    fn on_attach(&mut self, _vault: std::sync::Arc<myth_vault::VaultRegistry>) -> PluginResult<()> {
        // Vault handle is NOT passed into WASM — plugins access data only via wire packets
        Ok(())
    }

    fn on_detach(&mut self) -> PluginResult<()> { Ok(()) }

    fn process(&mut self, packet: &WirePacket) -> PluginResult<Vec<WirePacket>> {
        self.call_process(packet).map_err(|e| myth_plugin::PluginError::Runtime(e.to_string()))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn read_metadata(
    engine: &Engine,
    module: &Module,
) -> Result<(String, String, String, Vec<WireType>, Vec<WireType>), WasmHostError> {
    let mut store: Store<()> = Store::new(engine, ());
    store.set_fuel(1_000_000)?; // small budget for metadata reads
    let linker: Linker<()> = Linker::new(engine);
    let instance = linker.instantiate(&mut store, module)?;
    let memory   = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| WasmHostError::MissingExport("memory".into()))?;

    let id_str    = read_export_str(&instance, &mut store, &memory, EXPORT_ID)?;
    let name_str  = read_export_str(&instance, &mut store, &memory, EXPORT_NAME)?;
    let heraldry  = read_export_str(&instance, &mut store, &memory, EXPORT_HERALDRY)?;
    let wire_in_j = read_export_str(&instance, &mut store, &memory, EXPORT_WIRE_IN)?;
    let wire_out_j= read_export_str(&instance, &mut store, &memory, EXPORT_WIRE_OUT)?;

    let wire_in:  Vec<WireType> = parse_wire_types(&wire_in_j)?;
    let wire_out: Vec<WireType> = parse_wire_types(&wire_out_j)?;

    Ok((id_str, name_str, heraldry, wire_in, wire_out))
}

fn read_export_str(
    instance: &Instance,
    store: &mut Store<()>,
    memory: &wasmtime::Memory,
    export: &str,
) -> Result<String, WasmHostError> {
    let f = instance.get_typed_func::<(), i32>(store, export)
        .map_err(|_| WasmHostError::MissingExport(export.to_string()))?;
    let ptr = f.call(store, ())?;
    let data = memory.data(store);
    let bytes = read_cstr(data, ptr as usize)?;
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn read_cstr(data: &[u8], ptr: usize) -> Result<&[u8], WasmHostError> {
    let end = data[ptr..].iter().position(|&b| b == 0)
        .ok_or_else(|| WasmHostError::MissingExport("null terminator".into()))?;
    Ok(&data[ptr..ptr + end])
}

fn parse_wire_types(json: &str) -> Result<Vec<WireType>, WasmHostError> {
    let codes: Vec<String> = serde_json::from_str(json)?;
    Ok(codes.iter()
        .filter_map(|c| WireType::from_code(c))
        .collect())
}
