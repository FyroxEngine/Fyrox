mod protocol;
mod tools;
mod server;
mod asset;

use clap::Parser;
use std::path::PathBuf;
use tracing::info;

#[derive(Parser)]
#[command(name = "myth-vault-mcp", about = "MCP server for the myth-os Master Vault")]
struct Cli {
    /// Path to the vault root directory.
    /// Overrides MYTH_VAULT_ROOT env var.
    /// Move the vault directory freely — only this path needs updating.
    #[arg(long, env = "MYTH_VAULT_ROOT")]
    vault: PathBuf,
}

fn main() -> anyhow::Result<()> {
    // Log to stderr so stdout stays clean for JSON-RPC
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            std::env::var("MYTH_LOG").unwrap_or_else(|_| "info".into())
        )
        .init();

    let cli = Cli::parse();

    info!("myth-vault-mcp starting — vault root: {}", cli.vault.display());

    let vault = myth_vault::VaultRegistry::open(&cli.vault)
        .map_err(|e| anyhow::anyhow!("failed to open vault at {:?}: {e}", cli.vault))?;

    info!("vault profile: {:?}", vault.profile);

    server::run(vault, cli.vault)
}
