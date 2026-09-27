//! Port of `$GOROOT/src/time/zoneinfo.go` (go1.27.1): zone lookup, the
//! POSIX TZ ("extend") rules, `FixedZone`, `UTC`/`Local` and `LoadLocation`.
//!
//! Location identity: Go compares `*Location` pointers. Here `UTC` is a
//! process-wide `Arc` (and any zone-less location named "UTC" that is not
//! `Local` is treated as `&utcLoc`), `Local` is the `Arc` returned by
//! [`local`] (compared with `Arc::ptr_eq`).
//!
//! The per-Location lookup cache (`cacheStart/cacheEnd/cacheZone`) is not
//! modelled: it only memoises what `lookup` computes anyway (see PORTING.md).

use std::sync::{Arc, LazyLock, RwLock};

use go_value::{Location, Zone, ZoneTrans};

use crate::TimeError;
use crate::time::{
    INTERNAL_TO_ABSOLUTE, Month, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE,
    UNIX_TO_INTERNAL, abs_days, days_before, days_in, days_year_yday, is_leap,
};
use crate::zoneinfo_read::{
    LoadErr, load_location_from_tz_data, load_location_sources, load_tzinfo_from_dir_or_zip,
    runtime_goroot,
};
use crate::zoneinfo_unix::{PLATFORM_ZONE_SOURCES, init_local};

/// Go: `alpha`, the beginning of time for zone transitions.
pub(crate) const ALPHA: i64 = i64::MIN;
/// Go: `omega`, the end of time for zone transitions.
pub(crate) const OMEGA: i64 = i64::MAX;

static UTC_LOC: LazyLock<Arc<Location>> = LazyLock::new(|| {
    Arc::new(Location {
        name: "UTC".to_string(),
        zone: Vec::new(),
        tx: Vec::new(),
        extend: String::new(),
    })
});

/// Go: `time.UTC`.
pub fn utc() -> Arc<Location> {
    UTC_LOC.clone()
}

static LOCAL: RwLock<Option<Arc<Location>>> = RwLock::new(None);

/// Go: `time.Local`. Initialised on first use from `$TZ` (Go: `initLocal`).
pub fn local() -> Arc<Location> {
    {
        let r = LOCAL.read().unwrap_or_else(|e| e.into_inner());
        if let Some(l) = r.as_ref() {
            return l.clone();
        }
    }
    let mut w = LOCAL.write().unwrap_or_else(|e| e.into_inner());
    if w.is_none() {
        *w = Some(Arc::new(init_local()));
    }
    w.as_ref().expect("initialised above").clone()
}

/// Go: assigning `time.Local = loc`. Times created afterwards with `Local`
/// carry `loc`; used by tests and by callers that pin the local zone.
pub fn set_local(loc: Arc<Location>) {
    *LOCAL.write().unwrap_or_else(|e| e.into_inner()) = Some(loc);
}

/// Go: `l == Local` (pointer identity with the current `Local`).
pub fn is_local_loc(l: &Arc<Location>) -> bool {
    let r = LOCAL.read().unwrap_or_else(|e| e.into_inner());
    match r.as_ref() {
        Some(local) => Arc::ptr_eq(l, local),
        None => false,
    }
}

/// Go: `l == &utcLoc`. Besides the shared `UTC` Arc and go_value's shared
/// `Location::utc()`, any zone-less location named "UTC" that is not `Local`
/// is accepted, since Go cannot construct such a location other than `utcLoc`.
pub fn is_utc_loc(l: &Arc<Location>) -> bool {
    if Arc::ptr_eq(l, &UTC_LOC) {
        return true;
    }
    if l.name != "UTC" || !l.zone.is_empty() || !l.tx.is_empty() || !l.extend.is_empty() {
        return false;
    }
    // go_value's shared UTC is `&utcLoc` too, even after
    // `set_local(go_value::Location::utc())` (Go: `time.Local = time.UTC`);
    // any other zone-less "UTC" is `&utcLoc` unless it is Local (initLocal's
    // UTC fallback is `&localLoc`, not `&utcLoc`).
    Arc::ptr_eq(l, &Location::utc()) || !is_local_loc(l)
}

// Go: zoneinfo.go:(*Location).String
/// A descriptive name for the time zone (the `LoadLocation`/`FixedZone` name).
pub fn location_string(l: Option<&Arc<Location>>) -> String {
    match l {
        None => "UTC".to_string(),
        Some(l) => l.name.clone(),
    }
}

static UNNAMED_FIXED_ZONES: LazyLock<Vec<Arc<Location>>> = LazyLock::new(|| {
    let mut v = Vec::with_capacity((HOURS_BEFORE_UTC + 1 + HOURS_AFTER_UTC) as usize);
    for hr in -HOURS_BEFORE_UTC..=HOURS_AFTER_UTC {
        v.push(fixed_zone_(String::new(), hr * 60 * 60));
    }
    v
});

const HOURS_BEFORE_UTC: i64 = 12;
const HOURS_AFTER_UTC: i64 = 14;

// Go: zoneinfo.go:FixedZone
/// A Location that always uses the given zone name and offset (seconds east of UTC).
/// Offsets are stored as `i32` in `go_value::Zone` (Go: `int`); see PORTING.md.
pub fn fixed_zone(name: &str, offset: i64) -> Arc<Location> {
    // Most calls to FixedZone have an unnamed zone with an offset by the hour.
    // Optimize for that case by returning the same *Location for a given hour.
    let hour = offset / 60 / 60;
    if name.is_empty()
        && -HOURS_BEFORE_UTC <= hour
        && hour <= HOURS_AFTER_UTC
        && hour * 60 * 60 == offset
    {
        return UNNAMED_FIXED_ZONES[(hour + HOURS_BEFORE_UTC) as usize].clone();
    }
    fixed_zone_(name.to_string(), offset)
}

// Go: zoneinfo.go:fixedZone
fn fixed_zone_(name: String, offset: i64) -> Arc<Location> {
    Arc::new(Location {
        name: name.clone(),
        zone: vec![Zone {
            name,
            offset: offset as i32,
            is_dst: false,
        }],
        tx: vec![ZoneTrans {
            when: ALPHA,
            index: 0,
            is_std: false,
            is_utc: false,
        }],
        extend: String::new(),
    })
}

/// Result of [`lookup`] (Go returns these as five values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZoneLookup<'a> {
    pub name: &'a str,
    pub offset: i64,
    pub start: i64,
    pub end: i64,
    pub is_dst: bool,
}

const UTC_LOOKUP: ZoneLookup<'static> = ZoneLookup {
    name: "UTC",
    offset: 0,
    start: ALPHA,
    end: OMEGA,
    is_dst: false,
};

/// Go: `l.lookup(sec)` where `l` may be nil (UTC).
pub fn lookup_opt(l: Option<&Arc<Location>>, sec: i64) -> ZoneLookup<'_> {
    match l {
        None => UTC_LOOKUP,
        Some(l) => lookup(l, sec),
    }
}

// Go: zoneinfo.go:(*Location).lookup
/// The zone in use at the instant `sec` (seconds since the Unix epoch).
pub fn lookup(l: &Location, sec: i64) -> ZoneLookup<'_> {
    if l.zone.is_empty() {
        return UTC_LOOKUP;
    }

    // (lookup cache not modelled)

    if l.tx.is_empty() || sec < l.tx[0].when {
        let zone = &l.zone[lookup_first_zone(l)];
        let end = if !l.tx.is_empty() {
            l.tx[0].when
        } else {
            OMEGA
        };
        return ZoneLookup {
            name: &zone.name,
            offset: zone.offset as i64,
            start: ALPHA,
            end,
            is_dst: zone.is_dst,
        };
    }

    // Binary search for entry with largest time <= sec.
    let tx = &l.tx;
    let mut end = OMEGA;
    let mut lo = 0usize;
    let mut hi = tx.len();
    while hi - lo > 1 {
        let m = (lo + hi) >> 1;
        let lim = tx[m].when;
        if sec < lim {
            end = lim;
            hi = m;
        } else {
            lo = m;
        }
    }
    let zone = &l.zone[tx[lo].index as usize];
    let start = tx[lo].when;
    // end = maintained during the search

    // If we're at the end of the known zone transitions,
    // try the extend string.
    if lo == tx.len() - 1 && !l.extend.is_empty() {
        if let Some(e) = tzset(&l.extend, start, sec) {
            return e;
        }
    }

    ZoneLookup {
        name: &zone.name,
        offset: zone.offset as i64,
        start,
        end,
        is_dst: zone.is_dst,
    }
}

// Go: zoneinfo.go:(*Location).lookupFirstZone
/// Index of the zone to use for times before the first transition.
fn lookup_first_zone(l: &Location) -> usize {
    // Case 1.
    if !first_zone_used(l) {
        return 0;
    }

    // Case 2.
    if !l.tx.is_empty() && l.zone[l.tx[0].index as usize].is_dst {
        let mut zi = l.tx[0].index as i64 - 1;
        while zi >= 0 {
            if !l.zone[zi as usize].is_dst {
                return zi as usize;
            }
            zi -= 1;
        }
    }

    // Case 3.
    for (zi, z) in l.zone.iter().enumerate() {
        if !z.is_dst {
            return zi;
        }
    }

    // Case 4.
    0
}

// Go: zoneinfo.go:(*Location).firstZoneUsed
fn first_zone_used(l: &Location) -> bool {
    l.tx.iter().any(|tx| tx.index == 0)
}

// Go: zoneinfo.go:tzset
/// Evaluates a TZ string (the tzdata "extend" rule) at `sec`, given the time
/// of the last transition; `None` when the string does not parse.
pub(crate) fn tzset(s: &str, last_tx_sec: i64, sec: i64) -> Option<ZoneLookup<'_>> {
    let (mut std_name, s) = tzset_name(s)?;
    let (mut std_offset, s) = tzset_offset(s)?;

    // The numbers in the tzset string are added to local time to get UTC,
    // but our offsets are added to UTC to get local time,
    // so we negate the number we see here.
    std_offset = -std_offset;

    if s.is_empty() || s.as_bytes()[0] == b',' {
        // No daylight savings time.
        return Some(ZoneLookup {
            name: std_name,
            offset: std_offset,
            start: last_tx_sec,
            end: OMEGA,
            is_dst: false,
        });
    }

    let (mut dst_name, mut s) = tzset_name(s)?;
    let mut dst_offset;
    if s.is_empty() || s.as_bytes()[0] == b',' {
        dst_offset = std_offset + SECONDS_PER_HOUR;
    } else {
        let (o, rest) = tzset_offset(s)?;
        dst_offset = -o; // as with stdOffset, above
        s = rest;
    }

    if s.is_empty() {
        // Default DST rules per tzcode.
        s = ",M3.2.0,M11.1.0";
    }
    // The TZ definition does not mention ';' here but tzcode accepts it.
    if s.as_bytes()[0] != b',' && s.as_bytes()[0] != b';' {
        return None;
    }
    let s = &s[1..];

    let (start_rule, s) = tzset_rule(s)?;
    if s.is_empty() || s.as_bytes()[0] != b',' {
        return None;
    }
    let s = &s[1..];
    let (end_rule, s) = tzset_rule(s)?;
    if !s.is_empty() {
        return None;
    }

    // Compute start of year in seconds since Unix epoch,
    // and seconds since then to get to sec.
    let (year, yday) = days_year_yday(abs_days(
        sec.wrapping_add(UNIX_TO_INTERNAL)
            .wrapping_add(INTERNAL_TO_ABSOLUTE) as u64,
    ));
    let ysec = (yday - 1)
        .wrapping_mul(SECONDS_PER_DAY)
        .wrapping_add(sec % SECONDS_PER_DAY);
    let ystart = sec.wrapping_sub(ysec);

    let mut start_sec = tzrule_time(year, start_rule, std_offset);
    let mut end_sec = tzrule_time(year, end_rule, dst_offset);
    let (mut dst_is_dst, mut std_is_dst) = (true, false);
    // Note: this is a flipping of "DST" and "STD" while retaining the labels
    // This happens in southern hemispheres. The labelling here thus is a little
    // inconsistent with the goal.
    if end_sec < start_sec {
        std::mem::swap(&mut start_sec, &mut end_sec);
        std::mem::swap(&mut std_name, &mut dst_name);
        std::mem::swap(&mut std_offset, &mut dst_offset);
        std::mem::swap(&mut std_is_dst, &mut dst_is_dst);
    }

    // The start and end values that we return are accurate
    // close to a daylight savings transition, but are otherwise
    // just the start and end of the year. That suffices for
    // the only caller that cares, which is Date.
    if ysec < start_sec {
        Some(ZoneLookup {
            name: std_name,
            offset: std_offset,
            start: ystart,
            end: start_sec.wrapping_add(ystart),
            is_dst: std_is_dst,
        })
    } else if ysec >= end_sec {
        Some(ZoneLookup {
            name: std_name,
            offset: std_offset,
            start: end_sec.wrapping_add(ystart),
            end: ystart.wrapping_add(365 * SECONDS_PER_DAY),
            is_dst: std_is_dst,
        })
    } else {
        Some(ZoneLookup {
            name: dst_name,
            offset: dst_offset,
            start: start_sec.wrapping_add(ystart),
            end: end_sec.wrapping_add(ystart),
            is_dst: dst_is_dst,
        })
    }
}

// Go: zoneinfo.go:tzsetName
/// The timezone name at the start of s, and the remainder.
///
/// Go ranges over runes; only ASCII runes are tested and the indices are byte
/// offsets, so a byte scan is equivalent (a non-ASCII byte never matches).
fn tzset_name(s: &str) -> Option<(&str, &str)> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    if b[0] != b'<' {
        for (i, &r) in b.iter().enumerate() {
            match r {
                b'0'..=b'9' | b',' | b'-' | b'+' => {
                    if i < 3 {
                        return None;
                    }
                    return Some((&s[..i], &s[i..]));
                }
                _ => {}
            }
        }
        if b.len() < 3 {
            return None;
        }
        Some((s, ""))
    } else {
        for (i, &r) in b.iter().enumerate() {
            if r == b'>' {
                return Some((&s[1..i], &s[i + 1..]));
            }
        }
        None
    }
}

// Go: zoneinfo.go:tzsetOffset
/// The timezone offset (seconds) at the start of s, and the remainder.
fn tzset_offset(s: &str) -> Option<(i64, &str)> {
    let mut s = s;
    if s.is_empty() {
        return None;
    }
    let mut neg = false;
    if s.as_bytes()[0] == b'+' {
        s = &s[1..];
    } else if s.as_bytes()[0] == b'-' {
        s = &s[1..];
        neg = true;
    }

    // The tzdata code permits values up to 24 * 7 here,
    // although POSIX does not.
    let (hours, rest) = tzset_num(s, 0, 24 * 7)?;
    s = rest;
    let mut off = hours * SECONDS_PER_HOUR;
    if s.is_empty() || s.as_bytes()[0] != b':' {
        if neg {
            off = -off;
        }
        return Some((off, s));
    }

    let (mins, rest) = tzset_num(&s[1..], 0, 59)?;
    s = rest;
    off += mins * SECONDS_PER_MINUTE;
    if s.is_empty() || s.as_bytes()[0] != b':' {
        if neg {
            off = -off;
        }
        return Some((off, s));
    }

    let (secs, rest) = tzset_num(&s[1..], 0, 59)?;
    s = rest;
    off += secs;

    if neg {
        off = -off;
    }
    Some((off, s))
}

/// Go: `ruleKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleKind {
    Julian,
    Doy,
    MonthWeekDay,
}

/// Go: `rule`, a rule read from a tzset string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rule {
    kind: RuleKind,
    day: i64,
    week: i64,
    mon: i64,
    time: i64, // transition time
}

// Go: zoneinfo.go:tzsetRule
fn tzset_rule(s: &str) -> Option<(Rule, &str)> {
    let mut r = Rule {
        kind: RuleKind::Julian,
        day: 0,
        week: 0,
        mon: 0,
        time: 0,
    };
    if s.is_empty() {
        return None;
    }
    let mut s = s;
    let b0 = s.as_bytes()[0];
    if b0 == b'J' {
        let (jday, rest) = tzset_num(&s[1..], 1, 365)?;
        s = rest;
        r.kind = RuleKind::Julian;
        r.day = jday;
    } else if b0 == b'M' {
        let (mon, rest) = tzset_num(&s[1..], 1, 12)?;
        s = rest;
        if s.is_empty() || s.as_bytes()[0] != b'.' {
            return None;
        }
        let (week, rest) = tzset_num(&s[1..], 1, 5)?;
        s = rest;
        if s.is_empty() || s.as_bytes()[0] != b'.' {
            return None;
        }
        let (day, rest) = tzset_num(&s[1..], 0, 6)?;
        s = rest;
        r.kind = RuleKind::MonthWeekDay;
        r.day = day;
        r.week = week;
        r.mon = mon;
    } else {
        let (day, rest) = tzset_num(s, 0, 365)?;
        s = rest;
        r.kind = RuleKind::Doy;
        r.day = day;
    }

    if s.is_empty() || s.as_bytes()[0] != b'/' {
        r.time = 2 * SECONDS_PER_HOUR; // 2am is the default
        return Some((r, s));
    }

    let (offset, s) = tzset_offset(&s[1..])?;
    r.time = offset;

    Some((r, s))
}

// Go: zoneinfo.go:tzsetNum
/// A number in [min, max] at the start of s, and the remainder.
/// (Go ranges over runes; a byte scan is equivalent for the ASCII tests.)
fn tzset_num(s: &str, min: i64, max: i64) -> Option<(i64, &str)> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    let mut num: i64 = 0;
    for (i, &r) in b.iter().enumerate() {
        if !r.is_ascii_digit() {
            if i == 0 || num < min {
                return None;
            }
            return Some((num, &s[i..]));
        }
        num *= 10;
        num += (r - b'0') as i64;
        if num > max {
            return None;
        }
    }
    if num < min {
        return None;
    }
    Some((num, ""))
}

// Go: zoneinfo.go:tzruleTime
/// Seconds since the start of the year at which the rule takes effect.
fn tzrule_time(year: i64, r: Rule, off: i64) -> i64 {
    let s: i64;
    match r.kind {
        RuleKind::Julian => {
            let mut j = (r.day - 1) * SECONDS_PER_DAY;
            if is_leap(year) && r.day >= 60 {
                j += SECONDS_PER_DAY;
            }
            s = j;
        }
        RuleKind::Doy => {
            s = r.day * SECONDS_PER_DAY;
        }
        RuleKind::MonthWeekDay => {
            // Zeller's Congruence.
            let m1 = (r.mon + 9) % 12 + 1;
            let mut yy0 = year;
            if r.mon <= 2 {
                yy0 = yy0.wrapping_sub(1);
            }
            let yy1 = yy0 / 100;
            let yy2 = yy0 % 100;
            let mut dow = ((26 * m1 - 2) / 10 + 1 + yy2 + yy2 / 4 + yy1 / 4)
                .wrapping_sub(yy1.wrapping_mul(2))
                % 7;
            if dow < 0 {
                dow += 7;
            }
            // Now dow is the day-of-week of the first day of r.mon.
            // Get the day-of-month of the first "dow" day.
            let mut d = r.day - dow;
            if d < 0 {
                d += 7;
            }
            for _ in 1..r.week {
                if d + 7 >= days_in(Month(r.mon), year) {
                    break;
                }
                d += 7;
            }
            d += days_before(Month(r.mon));
            if is_leap(year) && r.mon > 2 {
                d += 1;
            }
            s = d * SECONDS_PER_DAY;
        }
    }

    s + r.time - off
}

// Go: zoneinfo.go:(*Location).lookupName
/// The offset of the zone named `name` at pseudo-Unix time `unix`.
pub(crate) fn lookup_name(l: &Location, name: &[u8], unix: i64) -> Option<i64> {
    // First try for a zone with the right name that was actually
    // in effect at the given time. (In Sydney, Australia, both standard
    // and daylight-savings time are abbreviated "EST". Using the
    // offset helps us pick the right one for the given time.
    // It's not perfect: during the backward transition we might pick
    // either one.)
    for zone in &l.zone {
        if zone.name.as_bytes() == name {
            let z = lookup(l, unix.wrapping_sub(zone.offset as i64));
            if z.name == zone.name {
                return Some(z.offset);
            }
        }
    }

    // Otherwise fall back to an ordinary name match.
    for zone in &l.zone {
        if zone.name.as_bytes() == name {
            return Some(zone.offset as i64);
        }
    }

    // Otherwise, give up.
    None
}

/// Go: `errLocation`.
const ERR_LOCATION: &str = "time: invalid location name";

static ZONEINFO: LazyLock<String> = LazyLock::new(|| std::env::var("ZONEINFO").unwrap_or_default());

// Go: zoneinfo.go:LoadLocation
/// The Location with the given IANA name ("" and "UTC" give `UTC`,
/// "Local" gives `Local`).
pub fn load_location_named(name: &str) -> Result<Arc<Location>, TimeError> {
    load_location_env(name, &ZONEINFO, runtime_goroot().as_deref())
}

/// [`load_location_named`] with explicit `$ZONEINFO` and `runtime.GOROOT()`.
pub fn load_location_env(
    name: &str,
    zoneinfo: &str,
    goroot: Option<&str>,
) -> Result<Arc<Location>, TimeError> {
    if name.is_empty() || name == "UTC" {
        return Ok(utc());
    }
    if name == "Local" {
        return Ok(local());
    }
    if contains_dot_dot(name) || name.as_bytes()[0] == b'/' || name.as_bytes()[0] == b'\\' {
        // No valid IANA Time Zone name contains a single dot,
        // much less dot dot. Likewise, none begin with a slash.
        return Err(TimeError::new(ERR_LOCATION));
    }
    let mut first_err: Option<LoadErr> = None;
    if !zoneinfo.is_empty() {
        match load_tzinfo_from_dir_or_zip(zoneinfo, name) {
            Ok(zone_data) => {
                if let Ok(z) = load_location_from_tz_data(name, &zone_data) {
                    return Ok(z);
                }
                // Go assigns the outer (nil) err here: `firstErr = err`
                // refers to the loadTzinfoFromDirOrZip error, which is nil.
                first_err = None;
            }
            Err(e) => {
                if !e.is_enoent() {
                    first_err = Some(e);
                }
            }
        }
    }
    match load_location_sources(name, PLATFORM_ZONE_SOURCES, goroot) {
        Ok(z) => Ok(z),
        Err(e) => {
            let e = match first_err {
                Some(fe) => fe,
                None => e,
            };
            Err(TimeError::new(e.message()))
        }
    }
}

// Go: zoneinfo.go:containsDotDot
fn contains_dot_dot(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 2 {
        return false;
    }
    for i in 0..b.len() - 1 {
        if b[i] == b'.' && b[i + 1] == b'.' {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tzset_name_cases() {
        assert_eq!(tzset_name("PST8PDT"), Some(("PST", "8PDT")));
        assert_eq!(tzset_name("<+07>-7"), Some(("+07", "-7")));
        assert_eq!(tzset_name("AB1"), None);
        assert_eq!(tzset_name("ABC"), Some(("ABC", "")));
        assert_eq!(tzset_name("<abc"), None);
    }

    // Go: zoneinfo_test.go:TestTzset
    #[test]
    #[allow(clippy::type_complexity)]
    fn go_tzset() {
        let cases: &[(&str, i64, i64, &str, i64, i64, i64, bool, bool)] = &[
            ("", 0, 0, "", 0, 0, 0, false, false),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2159200800,
                "PDT",
                -7 * 60 * 60,
                2152173600,
                2172733200,
                true,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2152173599,
                "PST",
                -8 * 60 * 60,
                2145916800,
                2152173600,
                false,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2152173600,
                "PDT",
                -7 * 60 * 60,
                2152173600,
                2172733200,
                true,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2152173601,
                "PDT",
                -7 * 60 * 60,
                2152173600,
                2172733200,
                true,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2172733199,
                "PDT",
                -7 * 60 * 60,
                2152173600,
                2172733200,
                true,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2172733200,
                "PST",
                -8 * 60 * 60,
                2172733200,
                2177452800,
                false,
                true,
            ),
            (
                "PST8PDT,M3.2.0,M11.1.0",
                0,
                2172733201,
                "PST",
                -8 * 60 * 60,
                2172733200,
                2177452800,
                false,
                true,
            ),
            (
                "KST-9",
                592333200,
                1677246697,
                "KST",
                9 * 60 * 60,
                592333200,
                i64::MAX,
                false,
                true,
            ),
        ];
        for &(s, in_end, in_sec, name, off, start, end, is_dst, ok) in cases {
            let got = tzset(s, in_end, in_sec);
            match got {
                None => assert!(!ok, "tzset({:?}) failed", s),
                Some(z) => {
                    assert!(ok, "tzset({:?}) should fail", s);
                    assert_eq!(
                        (z.name, z.offset, z.start, z.end, z.is_dst),
                        (name, off, start, end, is_dst),
                        "tzset({:?}, {}, {})",
                        s,
                        in_end,
                        in_sec
                    );
                }
            }
        }
    }

    // Go: zoneinfo_test.go:TestTzsetName
    #[test]
    fn go_tzset_name() {
        let cases: &[(&str, &str, &str, bool)] = &[
            ("", "", "", false),
            ("X", "", "", false),
            ("PST", "PST", "", true),
            ("PST8PDT", "PST", "8PDT", true),
            ("PST-08", "PST", "-08", true),
            ("<A+B>+08", "A+B", "+08", true),
        ];
        for &(input, name, out, ok) in cases {
            let got = tzset_name(input);
            assert_eq!(
                got,
                if ok { Some((name, out)) } else { None },
                "tzsetName({:?})",
                input
            );
        }
    }

    // Go: zoneinfo_test.go:TestTzsetOffset
    #[test]
    fn go_tzset_offset() {
        let cases: &[(&str, i64, &str, bool)] = &[
            ("", 0, "", false),
            ("X", 0, "", false),
            ("+", 0, "", false),
            ("+08", 8 * 60 * 60, "", true),
            ("-01:02:03", -60 * 60 - 2 * 60 - 3, "", true),
            ("01", 60 * 60, "", true),
            ("100", 100 * 60 * 60, "", true),
            ("1000", 0, "", false),
            ("8PDT", 8 * 60 * 60, "PDT", true),
        ];
        for &(input, off, out, ok) in cases {
            let got = tzset_offset(input);
            assert_eq!(
                got,
                if ok { Some((off, out)) } else { None },
                "tzsetOffset({:?})",
                input
            );
        }
    }

    // Go: zoneinfo_test.go:TestTzsetRule
    #[test]
    fn go_tzset_rule() {
        let r = |kind, day, week, mon, time| Rule {
            kind,
            day,
            week,
            mon,
            time,
        };
        let cases: Vec<(&str, Option<Rule>)> = vec![
            ("", None),
            ("X", None),
            ("J10", Some(r(RuleKind::Julian, 10, 0, 0, 2 * 60 * 60))),
            ("20", Some(r(RuleKind::Doy, 20, 0, 0, 2 * 60 * 60))),
            (
                "M1.2.3",
                Some(r(RuleKind::MonthWeekDay, 3, 2, 1, 2 * 60 * 60)),
            ),
            ("30/03:00:00", Some(r(RuleKind::Doy, 30, 0, 0, 3 * 60 * 60))),
            (
                "M4.5.6/03:00:00",
                Some(r(RuleKind::MonthWeekDay, 6, 5, 4, 3 * 60 * 60)),
            ),
            ("M4.5.7/03:00:00", None),
            (
                "M4.5.6/-04",
                Some(r(RuleKind::MonthWeekDay, 6, 5, 4, -4 * 60 * 60)),
            ),
        ];
        for (input, want) in cases {
            let got = tzset_rule(input);
            assert_eq!(got, want.map(|w| (w, "")), "tzsetRule({:?})", input);
        }
    }

    #[test]
    fn contains_dot_dot_cases() {
        assert!(contains_dot_dot(".."));
        assert!(contains_dot_dot("a/../b"));
        assert!(!contains_dot_dot("."));
        assert!(!contains_dot_dot("a.b"));
    }
}
