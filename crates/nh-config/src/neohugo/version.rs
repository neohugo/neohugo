//! Port of `common/neohugo/version.go`, `common/neohugo/version_current.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


/// Go: `neohugo.CurrentVersion` = v0.149.0-DEV (the golden binary reports `v0.149.0-DEV`).
pub const CURRENT_VERSION: Version = Version { major: 0, minor: 149, patch_level: 0, suffix: "-DEV" };

/// Go: `neohugo.Version`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub patch_level: i32,
    pub suffix: &'static str,
}

impl Version {
    /// Go: `Version.String()` -> "0.149.0-DEV".
    pub fn string(&self) -> String {
        format!("{}.{}.{}{}", self.major, self.minor, self.patch_level, self.suffix)
    }
}

/// Go: `neohugo.BuildVersionString()` — `neohugo v0.149.0-DEV darwin/arm64 BuildDate=...` (stdout only).
// Go: common/neohugo/version.go:BuildVersionString
pub fn build_version_string() -> String {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/neohugo/version.go (299 lines; 4/21 funcs executed)
//   types: Version, VersionString
// EX L47-49: (v Version) String() string
// EX L52-54: (v Version) Version() VersionString
//    L57-59: (h Version) Compare(other any) int
//    L64-66: (h VersionString) String() string
//    L69-71: (h VersionString) Compare(other any) int
//    L73-75: (h VersionString) Version() Version
//    L78-84: (h VersionString) Eq(other any) bool
//    L89-101: ParseVersion(s string) (Version, error)
//    L105-111: MustParseVersion(s string) Version
//    L114-117: (v Version) ReleaseVersion() Version
//    L120-122: (v Version) Next() Version
//    L125-127: (v Version) Prev() Version
//    L131-135: (v Version) NextPatchLevel(level int) Version
// EX L139-172: BuildVersionString() string
// EX L174-179: version(major, minor, patch int, suffix string) string
//    L185-187: CompareVersion(version any) int
//    L189-236: compareVersions(inVersion Version, in any) int
//    L238-252: parseVersion(s string) (int, int, int)
//    L256-278: compareFloatWithVersion(v1 float64, v2 Version) int
//    L280-282: GoMinorVersion() int
//    L284-299: goMinorVersion(version string) int
// Source: common/neohugo/version_current.go (23 lines; 0/0 funcs executed)
// ---------------------------------------------------------------------------
