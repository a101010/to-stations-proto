//! The format-manager view service.
//!
//! It persists and forwards the controller's format-window assignments to the
//! display library on `FormatManagerViewTopic`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use format_list::{FormatList, SavedAssignment, SavedAssignments};
use single_instance::Acquire;
use stations_dds::contract::{ServiceContract, TopicRegistry};
use stations_dds::cyclonedds::DdsSequence;
use stations_dds::dds::Node;
use stations_dds::format_manager::{
    FormatManagerCtlTopic, FormatManagerViewTopic, FormatWindowAssignment,
};

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

    /// Numeric display id this service forwards assignments for.
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

    let formats = FormatList::load(&dir.join("formats.toml")).map_err(|e| e.to_string())?;
    let display_id = args
        .display_id
        .ok_or_else(|| "--display-id is required".to_string())?;

    let _guard = match single_instance::acquire(&format!(
        "format-view-service-display-{display_id}"
    )) {
        Acquire::Primary(guard) => guard,
        Acquire::Duplicate => return Err("an instance is already running".to_string()),
    };

    let state_path = app_state::state_path("format-view-service", &display_id.to_string());
    let saved = state_path
        .as_deref()
        .and_then(|path| app_state::load::<SavedAssignments>(path))
        .unwrap_or_else(SavedAssignments::hello_world)
        .validated(&formats);

    let node = Node::new(args.domain, registry).map_err(|e| e.to_string())?;
    let reader = node
        .reader::<FormatManagerCtlTopic>("FormatManagerCtlTopic")
        .map_err(|e| e.to_string())?;
    let writer = node
        .writer::<FormatManagerViewTopic>("FormatManagerViewTopic")
        .map_err(|e| e.to_string())?;

    writer
        .write(&view_topic(display_id, &saved)?)
        .map_err(|e| e.to_string())?;

    loop {
        let samples = reader.take().map_err(|e| e.to_string())?;
        for sample in samples {
            if sample.display_id != display_id {
                continue;
            }
            let saved = saved_from_control(&sample).validated(&formats);
            if let Some(path) = &state_path {
                if let Err(error) = app_state::save(path, &saved) {
                    eprintln!("format-view-service: state save failed: {error}");
                }
            }
            writer
                .write(&view_topic(display_id, &saved)?)
                .map_err(|e| e.to_string())?;
        }
        std::thread::sleep(POLL);
    }
}

fn saved_from_control(sample: &FormatManagerCtlTopic) -> SavedAssignments {
    SavedAssignments {
        windows: sample
            .windows
            .as_slice()
            .iter()
            .map(|assignment| SavedAssignment {
                window: assignment.window,
                format: assignment.format.clone(),
            })
            .collect(),
    }
}

fn view_topic(
    display_id: u32,
    saved: &SavedAssignments,
) -> Result<FormatManagerViewTopic, String> {
    let windows = saved
        .windows
        .iter()
        .map(|assignment| FormatWindowAssignment {
            window: assignment.window,
            format: assignment.format.clone(),
        })
        .collect();
    Ok(FormatManagerViewTopic {
        display_id,
        windows: DdsSequence::from_vec(windows).map_err(|e| e.to_string())?,
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

    fn control(display_id: u32, format: &str) -> FormatManagerCtlTopic {
        FormatManagerCtlTopic {
            display_id,
            windows: DdsSequence::from_vec(vec![FormatWindowAssignment {
                window: 0,
                format: format.to_string(),
            }])
            .unwrap(),
        }
    }

    fn formats() -> FormatList {
        FormatList::from_toml(
            r#"
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
"#,
        )
        .unwrap()
    }

    #[test]
    fn matching_control_becomes_a_view_with_identity() {
        let saved = saved_from_control(&control(0, "default")).validated(&formats());
        let view = view_topic(0, &saved).unwrap();
        assert_eq!(view.display_id, 0);
        assert_eq!(view.windows.as_slice()[0].format, "default");
    }

    #[test]
    fn unknown_formats_fall_back_to_hello_world() {
        let saved = saved_from_control(&control(0, "missing")).validated(&formats());
        let view = view_topic(0, &saved).unwrap();
        assert_eq!(view.windows.as_slice()[0].format, "hello-world");
    }
}
