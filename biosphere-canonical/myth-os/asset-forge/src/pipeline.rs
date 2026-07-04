// Pipeline — batch-walks a directory of .toml configs and processes each one.

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::config;
use crate::manifest::{self, AssetManifest};
use crate::tokens;

/// Result of processing a single config file.
pub struct ProcessResult {
    pub config_path: PathBuf,
    pub manifests:   Vec<AssetManifest>,
    pub warnings:    Vec<String>,
    pub error:       Option<String>,
}

/// Walk `root` for `*.toml` files and process each one.
/// `out_dir` is where manifest JSON files will be written.
/// Returns a list of results (one per config).
pub fn run_batch(root: &Path, out_dir: &Path, dry_run: bool) -> Vec<ProcessResult> {
    let toml_files: Vec<PathBuf> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("toml"))
        .map(|e| e.path().to_path_buf())
        .collect();

    toml_files
        .into_iter()
        .map(|p| process_one(&p, out_dir, dry_run))
        .collect()
}

/// Process a single config file, generating manifests for each letter variant.
pub fn process_one(config_path: &Path, out_dir: &Path, dry_run: bool) -> ProcessResult {
    let cfg = match config::load(config_path)
        .with_context(|| format!("Loading {}", config_path.display()))
    {
        Ok(c)  => c,
        Err(e) => {
            return ProcessResult {
                config_path: config_path.to_path_buf(),
                manifests:   Vec::new(),
                warnings:    Vec::new(),
                error:       Some(e.to_string()),
            };
        }
    };

    // Validate tokens
    let report = tokens::validate(&cfg);
    let warnings = report.warnings.clone();

    // Determine how many letter variants to emit
    let letters: Vec<Option<String>> = if let Some(n) = cfg.asset.variants {
        // Auto-generate A, B, C, …
        (0..n)
            .map(|i| Some(letter_for(i)))
            .collect()
    } else if let Some(ref l) = cfg.asset.letter {
        // Explicit single letter
        vec![Some(l.clone())]
    } else {
        // No letter — single unnamed asset
        vec![None]
    };

    let mut manifests = Vec::new();

    for letter_opt in &letters {
        let m = manifest::build(&cfg, letter_opt.as_deref());

        if !dry_run {
            // Sub-directory mirrors the domain/zone structure
            let sub = out_dir.join(crate::prompt::build_dir(&cfg));
            if let Err(e) = manifest::write(&m, &sub) {
                // Non-fatal; include as warning
                let _ = &warnings; // already captured above
                eprintln!("  ✗  Could not write manifest: {e}");
            }
        }

        manifests.push(m);
    }

    ProcessResult {
        config_path: config_path.to_path_buf(),
        manifests,
        warnings,
        error: None,
    }
}

/// Pretty-print a batch result summary to stdout.
pub fn print_summary(results: &[ProcessResult]) {
    let total_configs  = results.len();
    let total_ok       = results.iter().filter(|r| r.error.is_none()).count();
    let total_err      = total_configs - total_ok;
    let total_warnings = results.iter().map(|r| r.warnings.len()).sum::<usize>();
    let total_assets   = results.iter().map(|r| r.manifests.len()).sum::<usize>();

    println!();
    println!("── Batch complete ─────────────────────────────────");
    println!("  Configs:   {total_configs}  ({total_ok} ok, {total_err} errors)");
    println!("  Assets:    {total_assets}");
    println!("  Warnings:  {total_warnings}");
    println!("───────────────────────────────────────────────────");

    for r in results {
        let stem = r.config_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        if let Some(ref e) = r.error {
            println!("  ✗  {stem}");
            println!("     {e}");
        } else {
            let asset_count = r.manifests.len();
            println!("  ✓  {stem}  ({asset_count} asset{})", if asset_count == 1 { "" } else { "s" });

            for w in &r.warnings {
                println!("     {w}");
            }
        }
    }
    println!();
}

/// Find all .toml files under `root` and print a status table.
pub fn print_status(root: &Path) {
    let entries: Vec<(PathBuf, Result<config::AssetConfig>)> = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("toml"))
        .map(|e| {
            let p = e.path().to_path_buf();
            let r = config::load(&p);
            (p, r)
        })
        .collect();

    println!();
    println!("── Asset Config Status ─────────────────────────────");
    println!("  {:<48} {:<8} WARN", "FILE", "STATUS");
    println!("  {}", "─".repeat(64));

    for (path, result) in &entries {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let rel_str = rel.to_string_lossy();

        match result {
            Ok(cfg) => {
                let report = tokens::validate(cfg);
                let w = report.warnings.len();
                let status = if report.is_clean() { "OK" } else { "WARN" };
                println!("  {:<48} {:<8} {}", trunc(&rel_str, 48), status, w);
            }
            Err(e) => {
                println!("  {:<48} ERROR   — {e}", trunc(&rel_str, 48));
            }
        }
    }

    let ok   = entries.iter().filter(|(_, r)| r.is_ok()).count();
    let err  = entries.len() - ok;
    println!("  {}", "─".repeat(64));
    println!("  Total: {}  ({} ok, {} error{})", entries.len(), ok, err, if err == 1 { "" } else { "s" });
    println!();
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn letter_for(i: u8) -> String {
    // 0→A, 1→B, …, 25→Z, 26→AA, etc.
    let mut n = i as u32;
    let mut s = String::new();
    loop {
        s.insert(0, char::from_u32(b'A' as u32 + n % 26).unwrap_or('A'));
        if n < 26 { break; }
        n = n / 26 - 1;
    }
    s
}

fn trunc(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("…{}", &s[s.len().saturating_sub(max - 1)..])
    }
}
