use clap::{Parser, Subcommand};
use fyrox_biosphere::{
    capacity::SealType,
    container_format::{
        CapsuleHeraldry, ContainerHeraldry, GenesisContainer,
    },
    domain::Domain,
    heraldry::CrestName,
    wire::WireType,
};
use std::{fs, path::PathBuf, str::FromStr};

#[derive(Parser)]
#[command(name = "qgenesis-tool", about = "BioSpark Quantum Genesis container toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Genesis Container (.qgenesis file)
    New {
        name: String,
        #[arg(short, long, default_value = "narrative")]
        domain: String,
        #[arg(short, long, default_value = "")]
        world_type: String,
        #[arg(long)]
        seal: Option<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Add a Mythos Container to an existing .qgenesis file
    AddMythos {
        file: PathBuf,
        name: String,
        #[arg(short, long, default_value = "Core")]
        crest: String,
    },
    /// Add a Container to a Mythos (by Mythos name or id)
    AddContainer {
        file: PathBuf,
        mythos: String,
        name: String,
        #[arg(long, default_value = "glyph")]
        heraldry: String,
    },
    /// Add a Capsule to a Container (by Container name or id)
    AddCapsule {
        file: PathBuf,
        container: String,
        name: String,
        #[arg(long, default_value = "mark")]
        heraldry: String,
        #[arg(long, default_value = "Data")]
        wire: String,
    },
    /// Transition a Genesis Container from Seeding → Active
    Activate { file: PathBuf },
    /// Seal a Genesis Container (freeze hierarchy, Active → Sealed)
    Seal { file: PathBuf },
    /// Validate a .qgenesis file (capacity law, alignment, B-DNA)
    Validate { file: PathBuf },
    /// Pretty-print the container hierarchy
    Inspect { file: PathBuf },
}

fn load(path: &PathBuf) -> anyhow::Result<GenesisContainer> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn save(path: &PathBuf, genesis: &GenesisContainer) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(genesis)?;
    fs::write(path, text)?;
    Ok(())
}

fn parse_crest(s: &str) -> CrestName {
    match s.to_lowercase().as_str() {
        "core" => CrestName::Core,
        "atlas" => CrestName::Atlas,
        "vault" => CrestName::Vault,
        "mythos" => CrestName::Mythos,
        "codex" => CrestName::Codex,
        "loom" => CrestName::Loom,
        "composer" => CrestName::Composer,
        "forge" => CrestName::Forge,
        "order" => CrestName::Order,
        "mind" => CrestName::Mind,
        "soul" => CrestName::Soul,
        _ => CrestName::Custom(s.to_string()),
    }
}

fn parse_container_heraldry(s: &str) -> anyhow::Result<ContainerHeraldry> {
    match s.to_lowercase().as_str() {
        "glyph" => Ok(ContainerHeraldry::Glyph),
        "device" => Ok(ContainerHeraldry::Device),
        "emblem" => Ok(ContainerHeraldry::Emblem),
        _ => Err(anyhow::anyhow!("Unknown container heraldry '{}' (use: glyph, device, emblem)", s)),
    }
}

fn parse_capsule_heraldry(s: &str) -> anyhow::Result<CapsuleHeraldry> {
    match s.to_lowercase().as_str() {
        "trait" => Ok(CapsuleHeraldry::Trait),
        "mark" => Ok(CapsuleHeraldry::Mark),
        "token" => Ok(CapsuleHeraldry::Token),
        "sigil" => Ok(CapsuleHeraldry::Sigil),
        _ => Err(anyhow::anyhow!("Unknown capsule heraldry '{}' (use: trait, mark, token, sigil)", s)),
    }
}

fn parse_wire(s: &str) -> anyhow::Result<WireType> {
    match s.to_uppercase().as_str() {
        "DAT" | "DATA" => Ok(WireType::Data),
        "CTL" | "CONTROL" => Ok(WireType::Control),
        "AUD" | "AUDIO" => Ok(WireType::Audio),
        "NAR" | "NARRATIVE" => Ok(WireType::Narrative),
        "TMP" | "TEMPORAL" => Ok(WireType::Temporal),
        "AGT" | "AGENT" => Ok(WireType::Agent),
        "VIS" | "VISUAL" => Ok(WireType::Visual),
        "SPA" | "SPATIAL" => Ok(WireType::Spatial),
        "BHV" | "BEHAVIORAL" => Ok(WireType::Behavioral),
        "SOC" | "SOCIAL" => Ok(WireType::Social),
        "ENR" | "ENERGY" => Ok(WireType::Energy),
        "IDN" | "IDENTITY" => Ok(WireType::Identity),
        "EVT" | "EVENT" => Ok(WireType::Event),
        "AST" | "ASSET" => Ok(WireType::Asset),
        "MET" | "META" => Ok(WireType::Meta),
        "LGC" | "LOGIC" => Ok(WireType::Logic),
        "RES" | "RESONANCE" => Ok(WireType::Resonance),
        _ => Err(anyhow::anyhow!("Unknown wire type '{}' (use abbreviation like DAT, NAR, AUD, RES...)", s)),
    }
}

fn cmd_new(
    name: String,
    domain_str: String,
    world_type: String,
    seal: Option<String>,
    output: Option<PathBuf>,
) -> anyhow::Result<()> {
    let domain = Domain::from_str(&domain_str).unwrap();
    let mut genesis = GenesisContainer::new(name.clone(), domain, None);
    genesis.world_type = world_type;
    if let Some(s) = seal {
        if s.to_lowercase() == "lesser" {
            genesis.seal_type = SealType::Lesser;
        }
    }
    let path = output.unwrap_or_else(|| {
        PathBuf::from(format!("{}.qgenesis", name.to_lowercase().replace(' ', "-")))
    });
    save(&path, &genesis)?;
    println!("Created: {}", path.display());
    println!("  Domain:     {}", genesis.domain);
    println!("  Seal type:  {:?}", genesis.seal_type);
    println!("  Lifecycle:  {:?}", genesis.lifecycle);
    println!("  Resonance:  {} Hz", genesis.resonance_hz);
    Ok(())
}

fn cmd_add_mythos(file: PathBuf, name: String, crest: String) -> anyhow::Result<()> {
    let mut genesis = load(&file)?;
    let crest = parse_crest(&crest);
    genesis.add_mythos(name.clone(), crest.clone(), None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    save(&file, &genesis)?;
    println!("Added Mythos '{}' (Crest: {}) to '{}'", name, crest, genesis.name);
    println!("  Mythos count: {}/{}", genesis.mythos.len(), genesis.capacity.max_children_current());
    Ok(())
}

fn cmd_add_container(file: PathBuf, mythos: String, name: String, heraldry: String) -> anyhow::Result<()> {
    let mut genesis = load(&file)?;
    let heraldry = parse_container_heraldry(&heraldry)?;
    let mythos_entry = genesis.mythos.iter_mut()
        .find(|m| m.name == mythos || m.id == mythos)
        .ok_or_else(|| anyhow::anyhow!("Mythos '{}' not found", mythos))?;
    mythos_entry.add_container(name.clone(), heraldry.clone(), None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let count = mythos_entry.containers.len();
    let max = mythos_entry.capacity.max_children_current();
    save(&file, &genesis)?;
    println!("Added Container '{}' ({:?}) to Mythos '{}'", name, heraldry, mythos);
    println!("  Container count: {}/{}", count, max);
    Ok(())
}

fn cmd_add_capsule(
    file: PathBuf,
    container: String,
    name: String,
    heraldry: String,
    wire: String,
) -> anyhow::Result<()> {
    let mut genesis = load(&file)?;
    let heraldry = parse_capsule_heraldry(&heraldry)?;
    let wire_type = parse_wire(&wire)?;

    for mythos in &mut genesis.mythos {
        for cont in &mut mythos.containers {
            if cont.name == container || cont.id == container {
                cont.add_capsule(name.clone(), heraldry.clone(), wire_type, serde_json::Value::Null, None)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let count = cont.capsules.len();
                let max = cont.capacity.max_children_current();
                save(&file, &genesis)?;
                println!("Added Capsule '{}' ({:?}, {}) to Container '{}'", name, heraldry, wire_type.abbreviation(), container);
                println!("  Capsule count: {}/{}", count, max);
                return Ok(());
            }
        }
    }
    Err(anyhow::anyhow!("Container '{}' not found", container))
}

fn cmd_activate(file: PathBuf) -> anyhow::Result<()> {
    let mut genesis = load(&file)?;
    genesis.activate().map_err(|e| anyhow::anyhow!("{e}"))?;
    save(&file, &genesis)?;
    println!("'{}' is now Active", genesis.name);
    Ok(())
}

fn cmd_seal(file: PathBuf) -> anyhow::Result<()> {
    let mut genesis = load(&file)?;
    genesis.seal().map_err(|e| anyhow::anyhow!("{e}"))?;
    save(&file, &genesis)?;
    println!("'{}' sealed — hierarchy frozen", genesis.name);
    println!("  Capsule count: {}", genesis.total_capsule_count());
    Ok(())
}

fn cmd_validate(file: PathBuf) -> anyhow::Result<()> {
    let genesis = load(&file)?;
    let errors = genesis.validate();
    if errors.is_empty() {
        println!("✓ '{}' is valid", genesis.name);
        println!("  Lifecycle: {:?}", genesis.lifecycle);
        println!("  Mythos: {}", genesis.mythos.len());
        println!("  Total capsules: {}", genesis.total_capsule_count());
    } else {
        eprintln!("✗ '{}' has {} validation error(s):", genesis.name, errors.len());
        for e in &errors {
            eprintln!("  - {e}");
        }
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_inspect(file: PathBuf) -> anyhow::Result<()> {
    let genesis = load(&file)?;
    let lifecycle_str = format!("{:?}", genesis.lifecycle);
    println!("╔═══════════════════════════════════════════");
    println!("║ GENESIS  {} [{}]", genesis.name, lifecycle_str);
    println!("║ Domain:  {}  |  World: {}  |  Seal: {:?}", genesis.domain,
        if genesis.world_type.is_empty() { "(unset)" } else { &genesis.world_type },
        genesis.seal_type);
    println!("║ Resonance: {} Hz  |  Mythos: {}/{}",
        genesis.resonance_hz, genesis.mythos.len(), genesis.capacity.max_children_current());
    println!("║ B-DNA: {}  |  Capsules total: {}", &genesis.bdna.signature[..8], genesis.total_capsule_count());
    println!("╠═══════════════════════════════════════════");

    for (mi, mythos) in genesis.mythos.iter().enumerate() {
        let tree_char = if mi == genesis.mythos.len() - 1 { "└" } else { "├" };
        println!("║ {}── MYTHOS  {} [{}] (Crest: {})",
            tree_char, mythos.name, format!("{:?}", mythos.lifecycle), mythos.crest);
        for (ci, container) in mythos.containers.iter().enumerate() {
            let c_tree = if ci == mythos.containers.len() - 1 { "    └" } else { "    ├" };
            println!("║ {}── CONTAINER  {} [{:?}] ({:?})",
                c_tree, container.name, container.lifecycle, container.heraldry);
            for (pi, capsule) in container.capsules.iter().enumerate() {
                let p_tree = if pi == container.capsules.len() - 1 { "         └" } else { "         ├" };
                println!("║ {}── CAPSULE  {} [{:?}] wire={} birth={:?}",
                    p_tree, capsule.name, capsule.heraldic_current,
                    capsule.wire_type.abbreviation(), capsule.heraldic_birth);
            }
        }
    }
    println!("╚═══════════════════════════════════════════");
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::New { name, domain, world_type, seal, output } =>
            cmd_new(name, domain, world_type, seal, output),
        Command::AddMythos { file, name, crest } =>
            cmd_add_mythos(file, name, crest),
        Command::AddContainer { file, mythos, name, heraldry } =>
            cmd_add_container(file, mythos, name, heraldry),
        Command::AddCapsule { file, container, name, heraldry, wire } =>
            cmd_add_capsule(file, container, name, heraldry, wire),
        Command::Activate { file } => cmd_activate(file),
        Command::Seal { file } => cmd_seal(file),
        Command::Validate { file } => cmd_validate(file),
        Command::Inspect { file } => cmd_inspect(file),
    };
    if let Err(e) = result {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
