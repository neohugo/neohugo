// Builds the vendored libwebp 1.3.2 exactly like cgo builds
// github.com/bep/gowebp@v0.3.0/internal/libwebp:
//
//   cc -I <pkgdir> -fPIC -arch arm64 -pthread -fmessage-length=0 -fno-common \
//      -I $WORK/b002/ -O2 -g -I<gowebp>/libwebp_src -c <unit>.c
//
// (`go build -n`, go1.27.1 darwin/arm64, CGO_CFLAGS unset => cgo's default
// "-O2 -g"; a__cgo_src.go adds -I../../libwebp_src.) No -D defines: in
// particular WEBP_USE_THREAD and HAVE_CONFIG_H stay undefined.
//
// cgo compiles one stub per unit (`#include "../../libwebp_src/<unit>"`); we
// compile the unit directly. The unit list (libwebp_units.txt) is every .c
// file under libwebp_src/{src,sharpyuv}, which is exactly the stub set that
// gowebp's gen/main.go generates.
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let src_root = manifest.join("libwebp_src");
    let units_file = manifest.join("libwebp_units.txt");
    let units = fs::read_to_string(&units_file).expect("read libwebp_units.txt");

    let mut b = cc::Build::new();
    // Flags mirrored from the cgo command line. cc's own defaults
    // (-ffunction-sections, -fno-omit-frame-pointer, --target,
    // -mmacosx-version-min, profile-dependent -O) are disabled so that the
    // compiler invocation is the cgo one; -O2 is pinned in every cargo
    // profile.
    b.no_default_flags(true)
        .warnings(false)
        .extra_warnings(false);
    for f in cgo_default_flags() {
        b.flag(f);
    }
    b.flag("-O2").flag("-g").include(&src_root);
    arm64_parity(&mut b, false);

    for line in units.lines() {
        let unit = line.trim();
        if unit.is_empty() || unit.starts_with('#') {
            continue;
        }
        let path = src_root.join(unit);
        assert!(path.exists(), "missing libwebp unit {}", path.display());
        b.file(path);
    }
    // Port of the a__encoder.go cgo preamble.
    b.file(manifest.join("csrc/gowebp_encoder.c"));
    b.compile("gowebp_libwebp");

    // a__cgo.go: `#cgo unix LDFLAGS: -lm`
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rerun-if-changed=libwebp_units.txt");
    println!("cargo:rerun-if-changed=csrc/gowebp_encoder.c");
    println!("cargo:rerun-if-changed=libwebp_src");
    println!("cargo:rerun-if-changed=build.rs");
}

/// The flags cgo puts in front of every C compile (go1.27.1
/// `cmd/go/internal/work` gccCmd): `-fPIC`, the arch flag, `-pthread`,
/// `-fmessage-length=0`, `-fno-common` (darwin).
fn cgo_default_flags() -> Vec<&'static str> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let mut v = vec!["-fPIC"];
    match (os.as_str(), arch.as_str()) {
        ("macos", "aarch64") => v.extend(["-arch", "arm64"]),
        ("macos", "x86_64") => v.extend(["-arch", "x86_64"]),
        (_, "x86_64") => v.push("-m64"),
        _ => {}
    }
    v.extend(["-pthread", "-fmessage-length=0"]);
    if os == "macos" {
        v.push("-fno-common");
    }
    v
}

/// darwin/arm64 parity on other hosts. The golden was built by Apple clang on
/// arm64, where clang's default `-ffp-contract=on` fuses `a*b+c` within one
/// expression into an FMA instruction, and C++ call arguments are evaluated
/// left to right. Elsewhere:
///
/// * gcc evaluates arguments right to left (LibSass error columns change)
///   and contracts across statements (`-ffp-contract=fast`, and `on` means
///   `off` before gcc 14), so off Apple we compile with clang unless the
///   user set CC/CXX;
/// * x86_64 has no FMA in the baseline ISA, so clang lowers the fused
///   operations to a multiply and an add, and the float results (libwebp's
///   encoder decisions) differ; `-mfma` restores them.
///
/// Verified on linux/x86_64 (clang 18): every checked-in darwin/arm64 fixture
/// matches; with gcc 13, or with clang without `-mfma`, some do not.
/// `NEOHUGO_NO_FMA=1` drops `-mfma` for CPUs without FMA3, at the cost of
/// rare output differences.
fn arm64_parity(b: &mut cc::Build, cpp: bool) {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let target = env::var("TARGET").unwrap_or_default();
    let var = if cpp { "CXX" } else { "CC" };
    let vars = [
        var.to_string(),
        format!("{var}_{target}"),
        format!("{var}_{}", target.replace('-', "_")),
    ];
    for v in &vars {
        println!("cargo:rerun-if-env-changed={v}");
    }
    println!("cargo:rerun-if-env-changed=NEOHUGO_NO_FMA");
    if os != "macos" && vars.iter().all(|v| env::var_os(v).is_none()) {
        let clang = if cpp { "clang++" } else { "clang" };
        if Command::new(clang)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
        {
            b.compiler(clang);
        }
    }
    if b.get_compiler().is_like_clang() {
        b.flag("-ffp-contract=on");
    } else {
        println!(
            "cargo:warning={var} is not clang: output may differ from the darwin/arm64 golden (see PORTING.md)"
        );
    }
    if arch == "x86_64" && env::var_os("NEOHUGO_NO_FMA").is_none() {
        b.flag("-mfma");
    }
}
