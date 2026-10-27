//! The hello_world view service: subscribes to `HelloWorldTopic` and relays it
//! as `HelloWorldViewTopic` for the display library to render.
//!
//! The relay republishes the latest model sample at a fixed rate. If no model
//! sample arrives for two seconds, the view data is marked invalid and the
//! greeting reports the time since the last model transmission.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::dds::Node;
use stations_dds::hello_world::{HelloWorldTopic, HelloWorldViewTopic};

mod relay;

#[derive(Parser, Debug)]
#[command(name = "hello-view-service", about = "hello_world view service")]
struct Args {
    /// DDS domain id.
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Relay publish period, in milliseconds.
    #[arg(long, default_value_t = 10)]
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
            eprintln!("hello-view-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let dir = contracts_dir(args.config.as_deref());
    let registry = TopicRegistry::load(&dir.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;
    let contract = ServiceContract::load(&dir.join("services/hello-view-service.toml"))
        .map_err(|e| e.to_string())?;
    contract.validate(&registry).map_err(|e| e.to_string())?;

    if args.topics {
        print!("{}", contract.render());
        return Ok(());
    }

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let writer = node
        .writer::<HelloWorldViewTopic>("HelloWorldViewTopic")
        .map_err(|e| e.to_string())?;
    let reader = node
        .reader::<HelloWorldTopic>("HelloWorldTopic")
        .map_err(|e| e.to_string())?;

    let period = Duration::from_millis(args.rate_ms.max(1));
    let mut relay = relay::ViewRelay::new(Instant::now());
    loop {
        let now = Instant::now();
        match reader.take() {
            Ok(samples) => {
                if let Some(sample) = samples.into_iter().last() {
                    relay.update(sample, now);
                }
            }
            Err(e) => eprintln!("hello-view-service: model reader: {e}"),
        }
        if let Some(view) = relay.view(now) {
            writer.write(&view).map_err(|e| e.to_string())?;
        }
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
