//! Port of `$GOROOT/src/time/zoneinfo_unix.go` (go1.27.1, `unix && !ios && !android`).

use go_value::Location;

use crate::zoneinfo_read::{load_location_sources, runtime_goroot};

/// Go: `platformZoneSources`. Many systems use /usr/share/zoneinfo, Solaris 2
/// has /usr/share/lib/zoneinfo, IRIX 6 has /usr/lib/locale/TZ, NixOS has
/// /etc/zoneinfo.
pub(crate) static PLATFORM_ZONE_SOURCES: &[&str] = &[
    "/usr/share/zoneinfo/",
    "/usr/share/lib/zoneinfo/",
    "/usr/lib/locale/TZ/",
    "/etc/zoneinfo",
];

// Go: zoneinfo_unix.go:initLocal
/// The `Local` location from `$TZ` (unset: /etc/localtime; "" or "UTC": UTC).
pub(crate) fn init_local() -> Location {
    let tz = std::env::var_os("TZ").map(|v| v.to_string_lossy().into_owned());
    init_local_from(tz.as_deref())
}

/// [`init_local`] for an explicit `$TZ` value (`None` = unset).
pub fn init_local_from(tz: Option<&str>) -> Location {
    init_local_from_env(tz, runtime_goroot().as_deref())
}

/// [`init_local_from`] with an explicit `runtime.GOROOT()` value.
pub fn init_local_from_env(tz: Option<&str>, goroot: Option<&str>) -> Location {
    // consult $TZ to find the time zone to use.
    // no $TZ means use the system default /etc/localtime.
    // $TZ="" means use UTC.
    // $TZ="foo" or $TZ=":foo" if foo is an absolute path, then the file pointed
    // by foo will be used to initialize timezone; otherwise, file
    // /usr/share/zoneinfo/foo will be used.
    match tz {
        None => {
            if let Ok(z) = load_location_sources("localtime", &["/etc"], goroot) {
                let mut l = (*z).clone();
                l.name = "Local".to_string();
                return l;
            }
        }
        Some(tz) if !tz.is_empty() => {
            let mut tz = tz;
            if tz.as_bytes()[0] == b':' {
                tz = &tz[1..];
            }
            if !tz.is_empty() && tz.as_bytes()[0] == b'/' {
                if let Ok(z) = load_location_sources(tz, &[""], goroot) {
                    let mut l = (*z).clone();
                    if tz == "/etc/localtime" {
                        l.name = "Local".to_string();
                    } else {
                        l.name = tz.to_string();
                    }
                    return l;
                }
            } else if !tz.is_empty() && tz != "UTC" {
                if let Ok(z) = load_location_sources(tz, PLATFORM_ZONE_SOURCES, goroot) {
                    return (*z).clone();
                }
            }
        }
        Some(_) => {}
    }

    // Fall back to UTC.
    Location {
        name: "UTC".to_string(),
        zone: Vec::new(),
        tx: Vec::new(),
        extend: String::new(),
    }
}
