//! Every zone of the system zoneinfo (the golden build's zone source) and of
//! `$GOROOT/lib/time/zoneinfo.zip`, compared with Go through digests
//! (tools/go-oracle/go-time/zoneall.go). Zones whose file (or zip) no longer
//! hashes like at fixture generation are skipped, so the test is machine
//! independent; it reports how many zones were compared.

mod common;

use std::fmt::Write as _;
use std::sync::Arc;

use common::*;
use go_time::{GoTimeExt, Location, Time};
use sha2::Digest;

fn zb(t: &Time) -> String {
    if t.go_is_zero() {
        "Z".to_string()
    } else {
        t.go_unix().to_string()
    }
}

/// Mirror of zoneall.go:zoneText.
fn zone_text(l: &Arc<Location>) -> String {
    let mut b = String::new();
    let look = |b: &mut String, s: i64| {
        let t = go_time::unix(s, 0).in_loc(l);
        let (name, off) = t.zone();
        let (st, en) = t.zone_bounds();
        writeln!(
            b,
            "L {} {} {} {} {} {}",
            s,
            name,
            off,
            t.is_dst(),
            zb(&st),
            zb(&en)
        )
        .unwrap();
    };
    for s in [
        i64::MIN,
        -1 << 40,
        -5000000000,
        -1,
        0,
        1790000000,
        4102444800,
        1 << 40,
        i64::MAX,
    ] {
        look(&mut b, s);
    }
    let utc = go_time::utc();
    let mut t = go_time::date(1800, go_time::Month(1), 1, 0, 0, 0, 0, &utc);
    for _ in 0..5000 {
        let (_, end) = t.in_loc(l).zone_bounds();
        if end.go_is_zero() || end.year() > 2080 {
            break;
        }
        if end.go_unix() <= t.go_unix() {
            // tzset's "end of year" bound (ystart+365 days) repeats in
            // leap years; step past it.
            t = t.add(go_time::Duration(24 * go_time::Duration::HOUR.0));
            continue;
        }
        let s = end.go_unix();
        look(&mut b, s - 1);
        look(&mut b, s);
        look(&mut b, s + 1);
        let lt = go_time::unix(s, 0).in_loc(l);
        let (y, m, d) = lt.date();
        let (h, mi, sec) = lt.clock();
        for dm in [-61, -60, -30, -1, 0, 1, 59, 60] {
            let dt = go_time::date(y, m, d, h, mi + dm, sec, 0, l);
            writeln!(
                b,
                "D {} {} {}",
                dm,
                dt.go_unix(),
                dt.format("2006-01-02 15:04:05 MST -07:00:00")
            )
            .unwrap();
        }
        t = end;
    }
    b
}

fn digest(s: &str) -> (usize, String) {
    (
        s.matches('\n').count(),
        hex(&sha2::Sha256::digest(s.as_bytes())),
    )
}

fn sha256_file(p: &str) -> String {
    match std::fs::read(p) {
        Ok(b) => hex(&sha2::Sha256::digest(&b)),
        Err(_) => "missing".to_string(),
    }
}

#[test]
fn zoneall_fixture() {
    setup();
    let recs = records("zoneall.txt");
    let mut c = Checker::new("zoneall");
    let mut zip_ok = false;
    let mut zip_path = String::new();
    let mut goroot: Option<String> = None;
    let (mut skipped_zip, mut skipped_sys) = (0, 0);
    for r in &recs {
        match r.tag() {
            b"ZH" => {
                zip_path = r.s(1);
                zip_ok = sha256_file(&zip_path) == r.s(2);
                goroot = zip_path
                    .strip_suffix("/lib/time/zoneinfo.zip")
                    .map(|s| s.to_string());
            }
            b"ZZ" => {
                if !zip_ok {
                    skipped_zip += 1;
                    continue;
                }
                let name = r.s(1);
                let res = go_time::load_location_env(&name, &zip_path, None);
                match (res, r.b(2)) {
                    (Err(e), b"ERR") => c.check(e.error().as_bytes() == r.b(3), || {
                        format!("{} got {}", show(r), e)
                    }),
                    (Ok(l), w) if w != b"ERR" => {
                        let (lines, dg) = digest(&zone_text(&l));
                        c.check(
                            lines.to_string().as_bytes() == r.b(2) && dg == r.s(3),
                            || format!("{} got {} lines {}", show(r), lines, dg),
                        );
                    }
                    (res, _) => c.check(false, || {
                        format!("{} got {:?}", show(r), res.map(|l| l.name.clone()))
                    }),
                }
            }
            b"SZ" => {
                let name = r.s(1);
                if sha256_file(&format!("/usr/share/zoneinfo/{}", name)) != r.s(2) {
                    skipped_sys += 1;
                    continue;
                }
                let res = go_time::load_location_env(&name, "", goroot.as_deref());
                match (res, r.b(3)) {
                    (Err(e), b"ERR") => c.check(e.error().as_bytes() == r.b(4), || {
                        format!("{} got {}", show(r), e)
                    }),
                    (Ok(l), w) if w != b"ERR" => {
                        let (lines, dg) = digest(&zone_text(&l));
                        c.check(
                            lines.to_string().as_bytes() == r.b(3) && dg == r.s(4),
                            || format!("{} got {} lines {}", show(r), lines, dg),
                        );
                    }
                    (res, _) => c.check(false, || {
                        format!("{} got {:?}", show(r), res.map(|l| l.name.clone()))
                    }),
                }
            }
            other => panic!("unknown tag {:?}", String::from_utf8_lossy(other)),
        }
    }
    eprintln!(
        "zoneall: skipped {} zip zones and {} system zones whose data changed",
        skipped_zip, skipped_sys
    );
    if c.total == 0 {
        eprintln!("zoneall: no zone data matches the fixture on this machine; nothing compared");
        return;
    }
    c.finish();
}
