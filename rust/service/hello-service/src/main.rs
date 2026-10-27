//! The hello_world model service: publishes `HelloWorldTopic` with the greeting
//! and the earth attitude at a fixed rate.
//!
//! The greeting cycles through a fixed Latin-script list every 15-60 seconds.
//! The attitude carries the permanent tilt and the 12-minute spin.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::dds::Node;
use stations_dds::hello_world::HelloWorldTopic;

mod attitude;
mod greeting;

#[derive(Parser, Debug)]
#[command(name = "hello-service", about = "hello_world model service")]
struct Args {
    /// DDS domain id.
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Publish period, in milliseconds.
    #[arg(long, default_value_t = 100)]
    rate_ms: u64,

    /// Contracts directory (default: `<exe_dir>/../config`, else `./contracts`).
    #[arg(long)]
    config: Option<PathBuf>,

    /// Print this service's contract and exit.
    #[arg(long)]
    topics: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("hello-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let dir = contracts_dir(args.config.as_deref());
    let registry = TopicRegistry::load(&dir.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;
    let contract =
        ServiceContract::load(&dir.join("services/hello-service.toml")).map_err(|e| e.to_string())?;
    contract.validate(&registry).map_err(|e| e.to_string())?;

    if args.topics {
        print!("{}", contract.render());
        return Ok(());
    }

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let writer = node
        .writer::<HelloWorldTopic>("HelloWorldTopic")
        .map_err(|e| e.to_string())?;

    let period = Duration::from_millis(args.rate_ms.max(1));
    let start = Instant::now();
    let mut greeting = greeting::GreetingCycle::new(start);
    loop {
        let now = Instant::now();
        greeting.advance_if_due(now);
        let sample = HelloWorldTopic {
            greeting: greeting.current().to_string(),
            attitude: attitude::attitude_at(start.elapsed()),
        };
        writer.write(&sample).map_err(|e| e.to_string())?;
        std::thread::sleep(period);
    }
}

/// The contracts directory: an explicit `--config`, else `<exe_dir>/../config`,
/// else `./contracts` for a development build.
fn contracts_dir(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    let deployed = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .and_then(|bin| bin.parent().map(|deploy| deploy.join("config")));
    if let Some(dir) = &deployed {
        if dir.join("topics.toml").is_file() {
            return dir.clone();
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("contracts")
}
