//! Locate the bundled `sing-box(.exe)` binary and `wintun.dll` next to the
//! running app.

use std::path::{Path, PathBuf};

/// Located paths to the sing-box binary and the directory containing
/// `wintun.dll`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    pub singbox: PathBuf,
    pub wintun_dir: PathBuf,
}

/// Binary filename for the current platform: `sing-box.exe` on Windows,
/// `sing-box` elsewhere.
fn singbox_filename() -> &'static str {
    if cfg!(windows) {
        "sing-box.exe"
    } else {
        "sing-box"
    }
}

const WINTUN_FILENAME: &str = "wintun.dll";

/// Search `candidate_dirs` in order for both `sing-box(.exe)` and
/// `wintun.dll`, taking the first directory that has each (they need not be
/// the same directory). Factored out from [`locate_resources`] so it's
/// testable against injected temp dirs instead of the real filesystem.
fn search_dirs(candidate_dirs: &[PathBuf]) -> anyhow::Result<Resources> {
    let singbox_name = singbox_filename();
    let mut singbox = None;
    let mut wintun_dir = None;

    for dir in candidate_dirs {
        if singbox.is_none() {
            let candidate = dir.join(singbox_name);
            if candidate.is_file() {
                singbox = Some(candidate);
            }
        }
        if wintun_dir.is_none() {
            let candidate = dir.join(WINTUN_FILENAME);
            if candidate.is_file() {
                wintun_dir = Some(dir.clone());
            }
        }
        if singbox.is_some() && wintun_dir.is_some() {
            break;
        }
    }

    match (singbox, wintun_dir) {
        (Some(singbox), Some(wintun_dir)) => {
            tracing::info!(
                singbox = %singbox.display(),
                wintun_dir = %wintun_dir.display(),
                "locate_resources: resolved sing-box binary and wintun dir"
            );
            Ok(Resources {
                singbox,
                wintun_dir,
            })
        }
        _ => {
            tracing::warn!(
                searched = ?candidate_dirs,
                "locate_resources: could not find {singbox_name} and/or {WINTUN_FILENAME}"
            );
            anyhow::bail!(
                "could not locate {singbox_name} and/or {WINTUN_FILENAME} in any of {candidate_dirs:?}"
            )
        }
    }
}

/// Look for `sing-box(.exe)` and `wintun.dll` in, in order: the directory
/// named by `WISP_RESOURCE_DIR` (and its `resources/` subdir) — set by the
/// Tauri app to where it unpacked the bundled binaries — then next to the
/// current exe, in `./resources`, and in the cargo manifest's
/// `../../resources` (dev builds). Returns the first hit for each.
pub fn locate_resources() -> anyhow::Result<Resources> {
    let mut dirs = Vec::new();

    if let Some(res_dir) = std::env::var_os("WISP_RESOURCE_DIR") {
        let res_dir = PathBuf::from(res_dir);
        dirs.push(res_dir.join("resources"));
        dirs.push(res_dir);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("resources"));
            dirs.push(parent.to_path_buf());
        }
    }
    dirs.push(PathBuf::from("resources"));
    dirs.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources"));

    tracing::debug!(candidate_dirs = ?dirs, "locate_resources: searching candidate dirs");
    search_dirs(&dirs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_both_files_in_single_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let singbox_name = singbox_filename();
        fs::write(tmp.path().join(singbox_name), b"fake").expect("write singbox");
        fs::write(tmp.path().join(WINTUN_FILENAME), b"fake").expect("write wintun");

        let resources = search_dirs(&[tmp.path().to_path_buf()]).expect("should find both");
        assert_eq!(resources.singbox, tmp.path().join(singbox_name));
        assert_eq!(resources.wintun_dir, tmp.path().to_path_buf());
    }

    #[test]
    fn finds_files_split_across_dirs() {
        let tmp1 = tempfile::tempdir().expect("tempdir1");
        let tmp2 = tempfile::tempdir().expect("tempdir2");
        fs::write(tmp1.path().join(singbox_filename()), b"fake").expect("write singbox");
        fs::write(tmp2.path().join(WINTUN_FILENAME), b"fake").expect("write wintun");

        let resources = search_dirs(&[tmp1.path().to_path_buf(), tmp2.path().to_path_buf()])
            .expect("should find both across dirs");
        assert_eq!(resources.singbox, tmp1.path().join(singbox_filename()));
        assert_eq!(resources.wintun_dir, tmp2.path().to_path_buf());
    }

    #[test]
    fn errors_when_nothing_found() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = search_dirs(&[tmp.path().to_path_buf()]);
        assert!(result.is_err());
    }
}
