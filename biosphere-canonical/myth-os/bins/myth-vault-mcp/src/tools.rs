//! MCP tool implementations — vault operations exposed to clients.

use serde_json::{json, Value};
use myth_vault::VaultRegistry;
use myth_wire::MythId;
use std::path::{Path, PathBuf};
use crate::asset::{AssetMeta, AssetType};
use crate::protocol::ToolResult;

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tool_list() -> Vec<crate::protocol::Tool> {
    vec![
        crate::protocol::Tool {
            name: "vault_ingest",
            description: "Store an asset in the Master Vault. Accepts base64-encoded bytes. \
                          Returns the MythId (content hash). Safe to call twice — dedup is automatic. \
                          Move the vault directory freely; only MYTH_VAULT_ROOT needs updating.",
            input_schema: json!({
                "type": "object",
                "required": ["filename", "data_base64", "name"],
                "properties": {
                    "filename":    { "type": "string", "description": "Original filename — used to infer asset type (e.g. model.glb, albedo.png, theme.wgsl)" },
                    "data_base64": { "type": "string", "description": "Base64-encoded file bytes" },
                    "name":        { "type": "string", "description": "Human-readable asset name" },
                    "description": { "type": "string", "description": "What this asset is and where it's used" },
                    "author":      { "type": "string", "description": "Creator name" },
                    "tags":        { "type": "array", "items": { "type": "string" },
                                     "description": "Searchable tags e.g. [\"biome:tropical\", \"lod:high\", \"module:atlas\"]" }
                }
            }),
        },
        crate::protocol::Tool {
            name: "vault_ingest_path",
            description: "Store an asset by file path — the server reads it directly. \
                          Preferred over vault_ingest for large files (GLB, PNG, audio). \
                          Can also ingest an entire directory of files at once.",
            input_schema: json!({
                "type": "object",
                "required": ["path"],
                "properties": {
                    "path":        { "type": "string", "description": "Absolute file path OR directory path to ingest all supported files from" },
                    "name":        { "type": "string", "description": "Human-readable name (omit for directory — uses filename)" },
                    "description": { "type": "string", "description": "What this asset is" },
                    "author":      { "type": "string", "description": "Creator name" },
                    "tags":        { "type": "array", "items": { "type": "string" }, "description": "Searchable tags" },
                    "extensions":  { "type": "array", "items": { "type": "string" },
                                     "description": "For directory ingests: which extensions to include e.g. [\".glb\",\".png\"]. Omit for all supported types." }
                }
            }),
        },
        crate::protocol::Tool {
            name: "vault_list",
            description: "List assets in the Master Vault, optionally filtered by type or tag.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "asset_type": { "type": "string",
                                    "description": "model_3d | texture | audio | shader | wasm_plugin | molecule | capsule | raw" },
                    "tag":   { "type": "string",  "description": "Filter by tag (exact match)" },
                    "limit": { "type": "integer", "description": "Max results (default 50)" }
                }
            }),
        },
        crate::protocol::Tool {
            name: "vault_fetch",
            description: "Retrieve an asset by MythId. Returns base64 bytes and metadata.",
            input_schema: json!({
                "type": "object",
                "required": ["myth_id"],
                "properties": {
                    "myth_id": { "type": "string" }
                }
            }),
        },
        crate::protocol::Tool {
            name: "vault_search",
            description: "Search asset metadata by name, description, or tag (case-insensitive substring).",
            input_schema: json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query":      { "type": "string" },
                    "asset_type": { "type": "string", "description": "Optional type filter" }
                }
            }),
        },
        crate::protocol::Tool {
            name: "vault_info",
            description: "Returns vault profile, total asset count, and breakdown by type.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
    ]
}

// ─── Dispatch ────────────────────────────────────────────────────────────────

pub fn dispatch(
    vault:      &VaultRegistry,
    vault_root: &Path,
    tool_name:  &str,
    params:     Option<&Value>,
) -> ToolResult {
    let p = params.cloned().unwrap_or(Value::Object(Default::default()));
    match tool_name {
        "vault_ingest"       => ingest(vault, vault_root, &p),
        "vault_ingest_path"  => ingest_path(vault, vault_root, &p),
        "vault_list"    => list(vault_root, &p),
        "vault_fetch"   => fetch(vault, vault_root, &p),
        "vault_search"  => search(vault_root, &p),
        "vault_info"    => info(vault, vault_root),
        _               => ToolResult::error(format!("unknown tool: {tool_name}")),
    }
}

// ─── vault_ingest_path ───────────────────────────────────────────────────────

fn ingest_path(vault: &VaultRegistry, vault_root: &Path, p: &Value) -> ToolResult {
    let path_str = str_field(p, "path");
    if path_str.is_empty() {
        return ToolResult::error("path is required");
    }

    let path = std::path::Path::new(&path_str);
    if !path.exists() {
        return ToolResult::error(format!("path does not exist: {path_str}"));
    }

    let author      = p["author"].as_str().unwrap_or("unknown").to_string();
    let description = p["description"].as_str().unwrap_or("").to_string();
    let tags: Vec<String> = p["tags"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if path.is_file() {
        // Single file
        ingest_one_file(vault, vault_root, path, &p["name"].as_str().unwrap_or("").to_string(), &description, &author, &tags)
    } else if path.is_dir() {
        // Directory — ingest all supported files
        let extensions: Vec<String> = p["extensions"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(|s| s.to_lowercase())).collect())
            .unwrap_or_else(|| vec![
                ".glb".into(), ".gltf".into(),
                ".png".into(), ".jpg".into(), ".jpeg".into(), ".hdr".into(), ".exr".into(), ".webp".into(),
                ".wav".into(), ".mp3".into(), ".flac".into(), ".ogg".into(),
                ".glsl".into(), ".wgsl".into(), ".vert".into(), ".frag".into(),
                ".wasm".into(),
            ]);

        let entries: Vec<_> = std::fs::read_dir(path)
            .map(|rd| rd.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();

        let mut results = Vec::new();
        let mut errors  = Vec::new();

        for entry in entries {
            let fp = entry.path();
            if !fp.is_file() { continue; }
            let ext = fp.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();
            if !extensions.contains(&ext) { continue; }

            let fname = fp.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
            match ingest_one_file(vault, vault_root, &fp, &fname, &description, &author, &tags) {
                ToolResult { is_error: false, content } => {
                    results.push(serde_json::from_str::<Value>(&content[0].text).unwrap_or(Value::Null));
                }
                ToolResult { content, .. } => {
                    errors.push(format!("{fname}: {}", content[0].text));
                }
            }
        }

        ToolResult::json(json!({
            "ingested": results.len(),
            "errors":   errors.len(),
            "assets":   results,
            "error_details": errors,
        }))
    } else {
        ToolResult::error(format!("path is neither a file nor directory: {path_str}"))
    }
}

fn ingest_one_file(
    vault:      &VaultRegistry,
    vault_root: &Path,
    path:       &std::path::Path,
    name:       &str,
    description: &str,
    author:     &str,
    tags:       &[String],
) -> ToolResult {
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let display_name = if name.is_empty() { &filename } else { name };

    let bytes = match std::fs::read(path) {
        Ok(b)  => b,
        Err(e) => return ToolResult::error(format!("failed to read {filename}: {e}")),
    };

    let id = MythId::new();
    match vault.ingest(id, &bytes) {
        Err(e) => ToolResult::error(format!("vault ingest failed for {filename}: {e}")),
        Ok(canonical_id) => {
            let asset_type  = AssetType::from_filename(&filename);
            let myth_id_str = canonical_id.as_str();
            let mut index   = load_index(vault_root);

            if !index.iter().any(|m| m.myth_id == myth_id_str) {
                index.push(AssetMeta {
                    myth_id:     myth_id_str.clone(),
                    name:        display_name.to_string(),
                    asset_type:  asset_type.clone(),
                    tags:        tags.to_vec(),
                    description: description.to_string(),
                    author:      author.to_string(),
                    created_at:  chrono::Utc::now().to_rfc3339(),
                    filename:    filename.clone(),
                    size:        bytes.len(),
                });
                save_index(vault_root, &index);
            }

            ToolResult::json(json!({
                "myth_id":    myth_id_str,
                "filename":   filename,
                "asset_type": asset_type.label(),
                "size_bytes": bytes.len(),
            }))
        }
    }
}

// ─── Index file (vault_root/asset-index.json) ────────────────────────────────
// Stores all AssetMeta entries keyed by myth_id.
// Separate from vault content so metadata is human-readable and grep-able.

fn index_path(vault_root: &Path) -> PathBuf {
    vault_root.join("asset-index.json")
}

fn load_index(vault_root: &Path) -> Vec<AssetMeta> {
    let path = index_path(vault_root);
    if !path.exists() { return vec![]; }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_index(vault_root: &Path, index: &[AssetMeta]) {
    let path = index_path(vault_root);
    if let Ok(json) = serde_json::to_string_pretty(index) {
        let _ = std::fs::write(path, json);
    }
}

// ─── vault_ingest ────────────────────────────────────────────────────────────

fn ingest(vault: &VaultRegistry, vault_root: &Path, p: &Value) -> ToolResult {
    let filename    = str_field(p, "filename");
    let data_b64    = str_field(p, "data_base64");
    let name        = str_field(p, "name");
    let description = p["description"].as_str().unwrap_or("").to_string();
    let author      = p["author"].as_str().unwrap_or("unknown").to_string();
    let tags: Vec<String> = p["tags"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    if filename.is_empty() || data_b64.is_empty() || name.is_empty() {
        return ToolResult::error("filename, data_base64, and name are required");
    }

    let bytes = match base64_decode(&data_b64) {
        Ok(b)  => b,
        Err(e) => return ToolResult::error(format!("base64 decode failed: {e}")),
    };

    let id = MythId::new();
    match vault.ingest(id, &bytes) {
        Err(e) => ToolResult::error(format!("vault ingest failed: {e}")),
        Ok(canonical_id) => {
            let asset_type = AssetType::from_filename(&filename);
            let myth_id_str = canonical_id.as_str();

            let mut index = load_index(vault_root);
            // Dedup by myth_id
            if !index.iter().any(|m| m.myth_id == myth_id_str) {
                index.push(AssetMeta {
                    myth_id: myth_id_str.clone(),
                    name: name.clone(),
                    asset_type: asset_type.clone(),
                    tags,
                    description,
                    author,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    filename,
                    size: bytes.len(),
                });
                save_index(vault_root, &index);
            }

            ToolResult::json(json!({
                "myth_id":    myth_id_str,
                "asset_type": asset_type.label(),
                "size_bytes": bytes.len(),
                "message":    "Stored. Reference this myth_id from any vault portal. Directory is relocatable — only MYTH_VAULT_ROOT needs updating."
            }))
        }
    }
}

// ─── vault_list ──────────────────────────────────────────────────────────────

fn list(vault_root: &Path, p: &Value) -> ToolResult {
    let type_filter = p["asset_type"].as_str().map(String::from);
    let tag_filter  = p["tag"].as_str().map(String::from);
    let limit       = p["limit"].as_u64().unwrap_or(50) as usize;

    let index = load_index(vault_root);
    let results: Vec<&AssetMeta> = index.iter()
        .filter(|m| {
            if let Some(tf) = &type_filter {
                let label = serde_json::to_value(&m.asset_type)
                    .ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                if &label != tf { return false; }
            }
            if let Some(tag) = &tag_filter {
                if !m.tags.iter().any(|t| t == tag) { return false; }
            }
            true
        })
        .take(limit)
        .collect();

    ToolResult::json(json!({ "count": results.len(), "assets": results }))
}

// ─── vault_fetch ─────────────────────────────────────────────────────────────

fn fetch(vault: &VaultRegistry, vault_root: &Path, p: &Value) -> ToolResult {
    let myth_id_str = str_field(p, "myth_id");
    if myth_id_str.is_empty() {
        return ToolResult::error("myth_id is required");
    }

    let id = match MythId::parse(&myth_id_str) {
        Some(id) => id,
        None     => return ToolResult::error(format!("invalid myth_id: {myth_id_str}")),
    };

    match vault.fetch(&id) {
        Err(e)    => ToolResult::error(format!("not found: {e}")),
        Ok(bytes) => {
            let index = load_index(vault_root);
            let meta  = index.iter().find(|m| m.myth_id == myth_id_str);
            ToolResult::json(json!({
                "myth_id":     myth_id_str,
                "meta":        meta,
                "size_bytes":  bytes.len(),
                "data_base64": base64_encode(&bytes),
            }))
        }
    }
}

// ─── vault_search ────────────────────────────────────────────────────────────

fn search(vault_root: &Path, p: &Value) -> ToolResult {
    let query       = str_field(p, "query").to_lowercase();
    let type_filter = p["asset_type"].as_str().map(String::from);

    if query.is_empty() {
        return ToolResult::error("query is required");
    }

    let index = load_index(vault_root);
    let results: Vec<&AssetMeta> = index.iter()
        .filter(|m| {
            let matches = m.name.to_lowercase().contains(&query)
                || m.description.to_lowercase().contains(&query)
                || m.tags.iter().any(|t| t.to_lowercase().contains(&query));
            if !matches { return false; }
            if let Some(tf) = &type_filter {
                let label = serde_json::to_value(&m.asset_type)
                    .ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                if &label != tf { return false; }
            }
            true
        })
        .collect();

    ToolResult::json(json!({ "count": results.len(), "assets": results }))
}

// ─── vault_info ──────────────────────────────────────────────────────────────

fn info(vault: &VaultRegistry, vault_root: &Path) -> ToolResult {
    let index = load_index(vault_root);
    let mut by_type: std::collections::HashMap<String, usize> = Default::default();
    for m in &index {
        *by_type.entry(m.asset_type.label().to_string()).or_default() += 1;
    }

    ToolResult::json(json!({
        "vault_profile": format!("{:?}", vault.profile),
        "total_assets":  index.len(),
        "by_type":       by_type,
        "index_path":    index_path(vault_root).display().to_string(),
        "relocatable":   true,
        "note": "Move the vault directory freely. Update MYTH_VAULT_ROOT env var or --vault arg. All MythIds remain valid."
    }))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn str_field(p: &Value, key: &str) -> String {
    p[key].as_str().unwrap_or("").to_string()
}

fn base64_encode(bytes: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };
        out.push(CHARS[b0 >> 2] as char);
        out.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        out.push(if chunk.len() > 1 { CHARS[((b1 & 0xf) << 2) | (b2 >> 6)] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[b2 & 0x3f] as char } else { '=' });
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim().replace(['\n', '\r', ' '], "");
    let lookup = |c: u8| -> Result<u8, String> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+'        => Ok(62),
            b'/'        => Ok(63),
            b'='        => Ok(0),
            _           => Err(format!("invalid base64 char: {c}")),
        }
    };
    let bytes = s.as_bytes();
    if bytes.len() % 4 != 0 { return Err("base64 length not multiple of 4".into()); }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let v0 = lookup(chunk[0])?; let v1 = lookup(chunk[1])?;
        let v2 = lookup(chunk[2])?; let v3 = lookup(chunk[3])?;
        out.push((v0 << 2) | (v1 >> 4));
        if chunk[2] != b'=' { out.push((v1 << 4) | (v2 >> 2)); }
        if chunk[3] != b'=' { out.push((v2 << 6) | v3); }
    }
    Ok(out)
}
