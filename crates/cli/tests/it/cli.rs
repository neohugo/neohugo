//! The command line itself: `version`, `--help`, usage errors, flags before the command, the Go
//! build's logging and housekeeping flags, `config`.

use std::ffi::OsString;

use ::neohugo::Cli;
use ::neohugo::args::command_first;
use ::neohugo::version::BuildInfo;
use clap::Parser as _;

use crate::{neohugo, site_from, stderr, stdout};

#[test]
fn version_help_and_usage_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let o = neohugo(dir.path(), &["version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    let line = format!("{}\n", BuildInfo::CURRENT);
    assert_eq!(stdout(&o), line);
    let prefix = format!("neohugo v{}", env!("CARGO_PKG_VERSION"));
    assert!(line.starts_with(&prefix), "{line}");
    assert!(line.contains(" BuildDate="), "{line}");
    let o = neohugo(dir.path(), &["--version"], &[]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(stdout(&o), line);
    let o = neohugo(dir.path(), &["--help"], &[]);
    assert_eq!(o.status.code(), Some(0));
    for cmd in ["build", "server", "templates", "config", "version"] {
        assert!(stdout(&o).contains(cmd), "{cmd}: {}", stdout(&o));
    }
    let o = neohugo(dir.path(), &["templates", "check", "--help"], &[]);
    assert!(stdout(&o).contains("--coverage"), "{}", stdout(&o));
    for bad in [
        &["--nope"][..],
        &["build", "--clock", "yesterday"],
        &["templates"],
        &["templates", "check", "--coverage", "some"],
        &["config", "--format", "xml"],
    ] {
        let o = neohugo(dir.path(), bad, &[]);
        assert_eq!(o.status.code(), Some(2), "{bad:?}");
        assert!(stderr(&o).contains("error"), "{bad:?}: {}", stderr(&o));
    }
}

/// `version` prints Go's `BuildVersionString` (`common/neohugo/version.go` at 44529028).
#[test]
fn version_line_has_the_go_format() {
    let mut info = BuildInfo {
        version: "0.150.0",
        commit: None,
        os: "linux",
        arch: "amd64",
        date: None,
        vendor: None,
    };
    assert_eq!(
        info.to_string(),
        "neohugo v0.150.0 linux/amd64 BuildDate=unknown"
    );
    info.commit = Some("0123456789abcdef0123456789abcdef01234567");
    info.os = "darwin";
    info.arch = "arm64";
    info.date = Some("2026-10-01T12:34:56Z");
    info.vendor = Some("neohugo");
    assert_eq!(
        info.to_string(),
        "neohugo v0.150.0-0123456789abcdef0123456789abcdef01234567 darwin/arm64 \
         BuildDate=2026-10-01T12:34:56Z VendorInfo=neohugo"
    );

    // Go's GOOS/GOARCH names of the release targets.
    let current = BuildInfo::CURRENT;
    let expected = if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(("linux", "amd64"))
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some(("linux", "arm64"))
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(("darwin", "amd64"))
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(("darwin", "arm64"))
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(("windows", "amd64"))
    } else {
        None
    };
    if let Some(os_arch) = expected {
        assert_eq!((current.os, current.arch), os_arch);
    }
    assert_eq!(current.version, env!("CARGO_PKG_VERSION"));
}

#[test]
fn config_prints_the_resolved_configuration() {
    let s = site_from(
        "-- neohugo.toml --\nbaseURL = \"https://e.org/\"\ntitle = \"T\"\n-- config/production/params.toml --\ncolor = \"red\"\n",
    );
    let o = neohugo(s.path(), &["config"], &[("NEOHUGO_PARAMS_SIZE", "9")]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).expect("json");
    assert_eq!(v["environment"], "production");
    let site = &v["sites"][0];
    assert_eq!(site["title"], "T", "{site}");
    let text = stdout(&o);
    assert!(text.contains("\"color\": \"red\""), "{text}");
    assert!(text.contains("\"size\": \"9\""), "{text}");

    let o = neohugo(
        s.path(),
        &["config", "-e", "dev", "--base-url", "https://b.org/"],
        &[],
    );
    let text = stdout(&o);
    assert!(text.contains("https://b.org/"), "{text}");
    assert!(!text.contains("\"color\""), "{text}");

    let o = neohugo(s.path(), &["config", "--format", "toml"], &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        stdout(&o).contains("environment = \"production\""),
        "{}",
        stdout(&o)
    );
}

/// Flags before the command, as the Go build (cobra) reads them: `command_first` moves the
/// command to the front, a flag keeps its value.
#[test]
fn command_first_moves_the_command_before_the_flags() {
    let first = |args: &[&str]| -> Vec<String> {
        command_first(args.iter().map(OsString::from).collect())
            .into_iter()
            .map(|a| a.into_string().expect("UTF-8"))
            .collect()
    };
    for (given, want) in [
        (
            &["neohugo", "-s", "site", "server", "-D"][..],
            &["neohugo", "server", "-s", "site", "-D"][..],
        ),
        (
            &["neohugo", "--environment=production", "--minify", "build"],
            &["neohugo", "build", "--environment=production", "--minify"],
        ),
        (
            &[
                "neohugo",
                "-e",
                "production",
                "templates",
                "-s",
                "x",
                "check",
            ],
            &[
                "neohugo",
                "templates",
                "check",
                "-e",
                "production",
                "-s",
                "x",
            ],
        ),
        // Short clusters: a short that takes a value takes the rest or the next argument.
        (
            &["neohugo", "-DEs", "server", "config"],
            &["neohugo", "config", "-DEs", "server"],
        ),
        (
            &["neohugo", "-sserver", "config"],
            &["neohugo", "config", "-sserver"],
        ),
        // `=` ends a cluster: what follows is a value, not shorts (`t` and `e` take values).
        (
            &["neohugo", "-D=t", "-E=True", "server"],
            &["neohugo", "server", "-D=t", "-E=True"],
        ),
        // A flag of a command (`server`'s `--port`) keeps its value too.
        (
            &["neohugo", "--port", "1314", "serve"],
            &["neohugo", "serve", "--port", "1314"],
        ),
        // pflag's explicit values of the boolean flags that set no configuration key: `=true` is
        // the flag, `=false` none. The others (`-D`, `--minify`, `--watch`, …) take `=BOOL`
        // themselves.
        (
            &[
                "neohugo",
                "--gc=true",
                "server",
                "-M=false",
                "--quiet=1",
                "-D=false",
                "--minify=0",
                "--watch=false",
            ],
            &[
                "neohugo",
                "server",
                "--gc",
                "--quiet",
                "-D=false",
                "--minify=0",
                "--watch=false",
            ],
        ),
    ] {
        assert_eq!(first(given), want, "{given:?}");
    }
    // Unchanged: a value that names a command, `=`-only values, no command, `--`, an unknown
    // command, a command after a command without subcommands.
    for args in [
        &["neohugo", "-e", "server"][..],
        &["neohugo", "server", "--append-port", "false"],
        &["neohugo", "-s", "site", "-D"],
        &["neohugo", "--", "server"],
        &["neohugo", "-s", "site", "nope"],
        &["neohugo", "build", "--minify", "server"],
        &["neohugo", "--minify=maybe", "-e=x"],
    ] {
        assert_eq!(first(args), args, "{args:?}");
    }
}

/// The binary takes flags before the command, and every command takes the Go build's persistent
/// flags (`-s`, `-d`, `-e`, `--config`, `--config-dir`, `--themes-dir`, `--clock`, `-q`, `-M`).
#[test]
fn persistent_flags_anywhere() {
    let s = site_from("-- neohugo.toml --\ntitle = \"T\"\n");
    let o = neohugo(s.path(), &["-s", ".", "-e", "staging", "config"], &[]);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let v: serde_json::Value = serde_json::from_str(&stdout(&o)).expect("json");
    assert_eq!(v["environment"], "staging");
    for args in [
        &[
            "config",
            "-s",
            ".",
            "--quiet",
            "-d",
            "out",
            "-M",
            "--clock",
            "2026-01-01T00:00:00Z",
        ][..],
        &[
            "--config",
            "neohugo.toml",
            "--config-dir",
            "config",
            "templates",
            "check",
            "--coverage",
            "none",
        ],
        &["-q", "version", "-s", ".", "--themes-dir", "themes"],
    ] {
        let o = neohugo(s.path(), args, &[]);
        assert_eq!(o.status.code(), Some(0), "{args:?}: {}", stderr(&o));
    }
    // A flag the command does not take is still an error, before the command or after it.
    for args in [&["--minify", "version"][..], &["version", "--minify"]] {
        let o = neohugo(s.path(), args, &[]);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert!(stderr(&o).contains("--minify"), "{args:?}: {}", stderr(&o));
    }
}

/// The Go build's logging and housekeeping flags are accepted (`args::HugoFlags`); those neohugo
/// does not act on give a warning.
#[test]
fn hugo_flags_are_accepted() {
    let s = site_from("-- neohugo.toml --\ntitle = \"T\"\n");
    let o = neohugo(
        s.path(),
        &[
            "--gc",
            "--minify",
            "--logLevel",
            "info",
            "--noBuildLock",
            "--printI18nWarnings",
            "--printPathWarnings",
            "--printUnusedTemplates",
            "--templateMetrics",
            "--templateMetricsHints",
        ],
        &[],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let err = stderr(&o);
    for flag in [
        "--gc ",
        "--logLevel info ",
        "--printI18nWarnings ",
        "--printUnusedTemplates ",
        "--templateMetrics ",
        "--templateMetricsHints ",
    ] {
        assert!(
            err.contains(&format!("WARN  [ignored-flag]: {flag}is ignored")),
            "{flag}: {err}"
        );
    }
    for flag in ["--noBuildLock", "--printPathWarnings", "--minify"] {
        assert!(!err.contains(flag), "{flag}: {err}");
    }
    // What neohugo does anyway: no warning.
    let o = neohugo(
        s.path(),
        &[
            "build",
            "--logLevel",
            "WARNING",
            "--noBuildLock",
            "--printPathWarnings",
        ],
        &[],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(!stderr(&o).contains("ignored-flag"), "{}", stderr(&o));
    // `--logLevel` and `--noBuildLock` were persistent flags: every command takes them.
    let o = neohugo(
        s.path(),
        &["config", "--logLevel", "error", "--noBuildLock"],
        &[],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    let server = [
        "neohugo",
        "--gc",
        "server",
        "--noTimes",
        "--printI18nWarnings",
    ];
    let args = command_first(server.iter().map(OsString::from).collect());
    assert!(Cli::try_parse_from(args).is_ok(), "{server:?}");
    // Explicit values of boolean flags (pflag), and the empty level (Go's default).
    let o = neohugo(
        s.path(),
        &[
            "--gc=false",
            "--minify=true",
            "--quiet=TRUE",
            "--logLevel",
            "",
        ],
        &[],
    );
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(!stderr(&o).contains("ignored-flag"), "{}", stderr(&o));
    assert!(!stdout(&o).contains("Total in"), "{}", stdout(&o));
    // The flags that take `=BOOL` themselves read it with Go's `strconv.ParseBool` spellings;
    // `=false` is kept (it overrides the configuration).
    let parse = |args: &[&str]| {
        let args = command_first(args.iter().map(OsString::from).collect());
        Cli::try_parse_from(&args).unwrap_or_else(|e| panic!("{args:?}: {e}"))
    };
    let cli = parse(&[
        "neohugo",
        "-DE=f",
        "--buildFuture=FALSE",
        "--minify=0",
        "--ignoreCache=T",
        "--cleanDestinationDir=False",
        "--noTimes=1",
        "--noChmod=t",
    ]);
    let (b, p) = (&cli.build, &cli.build.project);
    assert_eq!(
        (p.include.build_drafts, p.include.build_expired),
        (Some(true), Some(false))
    );
    assert_eq!(
        (p.include.build_future, b.minify),
        (Some(false), Some(false))
    );
    assert_eq!(
        (p.ignore_cache, b.output.clean_destination_dir),
        (Some(true), Some(false))
    );
    assert_eq!(
        (b.output.no_times, b.output.no_chmod),
        (Some(true), Some(true))
    );
    let cli = parse(&["neohugo"]);
    assert_eq!(
        (cli.build.project.include.build_drafts, cli.build.minify),
        (None, None)
    );
    let Some(::neohugo::args::Command::Server(server)) =
        parse(&["neohugo", "server", "--watch=0", "--appendPort=F", "-DEF"]).command
    else {
        panic!("server");
    };
    assert!(!server.watch.watch && !server.listen.append_port);
    assert_eq!(server.build.project.include.build_future, Some(true));
    // An unknown level, and a build flag of a command that does not build.
    for bad in [&["--logLevel", "loud"][..], &["config", "--gc"], &["-v"]] {
        let o = neohugo(s.path(), bad, &[]);
        assert_eq!(o.status.code(), Some(2), "{bad:?}: {}", stderr(&o));
    }
}

/// `--noTimes` and `--noChmod` (config `noTimes`, `noChmod`): the static copy leaves the copies'
/// modification times and permissions alone.
#[cfg(unix)]
#[test]
fn no_times_and_no_chmod_reach_the_static_copy() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, SystemTime};

    let s = site_from("-- neohugo.toml --\ntitle = \"T\"\n-- static/a.txt --\nhi\n");
    let src = s.path().join("static/a.txt");
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(978_307_200);
    std::fs::File::options()
        .write(true)
        .open(&src)
        .and_then(|f| f.set_modified(old))
        .expect("set mtime");
    std::fs::set_permissions(&src, std::fs::Permissions::from_mode(0o604)).expect("chmod");
    let copied = |args: &[&str]| {
        let _ = std::fs::remove_dir_all(s.path().join("public"));
        let o = neohugo(s.path(), args, &[]);
        assert_eq!(o.status.code(), Some(0), "{args:?}: {}", stderr(&o));
        let m = std::fs::metadata(s.path().join("public/a.txt")).expect("copied");
        (
            m.modified().expect("mtime") == old,
            m.permissions().mode() & 0o777 == 0o604,
        )
    };
    assert_eq!(copied(&["build"]), (true, true));
    assert_eq!(copied(&["build", "--noTimes", "--noChmod"]), (false, false));
    assert_eq!(copied(&["build", "--no-times"]), (false, true));
}
