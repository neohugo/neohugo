//! Port of `common/neohugo/version.go`, `common/neohugo/version_current.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::any::Any;
use std::borrow::Cow;

use go_value::{GoString, HostCtx, Kind, Object, Value};

/// Go: `neohugo.CurrentVersion` = v0.149.0-DEV (the golden binary reports `v0.149.0-DEV`).
pub const CURRENT_VERSION: Version = Version {
    major: 0,
    minor: 149,
    patch_level: 0,
    suffix: "-DEV",
};

/// Go: `neohugo.Version`: the Hugo build version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Version {
    pub major: i64,
    pub minor: i64,
    /// Increment this for bug releases
    pub patch_level: i64,
    /// HugoVersionSuffix is the suffix used in the Hugo version string. It will be blank for
    /// release versions (Go parses only `-test` and `-DEV`).
    pub suffix: &'static str,
}

/// Go: `versionSuffixes`.
const VERSION_SUFFIXES: [&str; 2] = ["-test", "-DEV"];

impl Version {
    /// Go: `Version.String()` -> "0.149.0-DEV" (patch level only for patch releases and
    /// versions after 0.53).
    // Go: common/neohugo/version.go:(Version).String
    pub fn string(&self) -> String {
        version(self.major, self.minor, self.patch_level, self.suffix)
    }

    /// Version returns the Hugo version.
    // Go: common/neohugo/version.go:(Version).Version
    pub fn version(&self) -> VersionString {
        VersionString(self.string())
    }

    /// Compare implements the compare.Comparer interface.
    // Go: common/neohugo/version.go:(Version).Compare
    pub fn compare(&self, other: &Value) -> i64 {
        compare_versions(*self, other)
    }

    /// ReleaseVersion represents the release version.
    // Go: common/neohugo/version.go:ReleaseVersion
    pub fn release_version(&self) -> Version {
        Version {
            suffix: "",
            ..*self
        }
    }

    /// Next returns the next Hugo release version.
    // Go: common/neohugo/version.go:Next
    pub fn next(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor.wrapping_add(1),
            ..Default::default()
        }
    }

    /// Prev returns the previous Hugo release version.
    // Go: common/neohugo/version.go:Prev
    pub fn prev(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor.wrapping_sub(1),
            ..Default::default()
        }
    }

    /// NextPatchLevel returns the next patch/bugfix Hugo version. This will be a patch
    /// increment on the previous Hugo version.
    // Go: common/neohugo/version.go:NextPatchLevel
    pub fn next_patch_level(&self, level: i64) -> Version {
        let mut prev = self.prev();
        prev.patch_level = level;
        prev
    }
}

/// `neohugo.Version` as a Go value.
impl Object for Version {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("neohugo.Version")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(
            name,
            "String"
                | "Version"
                | "Compare"
                | "ReleaseVersion"
                | "Next"
                | "Prev"
                | "NextPatchLevel"
        )
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        Some(Ok(match name {
            "String" => Value::string(self.string()),
            "Version" => Value::object(self.version()),
            "Compare" => match args {
                [a] => Value::int(self.compare(a)),
                _ => {
                    return Some(Err(go_value::Error::new(
                        "wrong number of args for Compare",
                    )));
                }
            },
            "ReleaseVersion" => Value::object(self.release_version()),
            "Next" => Value::object(self.next()),
            "Prev" => Value::object(self.prev()),
            "NextPatchLevel" => match args {
                [Value::Int(i, _)] => Value::object(self.next_patch_level(*i)),
                _ => {
                    return Some(Err(go_value::Error::new(
                        "wrong number of args for NextPatchLevel",
                    )));
                }
            },
            _ => return None,
        }))
    }
    fn field(&self, name: &str) -> Option<Value> {
        Some(match name {
            "Major" => Value::int(self.major),
            "Minor" => Value::int(self.minor),
            "PatchLevel" => Value::int(self.patch_level),
            "Suffix" => Value::string(self.suffix),
            _ => return None,
        })
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.string()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `neohugo.VersionString`: a Hugo version string (a named string type).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionString(pub String);

impl VersionString {
    // Go: common/neohugo/version.go:(VersionString).String
    pub fn string(&self) -> &str {
        &self.0
    }

    /// Compare implements the compare.Comparer interface.
    // Go: common/neohugo/version.go:(VersionString).Compare
    pub fn compare(&self, other: &Value) -> i64 {
        compare_versions(self.version(), other)
    }

    // Go: common/neohugo/version.go:(VersionString).Version
    pub fn version(&self) -> Version {
        must_parse_version(&self.0)
    }

    /// Eq implements the compare.Eqer interface.
    // Go: common/neohugo/version.go:(VersionString).Eq
    pub fn eq_value(&self, other: &Value) -> bool {
        match nh_common::cast::caste::to_string_e(other) {
            Ok(s) => s.as_bytes() == self.0.as_bytes(),
            Err(_) => false,
        }
    }
}

impl Object for VersionString {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("neohugo.VersionString")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "String" | "Compare" | "Version" | "Eq")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        let one = |args: &[Value]| -> go_value::Result<Value> {
            match args {
                [a] => Ok(a.clone()),
                _ => Err(go_value::Error::new(format!(
                    "wrong number of args for {name}: want 1 got {}",
                    args.len()
                ))),
            }
        };
        Some(match name {
            "String" => Ok(Value::string(self.0.as_str())),
            "Compare" => one(args).map(|a| Value::int(self.compare(&a))),
            "Version" => Ok(Value::object(self.version())),
            "Eq" => one(args).map(|a| Value::Bool(self.eq_value(&a))),
            _ => return None,
        })
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.as_str()))
    }
    fn underlying(&self) -> Option<Value> {
        Some(Value::string(self.0.as_str()))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// ParseVersion parses a version string.
// Go: common/neohugo/version.go:ParseVersion
pub fn parse_version(s: &str) -> Version {
    let mut vv = Version::default();
    let mut s = s;
    for suffix in VERSION_SUFFIXES {
        if let Some(t) = s.strip_suffix(suffix) {
            vv.suffix = suffix;
            s = t;
        }
    }

    let (major, minor, patch) = parse_version_parts(s);
    vv.major = major;
    vv.minor = minor;
    vv.patch_level = patch;

    vv
}

/// MustParseVersion parses a version string (it cannot fail).
// Go: common/neohugo/version.go:MustParseVersion
pub fn must_parse_version(s: &str) -> Version {
    parse_version(s)
}

/// Build information of the running binary (Go: `debug.ReadBuildInfo()` + the linker vars
/// `buildDate` and `vendorInfo`).
#[derive(Clone, Debug, Default)]
pub struct BuildInfo {
    pub revision: String,
    pub revision_time: String,
    pub go_os: String,
    pub go_arch: String,
    pub build_date: String,
    pub vendor_info: String,
}

impl BuildInfo {
    /// The build information of this binary: Go's GOOS/GOARCH names for the target, and the
    /// VCS revision/time from the `NEOHUGO_VCS_REVISION`/`NEOHUGO_VCS_TIME` build environment.
    pub fn current() -> BuildInfo {
        let go_arch = match std::env::consts::ARCH {
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            "x86" => "386",
            other => other,
        };
        BuildInfo {
            revision: option_env!("NEOHUGO_VCS_REVISION")
                .unwrap_or("")
                .to_string(),
            revision_time: option_env!("NEOHUGO_VCS_TIME").unwrap_or("").to_string(),
            go_os: std::env::consts::OS.to_string(),
            go_arch: go_arch.to_string(),
            build_date: String::new(),
            vendor_info: String::new(),
        }
    }
}

/// Go: `neohugo.BuildVersionString()` — `neohugo v0.149.0-DEV darwin/arm64 BuildDate=...` (stdout only).
// Go: common/neohugo/version.go:BuildVersionString
pub fn build_version_string() -> String {
    build_version_string_from(Some(&BuildInfo::current()))
}

/// [`build_version_string`] for the given build info (`None` = Go's nil build info).
pub fn build_version_string_from(bi: Option<&BuildInfo>) -> String {
    let program = "neohugo";

    let mut version = format!("v{}", CURRENT_VERSION.string());

    let Some(bi) = bi else {
        return version;
    };
    if !bi.revision.is_empty() {
        version.push('-');
        version.push_str(&bi.revision);
    }

    let os_arch = format!("{}/{}", bi.go_os, bi.go_arch);

    let mut date = bi.revision_time.clone();
    if date.is_empty() {
        // Accept vendor-specified build date if .git/ is unavailable.
        date = bi.build_date.clone();
    }
    if date.is_empty() {
        date = "unknown".to_string();
    }

    let mut version_string = format!("{program} {version} {os_arch} BuildDate={date}");

    if !bi.vendor_info.is_empty() {
        version_string.push_str(" VendorInfo=");
        version_string.push_str(&bi.vendor_info);
    }

    version_string
}

// Go: common/neohugo/version.go:version
fn version(major: i64, minor: i64, patch: i64, suffix: &str) -> String {
    if patch > 0 || minor > 53 {
        return format!("{major}.{minor}.{patch}{suffix}");
    }
    format!("{major}.{minor}{suffix}")
}

/// CompareVersion compares the given version string or number against the running Hugo
/// version: -1 if the given version is less than, 0 if equal and 1 if greater than the running
/// version.
// Go: common/neohugo/version.go:CompareVersion
pub fn compare_version(version: &Value) -> i64 {
    compare_versions(CURRENT_VERSION, version)
}

// Go: common/neohugo/version.go:compareVersions
fn compare_versions(in_version: Version, input: &Value) -> i64 {
    match input {
        Value::Float(f, _) => compare_float_with_version(*f, in_version),
        Value::Int(
            i,
            go_value::IntKind::Int | go_value::IntKind::Int32 | go_value::IntKind::Int64,
        ) => compare_float_with_version(*i as f64, in_version),
        Value::Object(o) if o.as_any().is::<Version>() => {
            let d = *o.as_any().downcast_ref::<Version>().expect("checked");
            if d.major == in_version.major
                && d.minor == in_version.minor
                && d.patch_level == in_version.patch_level
            {
                return match in_version.suffix.cmp(d.suffix) {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                };
            }
            if d.major > in_version.major {
                return 1;
            } else if d.major < in_version.major {
                return -1;
            }
            if d.minor > in_version.minor {
                return 1;
            } else if d.minor < in_version.minor {
                return -1;
            }
            if d.patch_level > in_version.patch_level {
                return 1;
            } else if d.patch_level < in_version.patch_level {
                return -1;
            }
            0
        }
        _ => {
            let Ok(s) = nh_common::cast::caste::to_string_e(input) else {
                return -1;
            };
            let v = parse_version(&String::from_utf8_lossy(&s));
            in_version.compare(&Value::object(v))
        }
    }
}

// Go: common/neohugo/version.go:parseVersion
fn parse_version_parts(s: &str) -> (i64, i64, i64) {
    let parts: Vec<&str> = s.split('.').collect();
    // Go ignores Atoi's error but keeps its value: the clamped int for a number out of range
    // (e.g. `uint64` max from `compareVersions`' string conversion), 0 for bad syntax.
    let atoi = |p: &str| go_strconv::internal::atoi(p.as_bytes()).0;
    let major = parts.first().map(|p| atoi(p)).unwrap_or(0);
    let minor = parts.get(1).map(|p| atoi(p)).unwrap_or(0);
    let patch = parts.get(2).map(|p| atoi(p)).unwrap_or(0);
    (major, minor, patch)
}

/// compareFloatWithVersion compares v1 with v2: -1 if v1 is less than v2, 0 if equal and 1 if
/// greater.
// Go: common/neohugo/version.go:compareFloatWithVersion
fn compare_float_with_version(v1: f64, v2: Version) -> i64 {
    // Go: math.Modf.
    let (mf, minf) = if v1.is_infinite() {
        (v1, f64::NAN)
    } else {
        let t = v1.trunc();
        (t, v1 - t)
    };
    let v1maj = mf as i64;
    let v1min = (minf * 100.0) as i64;

    if v2.major == v1maj && v2.minor == v1min {
        return 0;
    }

    if v1maj > v2.major {
        return 1;
    }

    if v1maj < v2.major {
        return -1;
    }

    if v1min > v2.minor {
        return 1;
    }

    -1
}

/// Go: `GoMinorVersion()` for the Go version the golden build used (`go1.27.1`).
// Go: common/neohugo/version.go:GoMinorVersion
pub fn go_minor_version() -> i64 {
    go_minor_version_of(super::neohugo::GO_VERSION)
}

/// Go: `goMinorVersion(version)`: `fmt.Sscanf(version, "go%d.%d%s", ...)`.
// Go: common/neohugo/version.go:goMinorVersion
pub fn go_minor_version_of(version: &str) -> i64 {
    if version.starts_with("devel") {
        return 9999; // magic
    }
    let Some(rest) = version.strip_prefix("go") else {
        return 0;
    };
    let (Some(major_len), rest_all) = (scan_int_len(rest), rest) else {
        return 0;
    };
    let rest = &rest_all[major_len..];
    let Some(rest) = rest.strip_prefix('.') else {
        return 0;
    };
    let Some(minor_len) = scan_int_len(rest) else {
        return 0;
    };
    let minor: i64 = match rest[..minor_len].parse() {
        Ok(m) => m,
        Err(_) => return 0,
    };
    // A trailing %s reads the rest; with nothing left Sscanf reports EOF after 2 items,
    // which is accepted.
    minor
}

/// The length of a `%d` token (optional sign, then digits) at the start of `s`.
fn scan_int_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i = 1;
    }
    let start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == start {
        return None;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A number out of `int` range: Go's `parseVersion` keeps `strconv.Atoi`'s clamped value
    /// (and `Version` fields are Go `int`s), so `uint64` max is a version above any other
    /// (go1.27.1: `VersionString("0.105.0").Compare(uint64(math.MaxUint64))` is 1).
    #[test]
    fn compare_out_of_range_numbers() {
        let v = VersionString("0.105.0".into());
        let max = Value::Uint(u64::MAX, go_value::UintKind::Uint64);
        assert_eq!(v.compare(&max), 1);
        assert_eq!(
            parse_version("18446744073709551615.1").major,
            i64::MAX,
            "clamped"
        );
        assert_eq!(parse_version("-99999999999999999999").major, i64::MIN);
        assert_eq!(parse_version("x.2").minor, 2);
        assert_eq!(
            VersionString("0.99".into()).compare(&max),
            1,
            "0.99 < MaxInt64.0.0"
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/neohugo/version.go (299 lines; 4/21 funcs executed)
//   types: Version, VersionString
// OK L47-49: (v Version) String() string
// OK L52-54: (v Version) Version() VersionString
// OK L57-59: (h Version) Compare(other any) int
// OK L64-66: (h VersionString) String() string
// OK L69-71: (h VersionString) Compare(other any) int
// OK L73-75: (h VersionString) Version() Version
// OK L78-84: (h VersionString) Eq(other any) bool
// OK L89-101: ParseVersion(s string) (Version, error)
// OK L105-111: MustParseVersion(s string) Version
// OK L114-117: (v Version) ReleaseVersion() Version
// OK L120-122: (v Version) Next() Version
// OK L125-127: (v Version) Prev() Version
// OK L131-135: (v Version) NextPatchLevel(level int) Version
// OK L139-172: BuildVersionString() string
// OK L174-179: version(major, minor, patch int, suffix string) string
// OK L185-187: CompareVersion(version any) int
// OK L189-236: compareVersions(inVersion Version, in any) int
// OK L238-252: parseVersion(s string) (int, int, int)
// OK L256-278: compareFloatWithVersion(v1 float64, v2 Version) int
// OK L280-282: GoMinorVersion() int
// OK L284-299: goMinorVersion(version string) int
// Source: common/neohugo/version_current.go (23 lines; 0/0 funcs executed)
// ---------------------------------------------------------------------------
