//! The hello_world view service: merges the global model with this display's
//! local view state and publishes `HelloWorldViewTopic` for the display library.
//!
//! The relay republishes the latest model sample at a fixed rate. If no model
//! sample arrives for two seconds, the view data is marked invalid and the
//! greeting reports the time since the last model transmission.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::Parser;
use single_instance::Acquire;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::dds::Node;
use stations_dds::hello_world::{HelloWorldCtlTopic, HelloWorldTopic, HelloWorldViewTopic};

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

    /// Contracts directory (default: `the deploy root's config`, else `./contracts`).
    #[arg(long)]
    config: Option<PathBuf>,

    /// Numeric display id this service renders for.
    #[arg(long)]
    display_id: Option<u32>,

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

    let display_id = args
        .display_id
        .ok_or_else(|| "--display-id is required".to_string())?;

    let _guard = match single_instance::acquire(&format!("hello-view-service-display-{display_id}"))
    {
        Acquire::Primary(guard) => guard,
        Acquire::Duplicate => return Err("an instance is already running".to_string()),
    };

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let writer = node
        .writer::<HelloWorldViewTopic>("HelloWorldViewTopic")
        .map_err(|e| e.to_string())?;
    let model_reader = node
        .reader::<HelloWorldTopic>("HelloWorldTopic")
        .map_err(|e| e.to_string())?;
    let control_reader = node
        .reader::<HelloWorldCtlTopic>("HelloWorldCtlTopic")
        .map_err(|e| e.to_string())?;

    let period = Duration::from_millis(args.rate_ms.max(1));
    let mut relay = relay::ViewRelay::new(Instant::now(), display_id);
    loop {
        let now = Instant::now();
        match model_reader.take() {
            Ok(samples) => {
                if let Some(sample) = samples.into_iter().last() {
                    relay.update(sample, now);
                }
            }
            Err(e) => eprintln!("hello-view-service: model reader: {e}"),
        }
        match control_reader.take() {
            Ok(samples) => {
                for sample in samples {
                    relay.control(&sample, now);
                }
            }
            Err(e) => eprintln!("hello-view-service: control reader: {e}"),
        }
        if let Some(view) = relay.view(now) {
            writer.write(&view).map_err(|e| e.to_string())?;
        }
        std::thread::sleep(period);
    }
}

/// The contracts directory: an explicit `--config`, else `the deploy root's config`,
/// else `./contracts` for a development build.
fn contracts_dir(explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return path.to_path_buf();
    }
    if let Some(dir) = deploy_paths::config_dir() {
        if dir.join("topics.toml").is_file() {
            return dir;
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("contracts")
}
