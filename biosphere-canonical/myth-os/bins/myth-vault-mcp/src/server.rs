use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use myth_vault::VaultRegistry;
use tracing::{info, warn, error};

use crate::protocol::{Request, Response, ServerInfo, Capabilities, ToolsCapability};
use crate::tools;

pub fn run(vault: VaultRegistry, vault_root: PathBuf) -> anyhow::Result<()> {
    let stdin  = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    info!("myth-vault-mcp ready — listening on stdin");

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l)  => l,
            Err(e) => { error!("stdin read error: {e}"); break; }
        };
        let line = line.trim().to_string();
        if line.is_empty() { continue; }

        let response = match serde_json::from_str::<Request>(&line) {
            Err(e) => {
                warn!("parse error: {e}");
                Response::err(Value::Null, -32700, format!("parse error: {e}"))
            }
            Ok(req) => handle(&vault, &vault_root, req),
        };

        let mut bytes = serde_json::to_vec(&response)?;
        bytes.push(b'\n');
        out.write_all(&bytes)?;
        out.flush()?;
    }

    info!("stdin closed — shutting down");
    Ok(())
}

fn handle(vault: &VaultRegistry, vault_root: &Path, req: Request) -> Response {
    let id = req.id.clone().unwrap_or(Value::Null);

    match req.method.as_str() {
        "initialize" => {
            info!("client initialized");
            Response::ok(id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": Capabilities {
                    tools: ToolsCapability { list_changed: false },
                },
                "serverInfo": ServerInfo { name: "myth-vault-mcp", version: "0.1.0" },
                "instructions": "Master Vault MCP server for myth-os. \
                    Stores and retrieves 3D models (GLB), textures (PNG/JPG/HDR), \
                    audio (WAV/MP3), shaders (WGSL/GLSL), WASM plugins, and MOLECULEs. \
                    All assets are content-addressed by MythId (blake3 hash). \
                    The vault directory is fully relocatable — move it anywhere, \
                    just update MYTH_VAULT_ROOT. Portals let child vaults reference \
                    master vault assets without copying bytes."
            }))
        }

        "notifications/initialized" | "notifications/cancelled" => {
            // One-way notifications — acknowledge but send no response
            Response::ok(id, Value::Null)
        }

        "tools/list" => {
            Response::ok(id, json!({ "tools": tools::tool_list() }))
        }

        "tools/call" => {
            let params  = req.params.as_ref();
            let name    = params.and_then(|p| p["name"].as_str()).unwrap_or("");
            let args    = params.and_then(|p| p.get("arguments"));
            info!("tool call: {name}");
            let result = tools::dispatch(vault, vault_root, name, args);
            Response::ok(id, result)
        }

        "ping" => Response::ok(id, json!({})),

        other => {
            warn!("unknown method: {other}");
            Response::err(id, -32601, format!("method not found: {other}"))
        }
    }
}
