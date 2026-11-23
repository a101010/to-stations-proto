//! The hello_world controller service.
//!
//! Display libraries publish local input on `HelloWorldInputTopic`. This service
//! filters it by display id and relays the controller's accepted input on
//! `HelloWorldCtlTopic` for that display's view service.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use single_instance::Acquire;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::dds::Node;
use stations_dds::hello_world::{HelloWorldCtlTopic, HelloWorldInputTopic};

const POLL: Duration = Duration::from_millis(10);

#[derive(Parser, Debug)]
#[command(name = "hello-ctl-service", about = "hello_world controller service")]
struct Args {
    /// DDS domain id.
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Contracts directory (default: `the deploy root's config`, else `./contracts`).
    #[arg(long)]
    config: Option<PathBuf>,

    /// Numeric display id this service controls.
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
            eprintln!("hello-ctl-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let dir = contracts_dir(args.config.as_deref());
    let registry = TopicRegistry::load(&dir.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;
    let contract = ServiceContract::load(&dir.join("services/hello-ctl-service.toml"))
        .map_err(|e| e.to_string())?;
    contract.validate(&registry).map_err(|e| e.to_string())?;

    if args.topics {
        print!("{}", contract.render());
        return Ok(());
    }

    let display_id = args
        .display_id
        .ok_or_else(|| "--display-id is required".to_string())?;

    let _guard = match single_instance::acquire(&format!("hello-ctl-service-display-{display_id}"))
    {
        Acquire::Primary(guard) => guard,
        Acquire::Duplicate => return Err("an instance is already running".to_string()),
    };

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let reader = node
        .reader::<HelloWorldInputTopic>("HelloWorldInputTopic")
        .map_err(|e| e.to_string())?;
    let writer = node
        .writer::<HelloWorldCtlTopic>("HelloWorldCtlTopic")
        .map_err(|e| e.to_string())?;

    loop {
        let samples = reader.take().map_err(|e| e.to_string())?;
        for sample in samples {
            if let Some(control) = control_for(&sample, display_id) {
                writer.write(&control).map_err(|e| e.to_string())?;
            }
        }
        std::thread::sleep(POLL);
    }
}

/// Relay input addressed to `display_id`; ignore other displays.
fn control_for(sample: &HelloWorldInputTopic, display_id: u32) -> Option<HelloWorldCtlTopic> {
    if sample.display_id != display_id {
        return None;
    }
    Some(HelloWorldCtlTopic {
        display_id,
        action: sample.action,
        x: sample.x,
        y: sample.y,
        z: sample.z,
        pressed: sample.pressed,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use format_list::{HELLO_ACTION_DRAG, HELLO_ACTION_GRAB};

    fn input(display_id: u32, action: i32) -> HelloWorldInputTopic {
        HelloWorldInputTopic {
            display_id,
            action,
            x: 0.25,
            y: -0.5,
            z: 0.0,
            pressed: true,
        }
    }

    #[test]
    fn matching_input_is_relayed() {
        let control = control_for(&input(0, HELLO_ACTION_GRAB), 0).expect("relayed");
        assert_eq!(control.display_id, 0);
        assert_eq!(control.action, HELLO_ACTION_GRAB);
        assert_eq!(control.x, 0.25);
        assert_eq!(control.y, -0.5);
        assert!(control.pressed);
    }

    #[test]
    fn other_displays_are_ignored() {
        assert!(control_for(&input(1, HELLO_ACTION_DRAG), 0).is_none());
    }
}
