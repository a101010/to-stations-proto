//! `dds-contracts`: validate the service contracts, write `build/topics.md`,
//! and scan the caller's source roots for raw CycloneDDS topic-API calls.

use std::path::{Path, PathBuf};

use clap::Parser;

use dds_contracts::scan;
use stations_dds::contract::{self, ServiceContract, TopicRegistry};

#[derive(Parser, Debug)]
#[command(
    name = "dds-contracts",
    about = "validate service contracts and scan for DDS topic-API bypasses"
)]
struct Args {
    /// Directory holding `topics.toml` and `services/`.
    #[arg(long, default_value = "contracts")]
    contracts: PathBuf,

    /// Scan a source root for raw CycloneDDS topic-API calls (repeatable).
    #[arg(long = "scan")]
    scan: Vec<PathBuf>,

    /// Skip a subtree during the scan (repeatable).
    #[arg(long = "allow")]
    allow: Vec<PathBuf>,

    /// Write the summary here.
    #[arg(long, default_value = "build/topics.md")]
    summary: PathBuf,

    /// Print a service's contract and exit.
    #[arg(long)]
    topics: Option<String>,

    /// Validate only; do not write the summary.
    #[arg(long)]
    check: bool,
}

fn main() -> std::process::ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("dds-contracts: {message}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    if !args.scan.is_empty() {
        let bypasses = scan::scan(&args.scan, &args.allow).map_err(|e| format!("scan: {e}"))?;
        for b in &bypasses {
            eprintln!(
                "{}:{}: raw topic API `{}` (use the Node API)",
                b.file.display(),
                b.line,
                b.name
            );
        }
        if !bypasses.is_empty() {
            return Err(format!("{} raw topic-API call(s) found", bypasses.len()));
        }
    }

    let registry = TopicRegistry::load(&args.contracts.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;

    let services = load_services(&args.contracts, &registry)?;

    if let Some(name) = &args.topics {
        let service = services
            .iter()
            .find(|s| &s.service.name == name)
            .ok_or_else(|| format!("no contract for service `{name}`"))?;
        print!("{}", service.render());
        return Ok(());
    }

    if args.check {
        return Ok(());
    }

    let text = contract::summary(&registry, &services);
    if let Some(parent) = args.summary.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::write(&args.summary, text).map_err(|e| format!("write {}: {e}", args.summary.display()))?;
    Ok(())
}

fn load_services(dir: &Path, registry: &TopicRegistry) -> Result<Vec<ServiceContract>, String> {
    let services_dir = dir.join("services");
    let mut paths = Vec::new();
    if services_dir.is_dir() {
        for entry in std::fs::read_dir(&services_dir)
            .map_err(|e| format!("read {}: {e}", services_dir.display()))?
        {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                paths.push(path);
            }
        }
    }
    paths.sort();

    let mut services = Vec::new();
    for path in paths {
        let service = ServiceContract::load(&path).map_err(|e| e.to_string())?;
        service.validate(registry).map_err(|e| e.to_string())?;
        services.push(service);
    }
    Ok(services)
}
