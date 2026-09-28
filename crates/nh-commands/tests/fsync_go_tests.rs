//! Ports of spf13/fsync v0.10.1 `fsync_test.go` (TestSync, TestDeleteFileFilter,
//! TestDeleteFileFilterNotSet) on the OS file system. `SyncTo(to, srcs...)` is
//! `Sync(join(to, base(src)), src)` for each source.

mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use nh_commands::fsync::{ERR_FILE_OVER_DIR, Syncer};
use support::TempDir;

fn os_syncer() -> Syncer {
    Syncer::new(
        Arc::new(nh_hugofs::afero::OsFs),
        Arc::new(nh_hugofs::afero::OsFs),
    )
}

fn p(dir: &TempDir, rel: &str) -> String {
    dir.path.join(rel).to_str().unwrap().to_string()
}

fn perms(path: &str) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

fn mtime(path: &str) -> SystemTime {
    std::fs::metadata(path).unwrap().modified().unwrap()
}

fn dir_count(path: &str) -> usize {
    std::fs::read_dir(path).unwrap().count()
}

fn set_time(path: &str, t: SystemTime) {
    std::fs::File::open(path).unwrap().set_modified(t).unwrap();
}

#[test]
fn test_sync() {
    let d = TempDir::new("fsync_test");
    std::fs::create_dir_all(p(&d, "src/a")).unwrap();
    std::fs::write(p(&d, "src/a/b"), "file b").unwrap();
    std::fs::write(p(&d, "src/c"), "file c").unwrap();
    // set times in the past to make sure times are synced, not accidentally the same
    let tt = SystemTime::now() - Duration::from_secs(3600);
    for f in ["src/a/b", "src/a", "src/c", "src"] {
        set_time(&p(&d, f), tt);
    }

    let mut s = os_syncer();
    // SyncTo("dst", "src/a", "src/c")
    s.sync(&p(&d, "dst/a"), &p(&d, "src/a")).unwrap();
    s.sync(&p(&d, "dst/c"), &p(&d, "src/c")).unwrap();

    assert_eq!(dir_count(&p(&d, "dst")), 2);
    assert_eq!(dir_count(&p(&d, "dst/a")), 1);
    assert_eq!(std::fs::read(p(&d, "dst/a/b")).unwrap(), b"file b");
    assert_eq!(std::fs::read(p(&d, "dst/c")).unwrap(), b"file c");
    for f in ["a", "a/b", "c"] {
        assert_eq!(
            perms(&p(&d, &format!("dst/{f}"))),
            perms(&p(&d, &format!("src/{f}")))
        );
        assert_eq!(
            mtime(&p(&d, &format!("dst/{f}"))),
            mtime(&p(&d, &format!("src/{f}")))
        );
    }

    // sync the parent directory too
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(perms(&p(&d, "dst")), perms(&p(&d, "src")));
    assert_eq!(mtime(&p(&d, "dst")), mtime(&p(&d, "src")));

    // modify src
    std::fs::write(p(&d, "src/a/b"), "file b changed").unwrap();
    std::fs::set_permissions(p(&d, "src/a"), std::fs::Permissions::from_mode(0o775)).unwrap();
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(std::fs::read(p(&d, "dst/a/b")).unwrap(), b"file b changed");
    assert_eq!(perms(&p(&d, "dst/a")), perms(&p(&d, "src/a")));
    for f in ["", "/a", "/a/b", "/c"] {
        assert_eq!(
            mtime(&p(&d, &format!("dst{f}"))),
            mtime(&p(&d, &format!("src{f}")))
        );
    }

    // remove c; sync: c should still exist
    std::fs::remove_file(p(&d, "src/c")).unwrap();
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(dir_count(&p(&d, "dst")), 2);
    assert!(Path::new(&p(&d, "dst/c")).exists());

    // with Delete: c should no longer exist
    s.delete = true;
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(dir_count(&p(&d, "dst")), 1);
    assert!(!Path::new(&p(&d, "dst/c")).exists());

    s.delete = false;
    let err = s.sync(&p(&d, "dst"), &p(&d, "src/a/b")).unwrap_err();
    assert_eq!(err.to_string(), ERR_FILE_OVER_DIR);
}

fn delete_filter_setup(d: &TempDir) {
    std::fs::create_dir_all(p(d, "src/a")).unwrap();
    std::fs::write(p(d, "src/a/b"), "file b").unwrap();
    std::fs::create_dir_all(p(d, "dst")).unwrap();
    std::fs::write(p(d, "dst/c"), "file c").unwrap();
    std::fs::write(p(d, "dst/d"), "file c").unwrap();
    assert_eq!(dir_count(&p(d, "dst")), 2);
}

#[test]
fn test_delete_file_filter() {
    let d = TempDir::new("fsync_test_delete_filter");
    delete_filter_setup(&d);
    let mut s = os_syncer();
    s.delete = true;
    s.delete_filter = Some(|name, _is_dir| name == "d");
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(dir_count(&p(&d, "dst")), 2);
    assert!(Path::new(&p(&d, "dst/a/b")).exists());
    assert!(!Path::new(&p(&d, "dst/c")).exists());
    assert!(Path::new(&p(&d, "dst/d")).exists());
}

#[test]
fn test_delete_file_filter_not_set() {
    let d = TempDir::new("fsync_test_delete_filter_not_set");
    delete_filter_setup(&d);
    let mut s = os_syncer();
    s.delete = true;
    s.sync(&p(&d, "dst"), &p(&d, "src")).unwrap();
    assert_eq!(dir_count(&p(&d, "dst")), 1);
    assert!(Path::new(&p(&d, "dst/a/b")).exists());
    assert!(!Path::new(&p(&d, "dst/c")).exists());
    assert!(!Path::new(&p(&d, "dst/d")).exists());
}
