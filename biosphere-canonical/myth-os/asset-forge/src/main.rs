// qforge — Asset Forge CLI
// Part of the myth-os / Quantum Ecosystem toolchain.
//
// Subcommands:
//   generate   Build manifests for all .toml configs in a directory
//   manifest   Build a single manifest from one .toml and print / write it
//   validate   Validate token vocabulary in one or all configs
//   status     Print a table of all config files under a directory
//   new        Scaffold a new .toml config from a guided prompt

mod config;
mod manifest;
mod pipeline;
mod prompt;
mod tokens;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name    = "qforge",
    version = "0.1.0",
    about   = "Quantum Asset Forge — manifest & prompt builder for modular GLTF assets",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate manifests for every .toml config under a directory
    Generate {
        /// Root directory containing .toml asset configs
        #[arg(short, long, default_value = ".")]
        input: PathBuf,

        /// Output directory for manifest JSON files
        #[arg(short, long, default_value = "manifests")]
        output: PathBuf,

        /// Print what would be generated without writing any files
        #[arg(long)]
        dry_run: bool,

        /// Also write a combined catalogue JSON
        #[arg(long)]
        catalogue: bool,
    },

    /// Build and display a single manifest from a .toml config
    Manifest {
        /// Path to the .toml config file
        config: PathBuf,

        /// Specific letter variant to generate (A, B, C…)
        #[arg(short, long)]
        letter: Option<String>,

        /// Write the manifest JSON to this file (stdout if omitted)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Validate token vocabulary in one config or a whole directory
    Validate {
        /// File or directory to validate
        path: PathBuf,
    },

    /// Print a status table of all .toml configs under a directory
    Status {
        /// Root directory to scan
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Scaffold a new asset config interactively
    New {
        /// Where to write the new .toml
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate { input, output, dry_run, catalogue } => {
            cmd_generate(&input, &output, dry_run, catalogue)
        }
        Commands::Manifest { config, letter, output } => {
            cmd_manifest(&config, letter.as_deref(), output.as_deref())
        }
        Commands::Validate { path } => {
            cmd_validate(&path)
        }
        Commands::Status { path } => {
            cmd_status(&path)
        }
        Commands::New { output } => {
            cmd_new(output.as_deref())
        }
    }
}

// ── Subcommand implementations ────────────────────────────────────────────────

fn cmd_generate(
    input:     &PathBuf,
    output:    &PathBuf,
    dry_run:   bool,
    catalogue: bool,
) -> Result<()> {
    if !input.exists() {
        bail!("Input path does not exist: {}", input.display());
    }

    println!();
    println!("⬡  qforge generate");
    println!("   input  : {}", input.display());
    println!("   output : {}", output.display());
    if dry_run { println!("   mode   : DRY RUN — no files will be written"); }
    println!();

    let results = pipeline::run_batch(input, output, dry_run);

    if catalogue && !dry_run {
        let all: Vec<_> = results.iter()
            .flat_map(|r| r.manifests.iter().cloned())
            .collect();

        let cat_path = manifest::write_catalogue(&all, output, "catalogue")
            .context("Writing catalogue JSON")?;
        println!("  ⬡  Catalogue → {}", cat_path.display());
    }

    pipeline::print_summary(&results);

    let has_err = results.iter().any(|r| r.error.is_some());
    if has_err {
        std::process::exit(1);
    }

    Ok(())
}

fn cmd_manifest(
    config_path: &PathBuf,
    letter:      Option<&str>,
    output:      Option<&std::path::Path>,
) -> Result<()> {
    let cfg = config::load(config_path)
        .with_context(|| format!("Loading {}", config_path.display()))?;

    let report = tokens::validate(&cfg);
    if !report.is_clean() {
        for w in &report.warnings {
            eprintln!("{w}");
        }
    }

    let m    = manifest::build(&cfg, letter);
    let json = serde_json::to_string_pretty(&m)?;

    match output {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, &json)
                .with_context(|| format!("Writing {}", p.display()))?;
            println!("  ✓  Manifest written → {}", p.display());
        }
        None => {
            println!("{json}");
        }
    }

    Ok(())
}

fn cmd_validate(path: &PathBuf) -> Result<()> {
    println!();
    println!("⬡  qforge validate — {}", path.display());
    println!();

    if path.is_file() {
        validate_file(path);
    } else if path.is_dir() {
        use walkdir::WalkDir;
        let files: Vec<PathBuf> = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("toml"))
            .map(|e| e.path().to_path_buf())
            .collect();

        if files.is_empty() {
            println!("  No .toml files found.");
        } else {
            for f in &files {
                validate_file(f);
            }
        }
    } else {
        bail!("Path does not exist: {}", path.display());
    }

    println!();
    Ok(())
}

fn cmd_status(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        bail!("Path does not exist: {}", path.display());
    }
    pipeline::print_status(path);
    Ok(())
}

fn cmd_new(output: Option<&std::path::Path>) -> Result<()> {
    println!();
    println!("⬡  qforge new — scaffold a new asset config");
    println!();

    // Gather values interactively from stdin
    let asset_type  = prompt_user("Asset type token (e.g. CAVE_ENTRANCE, AIRSHIP_HULL)", None)?;
    let zone        = prompt_user("Zone (e.g. CAVE, SKY, ALIEN — blank to skip)", Some(""))?;
    let function    = prompt_user("Function (e.g. ENTRANCE, HULL — blank to skip)", Some(""))?;
    let variant     = prompt_user("Variant (e.g. ORGANIC, RUINED — blank to skip)", Some(""))?;
    let scale       = prompt_user("Scale (e.g. 1X1, 2X2 — blank to skip)", Some("1X1"))?;
    let direction   = prompt_user("Render direction (ISW / INE / FRONT / etc.)", Some("ISW"))?;
    let angle       = prompt_user("Render angle (ISOMETRIC / PERSPECTIVE)", Some("ISOMETRIC"))?;
    let shader      = prompt_user("Shader (PBR / UNLIT / MATCAP / TOON)", Some("PBR"))?;
    let variants_n  = prompt_user("How many letter variants? (1–8, or 0 for none)", Some("0"))?;

    let variants_field = match variants_n.trim().parse::<u8>() {
        Ok(0) | Err(_) => String::new(),
        Ok(n)           => format!("variants = {n}\n"),
    };

    let opt = |v: &str, key: &str| -> String {
        if v.trim().is_empty() { String::new() }
        else { format!("{key} = \"{}\"\n", v.trim().to_uppercase()) }
    };

    let toml = format!(
        r#"[asset]
type     = "{type}"
{zone}{function}{variant}{scale}{variants}
[sockets]
# north = "PASSAGE_MD"
# south = "CLOSED"

[render]
background = "TRANSPARENT"
{direction}{angle}{shader}
[meta]
# quantum_module = "Forge"
# resonance_hz   = 174.6
# tags = ["tileable"]
"#,
        type      = asset_type.trim().to_uppercase(),
        zone      = opt(&zone,     "zone"),
        function  = opt(&function, "function"),
        variant   = opt(&variant,  "variant"),
        scale     = opt(&scale,    "scale"),
        variants  = variants_field,
        direction = opt(&direction, "direction"),
        angle     = opt(&angle,     "angle"),
        shader    = opt(&shader,    "shader"),
    );

    // Determine output path
    let stem = asset_type.trim().to_uppercase().replace(' ', "_");
    let default_path = PathBuf::from(format!("{stem}.toml"));
    let out_path = output.unwrap_or(&default_path);

    std::fs::write(out_path, &toml)
        .with_context(|| format!("Writing {}", out_path.display()))?;

    println!();
    println!("  ✓  Created {}", out_path.display());
    println!("     Run `qforge validate {}` to check tokens.", out_path.display());
    println!();

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_file(path: &PathBuf) {
    let stem = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
    match config::load(path) {
        Err(e) => println!("  ✗  {stem} — parse error: {e}"),
        Ok(cfg) => {
            let report = tokens::validate(&cfg);
            if report.is_clean() {
                println!("  ✓  {stem}");
            } else {
                println!("  ⚠  {stem}  ({} warning{})",
                    report.warnings.len(),
                    if report.warnings.len() == 1 { "" } else { "s" });
                for w in &report.warnings {
                    println!("     {w}");
                }
            }
        }
    }
}

fn prompt_user(question: &str, default: Option<&str>) -> Result<String> {
    use std::io::{self, Write};

    match default {
        Some(d) if !d.is_empty() => print!("  {question} [{d}]: "),
        _                         => print!("  {question}: "),
    }
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim().to_string();

    Ok(if trimmed.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        trimmed
    })
}
