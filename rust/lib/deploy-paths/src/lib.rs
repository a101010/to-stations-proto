use std::path::{Path, PathBuf};

pub fn deploy_root_for(executable: &Path) -> PathBuf {
    executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn deploy_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .map(|executable| deploy_root_for(&executable))
}

pub fn config_dir() -> Option<PathBuf> {
    deploy_root().map(|root| root.join("config"))
}

pub fn assets_dir() -> Option<PathBuf> {
    deploy_root().map(|root| root.join("assets"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_and_cpp_deploy_roots_match() {
        let root = Path::new("build/deploy");
        assert_eq!(deploy_root_for(&root.join("rust/bin/tool.exe")), root);
        assert_eq!(deploy_root_for(&root.join("cpp/bin/tool.exe")), root);
    }

    #[test]
    fn config_and_assets_are_at_the_deploy_root() {
        let root = Path::new("build/deploy");
        let executable = root.join("rust/bin/tool.exe");
        assert_eq!(
            deploy_root_for(&executable).join("config"),
            root.join("config")
        );
        assert_eq!(
            deploy_root_for(&executable).join("assets"),
            root.join("assets")
        );
    }

    #[test]
    fn short_paths_fall_back_to_dot() {
        assert_eq!(deploy_root_for(Path::new("tool.exe")), PathBuf::from("."));
    }
}
