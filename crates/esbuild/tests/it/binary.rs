//! The lookup of the esbuild binary (`binary_path`): the variable, next to the executable, `PATH`,
//! then the checkout's path.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use neohugo_esbuild::{BINARY_FILE, DEFAULT_BINARY, find_binary};

use crate::scratch;

/// `dir/<name>/esbuild` (an empty file) and its directory.
fn bin_dir(dir: &Path, name: &str) -> PathBuf {
    let d = dir.join(name);
    std::fs::create_dir_all(&d).expect("mkdir");
    std::fs::write(d.join(BINARY_FILE), "").expect("write");
    d
}

#[test]
fn binary_lookup_order() {
    let dir = scratch("binary_lookup_order");
    let exe = bin_dir(&dir, "exe");
    let on_path = bin_dir(&dir, "path");
    let empty = dir.join("empty");
    std::fs::create_dir_all(&empty).expect("mkdir");
    let joined = |dirs: &[&Path]| std::env::join_paths(dirs).expect("join_paths");
    let (only_path, both, relative, empty_only) = (
        joined(&[&on_path]),
        joined(&[&empty, &on_path]),
        joined(&[Path::new("."), Path::new("rel")]),
        joined(&[&empty]),
    );

    // The variable wins, unless empty.
    let var = OsStr::new("/opt/esbuild");
    assert_eq!(
        find_binary(Some(var), Some(&exe), Some(&only_path)),
        PathBuf::from("/opt/esbuild")
    );
    assert_eq!(
        find_binary(Some(OsStr::new("")), Some(&exe), None),
        exe.join(BINARY_FILE)
    );
    // Next to the executable, then the first PATH directory that has it.
    assert_eq!(
        find_binary(None, Some(&exe), Some(&only_path)),
        exe.join(BINARY_FILE)
    );
    assert_eq!(
        find_binary(None, Some(&empty), Some(&both)),
        on_path.join(BINARY_FILE)
    );
    // Relative PATH entries (the working directory) are not searched.
    assert_eq!(
        find_binary(None, None, Some(&relative)),
        PathBuf::from(DEFAULT_BINARY)
    );
    // Nothing found: the checkout's path.
    assert_eq!(
        find_binary(None, Some(&empty), Some(&empty_only)),
        PathBuf::from(DEFAULT_BINARY)
    );
    assert_eq!(find_binary(None, None, None), PathBuf::from(DEFAULT_BINARY));
}
