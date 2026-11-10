//! The format-manager controller service.
//!
//! The display library publishes logical format events. This service owns the
//! active format for each format window and publishes the full assignment set
//! on `FormatManagerCtlTopic` for the format-manager view service.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use format_list::{FormatList, FORMAT_COMMAND_NEXT, FORMAT_COMMAND_SELECT};
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::cyclonedds::DdsSequence;
use stations_dds::dds::Node;
use stations_dds::format_manager::{
    FormatManagerCtlTopic, FormatManagerInputTopic, FormatWindowAssignment,
};

const POLL: Duration = Duration::from_millis(10);

#[derive(Parser, Debug)]
#[command(
    name = "format-ctl-service",
    about = "format-manager controller service"
)]
struct Args {
    /// DDS domain id.
    #[arg(long, default_value_t = 0)]
    domain: u32,

    /// Contracts directory (default: `<exe_dir>/../config`, else `./contracts`).
    #[arg(long)]
    config: Option<PathBuf>,

    /// Unique display id this service controls.
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
            eprintln!("format-ctl-service: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let dir = contracts_dir(args.config.as_deref());
    let registry = TopicRegistry::load(&dir.join("topics.toml")).map_err(|e| e.to_string())?;
    registry.validate().map_err(|e| e.to_string())?;
    let contract = ServiceContract::load(&dir.join("services/format-ctl-service.toml"))
        .map_err(|e| e.to_string())?;
    contract.validate(&registry).map_err(|e| e.to_string())?;
    let formats = FormatList::load(&dir.join("formats.toml")).map_err(|e| e.to_string())?;

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
        .reader::<FormatManagerInputTopic>("FormatManagerInputTopic")
        .map_err(|e| e.to_string())?;
    let writer = node
        .writer::<FormatManagerCtlTopic>("FormatManagerCtlTopic")
        .map_err(|e| e.to_string())?;

    let mut assignments: BTreeMap<i32, String> = BTreeMap::new();
    loop {
        let samples = reader.take().map_err(|e| e.to_string())?;
        for sample in samples {
            let changed = apply_for(&display_id, &formats, &mut assignments, sample);
            if changed {
                let control = control_topic(&display_id, &assignments)?;
                writer.write(&control).map_err(|e| e.to_string())?;
            }
        }
        std::thread::sleep(POLL);
    }
}

fn control_topic(
    display_id: &str,
    assignments: &BTreeMap<i32, String>,
) -> Result<FormatManagerCtlTopic, String> {
    let windows = assignments
        .iter()
        .map(|(&window, format)| FormatWindowAssignment {
            window,
            format: format.clone(),
        })
        .collect();
    Ok(FormatManagerCtlTopic {
        display_id: display_id.to_string(),
        windows: DdsSequence::from_vec(windows).map_err(|e| e.to_string())?,
    })
}

/// Apply an event only when it is addressed to `display_id`.
fn apply_for(
    display_id: &str,
    formats: &FormatList,
    assignments: &mut BTreeMap<i32, String>,
    event: FormatManagerInputTopic,
) -> bool {
    if event.display_id != display_id {
        return false;
    }
    apply(formats, assignments, event)
}

fn apply(
    formats: &FormatList,
    assignments: &mut BTreeMap<i32, String>,
    event: FormatManagerInputTopic,
) -> bool {
    match event.command {
        FORMAT_COMMAND_NEXT => {
            let current = assignments
                .get(&event.window)
                .map(String::as_str)
                .or_else(|| formats.first());
            let Some(current) = current else {
                return false;
            };
            let Some(next) = formats.next(current) else {
                return false;
            };
            let changed = assignments.get(&event.window).map(String::as_str) != Some(next);
            assignments.insert(event.window, next.to_string());
            changed
        }
        FORMAT_COMMAND_SELECT => {
            if !formats.contains(&event.format) {
                return false;
            }
            let changed = assignments.get(&event.window) != Some(&event.format);
            assignments.insert(event.window, event.format);
            changed
        }
        _ => false,
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

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATS: &str = r#"
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
"#;

    fn formats() -> FormatList {
        FormatList::from_toml(FORMATS).unwrap()
    }

    fn input(display_id: &str, window: i32, command: i32, format: &str) -> FormatManagerInputTopic {
        FormatManagerInputTopic {
            display_id: display_id.to_string(),
            window,
            command,
            format: format.to_string(),
        }
    }

    #[test]
    fn next_advances_from_the_default_and_wraps() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        assert!(apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_NEXT, "")
        ));
        assert_eq!(assignments.get(&0).map(String::as_str), Some("default"));
        assert!(apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_NEXT, "")
        ));
        assert_eq!(assignments.get(&0).map(String::as_str), Some("hello-world"));
    }

    #[test]
    fn select_validates_and_applies() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        assert!(apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_SELECT, "default")
        ));
        assert_eq!(assignments.get(&0).map(String::as_str), Some("default"));
        assert!(!apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_SELECT, "missing")
        ));
    }

    #[test]
    fn windows_are_independent() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_SELECT, "default"),
        );
        apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 1, FORMAT_COMMAND_SELECT, "hello-world"),
        );
        assert_eq!(assignments.get(&0).map(String::as_str), Some("default"));
        assert_eq!(assignments.get(&1).map(String::as_str), Some("hello-world"));
    }

    #[test]
    fn unknown_command_is_ignored() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        assert!(!apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, 99, "default")
        ));
        assert!(assignments.is_empty());
    }

    #[test]
    fn other_display_events_are_ignored() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        assert!(!apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-b", 0, FORMAT_COMMAND_SELECT, "default")
        ));
        assert!(assignments.is_empty());
    }

    #[test]
    fn control_carries_the_display_id() {
        let formats = formats();
        let mut assignments = BTreeMap::new();
        apply_for(
            "lil-d-a",
            &formats,
            &mut assignments,
            input("lil-d-a", 0, FORMAT_COMMAND_SELECT, "default"),
        );
        let control = control_topic("lil-d-a", &assignments).unwrap();
        assert_eq!(control.display_id, "lil-d-a");
        assert_eq!(control.windows.as_slice()[0].format, "default");
    }
}
