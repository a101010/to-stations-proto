//! The format-manager view service.
//!
//! It forwards the controller's format-window assignments to the display
//! library on `FormatManagerViewTopic`. It is a pass-through in the first
//! increment.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::dds::Node;
use stations_dds::format_manager::{FormatManagerCtlTopic, FormatManagerViewTopic};

const POLL: Duration = Duration::from_millis(10);

#[derive(Parser, Debug)]
#[command(name = "format-view-service", about = "format-manager view service")]
struct Args {
    /// DDS domain id.
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Contracts directory (default: `<exe_dir>/../config`, else `./contracts`).
    #[arg(long)]
    config: Option<PathBuf>,

    /// Unique display id this service forwards assignments for.
    #[arg(long)]
    display_id: Option<String>,

    /// Print this service's contract and exit.
    #[arg(long)]
    topics: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("format-view-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let dir = contracts_dir(args.config.as_deref());
    let registry = TopicRegistry::load(&dir.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;
    let contract = ServiceContract::load(&dir.join("services/format-view-service.toml"))
        .map_err(|e| e.to_string())?;
    contract.validate(&registry).map_err(|e| e.to_string())?;

    if args.topics {
        print!("{}", contract.render());
        return Ok(());
    }

    let display_id = args
        .display_id
        .clone()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| "--display-id is required".to_string())?;

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let reader = node
        .reader::<FormatManagerCtlTopic>("FormatManagerCtlTopic")
        .map_err(|e| e.to_string())?;
    let writer = node
        .writer::<FormatManagerViewTopic>("FormatManagerViewTopic")
        .map_err(|e| e.to_string())?;

    loop {
        let samples = reader.take().map_err(|e| e.to_string())?;
        for sample in samples {
            if let Some(view) = view_for(&display_id, sample) {
                writer.write(&view).map_err(|e| e.to_string())?;
            }
        }
        std::thread::sleep(POLL);
    }
}

/// Forward a control sample only when it is addressed to `display_id`.
fn view_for(display_id: &str, sample: FormatManagerCtlTopic) -> Option<FormatManagerViewTopic> {
    if sample.display_id != display_id {
        return None;
    }
    Some(FormatManagerViewTopic {
        display_id: display_id.to_string(),
        windows: sample.windows,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use stations_dds::cyclonedds::DdsSequence;
    use stations_dds::format_manager::FormatWindowAssignment;

    fn control(display_id: &str, format: &str) -> FormatManagerCtlTopic {
        FormatManagerCtlTopic {
            display_id: display_id.to_string(),
            windows: DdsSequence::from_vec(vec![FormatWindowAssignment {
                window: 0,
                format: format.to_string(),
            }])
            .unwrap(),
        }
    }

    #[test]
    fn matching_control_is_forwarded_with_identity() {
        let view = view_for("lil-d-a", control("lil-d-a", "default")).expect("forwarded");
        assert_eq!(view.display_id, "lil-d-a");
        assert_eq!(view.windows.as_slice()[0].format, "default");
    }

    #[test]
    fn other_display_control_is_ignored() {
        assert!(view_for("lil-d-a", control("lil-d-b", "default")).is_none());
    }
}
