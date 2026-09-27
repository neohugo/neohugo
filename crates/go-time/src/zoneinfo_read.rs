//! Port of `$GOROOT/src/time/zoneinfo_read.go` (go1.27.1): TZif parsing,
//! the uncompressed-zip reader and the zone source search.

use std::sync::Arc;

use go_value::{Location, Zone, ZoneTrans};

use crate::TimeError;
use crate::sys_unix::{ENOENT, Fd, close_fd, errno_string, open, preadn, read};
use crate::zoneinfo::ALPHA;

/// Go: `maxFileSize`, the max permitted size of files read by readFile.
const MAX_FILE_SIZE: usize = 10 << 20;

/// Errors of the zone loading helpers. Go compares them with
/// `syscall.ENOENT`, so the errno is kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LoadErr {
    /// A `syscall.Errno`.
    Errno(i32),
    /// Any other error, by its message.
    Msg(String),
}

impl LoadErr {
    pub(crate) fn is_enoent(&self) -> bool {
        matches!(self, LoadErr::Errno(ENOENT))
    }

    pub(crate) fn message(&self) -> String {
        match self {
            LoadErr::Errno(e) => errno_string(*e),
            LoadErr::Msg(m) => m.clone(),
        }
    }
}

/// Go: `dataIO`, a simple I/O interface to a binary blob of data.
struct DataIo<'a> {
    p: &'a [u8],
    error: bool,
}

impl<'a> DataIo<'a> {
    // Go: zoneinfo_read.go:(*dataIO).read
    fn read(&mut self, n: usize) -> &'a [u8] {
        if self.p.len() < n {
            self.p = &[];
            self.error = true;
            return &[];
        }
        let p = &self.p[0..n];
        self.p = &self.p[n..];
        p
    }

    // Go: zoneinfo_read.go:(*dataIO).big4
    fn big4(&mut self) -> Option<u32> {
        let p = self.read(4);
        if p.len() < 4 {
            self.error = true;
            return None;
        }
        Some((p[3] as u32) | (p[2] as u32) << 8 | (p[1] as u32) << 16 | (p[0] as u32) << 24)
    }

    // Go: zoneinfo_read.go:(*dataIO).big8
    fn big8(&mut self) -> Option<u64> {
        let n1 = self.big4();
        let n2 = self.big4();
        match (n1, n2) {
            (Some(n1), Some(n2)) => Some(((n1 as u64) << 32) | n2 as u64),
            _ => {
                self.error = true;
                None
            }
        }
    }

    // Go: zoneinfo_read.go:(*dataIO).byte
    fn byte(&mut self) -> Option<u8> {
        let p = self.read(1);
        if p.is_empty() {
            self.error = true;
            return None;
        }
        Some(p[0])
    }

    // Go: zoneinfo_read.go:(*dataIO).rest
    fn rest(&mut self) -> &'a [u8] {
        let r = self.p;
        self.p = &[];
        r
    }
}

// Go: zoneinfo_read.go:byteString
/// A string up to the first NUL. Zone abbreviations are Go strings (bytes);
/// `go_value::Zone::name` is a Rust `String`, so non-UTF-8 bytes (never
/// present in real tzdata) are replaced (see PORTING.md).
fn byte_string(p: &[u8]) -> String {
    let p = match p.iter().position(|&b| b == 0) {
        Some(i) => &p[..i],
        None => p,
    };
    String::from_utf8_lossy(p).into_owned()
}

/// The TZ ("extend") string of a TZif footer as a Rust `String`.
///
/// Go keeps the raw bytes and `tzset` ranges over them as runes, so every
/// byte of an invalid UTF-8 sequence is one `RuneError` of width 1 that is
/// neither a digit nor any of `,-+<>:./;JM`. Replacing such a byte by U+FFFD
/// (three bytes) would shift the byte offsets `tzsetName` checks (`i < 3`)
/// and accept rules Go rejects; replacing it by the single inert byte `?`
/// keeps `tzset`'s behaviour identical. Only zone names derived from those
/// bytes differ from Go (see PORTING.md).
fn extend_string(p: &[u8]) -> String {
    match std::str::from_utf8(p) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let mut out = String::with_capacity(p.len());
            let mut i = 0;
            while i < p.len() {
                let (r, size) = crate::utf8::decode_rune(&p[i..]);
                if r == crate::utf8::RUNE_ERROR && size == 1 {
                    out.push('?');
                } else {
                    // A valid sequence (the input is valid UTF-8 here).
                    out.push_str(std::str::from_utf8(&p[i..i + size]).unwrap_or("?"));
                }
                i += size;
            }
            out
        }
    }
}

/// Go: `errBadData`.
const ERR_BAD_DATA: &str = "malformed time zone information";

// Go: zoneinfo_read.go:LoadLocationFromTZData
/// A Location with the given name initialised from IANA TZif data.
pub fn load_location_from_tz_data(name: &str, data: &[u8]) -> Result<Arc<Location>, TimeError> {
    load_location_from_tz_data_(name, data).map_err(|e| TimeError::new(e.message()))
}

fn load_location_from_tz_data_(name: &str, data: &[u8]) -> Result<Arc<Location>, LoadErr> {
    let bad = || LoadErr::Msg(ERR_BAD_DATA.to_string());
    let mut d = DataIo {
        p: data,
        error: false,
    };

    // 4-byte magic "TZif"
    if d.read(4) != b"TZif" {
        return Err(bad());
    }

    // 1-byte version, then 15 bytes of padding
    let version: i32;
    let p = d.read(16);
    if p.len() != 16 {
        return Err(bad());
    } else {
        version = match p[0] {
            0 => 1,
            b'2' => 2,
            b'3' => 3,
            _ => return Err(bad()),
        };
    }

    // six big-endian 32-bit integers:
    //	number of UTC/local indicators
    //	number of standard/wall indicators
    //	number of leap seconds
    //	number of transition times
    //	number of local time zones
    //	number of characters of time zone abbrev strings
    const NUTC_LOCAL: usize = 0;
    const NSTD_WALL: usize = 1;
    const NLEAP: usize = 2;
    const NTIME: usize = 3;
    const NZONE: usize = 4;
    const NCHAR: usize = 5;
    let mut n = [0usize; 6];
    for slot in n.iter_mut() {
        let nn = d.big4().ok_or_else(bad)?;
        *slot = nn as usize;
    }

    // If we have version 2 or 3, then the data is first written out
    // in a 32-bit format, then written out again in a 64-bit format.
    // Skip the 32-bit format and read the 64-bit one, as it can
    // describe a broader range of dates.

    let mut is64 = false;
    if version > 1 {
        // Skip the 32-bit data.
        let mut skip = n[NTIME] * 4
            + n[NTIME]
            + n[NZONE] * 6
            + n[NCHAR]
            + n[NLEAP] * 8
            + n[NSTD_WALL]
            + n[NUTC_LOCAL];
        // Skip the version 2 header that we just read.
        skip += 4 + 16;
        d.read(skip);

        is64 = true;

        // Read the counts again, they can differ.
        for slot in n.iter_mut() {
            let nn = d.big4().ok_or_else(bad)?;
            *slot = nn as usize;
        }
    }

    let size = if is64 { 8 } else { 4 };

    // Transition times.
    let mut txtimes = DataIo {
        p: d.read(n[NTIME] * size),
        error: false,
    };

    // Time zone indices for transition times.
    let txzones = d.read(n[NTIME]);

    // Zone info structures
    let mut zonedata = DataIo {
        p: d.read(n[NZONE] * 6),
        error: false,
    };

    // Time zone abbreviations.
    let abbrev = d.read(n[NCHAR]);

    // Leap-second time pairs
    d.read(n[NLEAP] * (size + 4));

    // Whether tx times associated with local time types
    // are specified as standard time or wall time.
    let isstd = d.read(n[NSTD_WALL]);

    // Whether tx times associated with local time types
    // are specified as UTC or local time.
    let isutc = d.read(n[NUTC_LOCAL]);

    if d.error {
        // ran out of data
        return Err(bad());
    }

    let mut extend = String::new();
    let rest = d.rest();
    if rest.len() > 2 && rest[0] == b'\n' && rest[rest.len() - 1] == b'\n' {
        // Go: string(rest[1 : len(rest)-1]).
        extend = extend_string(&rest[1..rest.len() - 1]);
    }

    // Now we can build up a useful data structure.
    // First the zone information.
    //	utcoff[4] isdst[1] nameindex[1]
    let nzone = n[NZONE];
    if nzone == 0 {
        // Reject tzdata files with no zones. There's nothing useful in them.
        // This also avoids a panic later when we add and then use a fake transition (golang.org/issue/29437).
        return Err(bad());
    }
    let mut zones: Vec<Zone> = Vec::with_capacity(nzone);
    for _ in 0..nzone {
        let n = zonedata.big4().ok_or_else(bad)?;
        let offset = n as i32;
        let b = zonedata.byte().ok_or_else(bad)?;
        let is_dst = b != 0;
        let b = zonedata.byte().ok_or_else(bad)?;
        if b as usize >= abbrev.len() {
            return Err(bad());
        }
        let zname = byte_string(&abbrev[b as usize..]);
        // (the AIX Etc/GMT special case does not apply on darwin)
        zones.push(Zone {
            name: zname,
            offset,
            is_dst,
        });
    }

    // Now the transition time info.
    let mut tx: Vec<ZoneTrans> = Vec::with_capacity(n[NTIME]);
    for i in 0..n[NTIME] {
        let when: i64 = if !is64 {
            let n4 = txtimes.big4().ok_or_else(bad)?;
            n4 as i32 as i64
        } else {
            let n8 = txtimes.big8().ok_or_else(bad)?;
            n8 as i64
        };
        if txzones[i] as usize >= zones.len() {
            return Err(bad());
        }
        let mut t = ZoneTrans {
            when,
            index: txzones[i],
            is_std: false,
            is_utc: false,
        };
        if i < isstd.len() {
            t.is_std = isstd[i] != 0;
        }
        if i < isutc.len() {
            t.is_utc = isutc[i] != 0;
        }
        tx.push(t);
    }

    if tx.is_empty() {
        // Build fake transition to cover all time.
        // This happens in fixed locations like "Etc/GMT0".
        tx.push(ZoneTrans {
            when: ALPHA,
            index: 0,
            is_std: false,
            is_utc: false,
        });
    }

    // Committed to succeed.
    // (The "right now" lookup cache is not modelled.)
    Ok(Arc::new(Location {
        name: name.to_string(),
        zone: zones,
        tx,
        extend,
    }))
}

// Go: zoneinfo_read.go:loadTzinfoFromDirOrZip
/// The contents of the file `name` in `dir` (a directory or uncompressed zip).
pub(crate) fn load_tzinfo_from_dir_or_zip(dir: &str, name: &str) -> Result<Vec<u8>, LoadErr> {
    if dir.len() > 4 && dir.ends_with(".zip") {
        return load_tzinfo_from_zip(dir, name);
    }
    let path = if !dir.is_empty() {
        format!("{}/{}", dir, name)
    } else {
        name.to_string()
    };
    read_file(&path)
}

// Go: zoneinfo_read.go:get4
/// The little-endian 32-bit value in b (0 if short).
fn get4(b: &[u8]) -> usize {
    if b.len() < 4 {
        return 0;
    }
    (b[0] as usize) | (b[1] as usize) << 8 | (b[2] as usize) << 16 | (b[3] as usize) << 24
}

// Go: zoneinfo_read.go:get2
/// The little-endian 16-bit value in b (0 if short).
fn get2(b: &[u8]) -> usize {
    if b.len() < 2 {
        return 0;
    }
    (b[0] as usize) | (b[1] as usize) << 8
}

/// `&b[i..]` where Go would panic on a malformed zip; see PORTING.md.
fn tail(b: &[u8], i: usize) -> Option<&[u8]> {
    b.get(i..)
}

// Go: zoneinfo_read.go:loadTzinfoFromZip
/// The contents of `name` in the uncompressed zip file `zipfile`.
fn load_tzinfo_from_zip(zipfile: &str, name: &str) -> Result<Vec<u8>, LoadErr> {
    let fd = open(zipfile)?;
    let r = load_tzinfo_from_zip_fd(&fd, zipfile, name);
    close_fd(fd);
    r
}

fn load_tzinfo_from_zip_fd(fd: &Fd, zipfile: &str, name: &str) -> Result<Vec<u8>, LoadErr> {
    const ZECHEADER: usize = 0x06054b50;
    const ZCHEADER: usize = 0x02014b50;
    const ZTAILSIZE: usize = 22;

    const ZHEADERSIZE: usize = 30;
    const ZHEADER: usize = 0x04034b50;

    let corrupt = || LoadErr::Msg(format!("corrupt zip file {}", zipfile));

    let mut buf = vec![0u8; ZTAILSIZE];
    if preadn(fd, &mut buf, -(ZTAILSIZE as i64)).is_err() || get4(&buf) != ZECHEADER {
        return Err(corrupt());
    }
    let n = get2(&buf[10..]);
    let size = get4(&buf[12..]);
    let off = get4(&buf[16..]);

    let mut dir = vec![0u8; size];
    if preadn(fd, &mut dir, off as i64).is_err() {
        return Err(corrupt());
    }
    let mut buf: &[u8] = &dir;

    for _ in 0..n {
        // zip entry layout:
        //	0	magic[4]
        //	4	madevers[1]
        //	5	madeos[1]
        //	6	extvers[1]
        //	7	extos[1]
        //	8	flags[2]
        //	10	meth[2]
        //	12	modtime[2]
        //	14	moddate[2]
        //	16	crc[4]
        //	20	csize[4]
        //	24	uncsize[4]
        //	28	namelen[2]
        //	30	xlen[2]
        //	32	fclen[2]
        //	34	disknum[2]
        //	36	iattr[2]
        //	38	eattr[4]
        //	42	off[4]
        //	46	name[namelen]
        //	46+namelen+xlen+fclen - next header
        //
        if get4(buf) != ZCHEADER {
            break;
        }
        let meth = get2(tail(buf, 10).ok_or_else(corrupt)?);
        let size = get4(tail(buf, 24).ok_or_else(corrupt)?);
        let namelen = get2(tail(buf, 28).ok_or_else(corrupt)?);
        let xlen = get2(tail(buf, 30).ok_or_else(corrupt)?);
        let fclen = get2(tail(buf, 32).ok_or_else(corrupt)?);
        let off = get4(tail(buf, 42).ok_or_else(corrupt)?);
        let zname = buf.get(46..46 + namelen).ok_or_else(corrupt)?;
        let zname_matches = zname == name.as_bytes();
        buf = tail(buf, 46 + namelen + xlen + fclen).ok_or_else(corrupt)?;
        if !zname_matches {
            continue;
        }
        if meth != 0 {
            return Err(LoadErr::Msg(format!(
                "unsupported compression for {} in {}",
                name, zipfile
            )));
        }

        // zip per-file header layout:
        //	0	magic[4]
        //	4	extvers[1]
        //	5	extos[1]
        //	6	flags[2]
        //	8	meth[2]
        //	10	modtime[2]
        //	12	moddate[2]
        //	14	crc[4]
        //	18	csize[4]
        //	22	uncsize[4]
        //	26	namelen[2]
        //	28	xlen[2]
        //	30	name[namelen]
        //	30+namelen+xlen - file data
        //
        let mut hdr = vec![0u8; ZHEADERSIZE + namelen];
        if preadn(fd, &mut hdr, off as i64).is_err()
            || get4(&hdr) != ZHEADER
            || get2(&hdr[8..]) != meth
            || get2(&hdr[26..]) != namelen
            || &hdr[30..30 + namelen] != name.as_bytes()
        {
            return Err(corrupt());
        }
        let xlen = get2(&hdr[28..]);

        let mut data = vec![0u8; size];
        if preadn(fd, &mut data, (off + 30 + namelen + xlen) as i64).is_err() {
            return Err(corrupt());
        }

        return Ok(data);
    }

    Err(LoadErr::Errno(ENOENT))
}

// Go: zoneinfo_read.go:loadTzinfo
/// The time zone information of `name` from a source (directory or zip).
/// (The Android "tzdata" source kind is never used on darwin.)
fn load_tzinfo(name: &str, source: &str) -> Result<Vec<u8>, LoadErr> {
    load_tzinfo_from_dir_or_zip(source, name)
}

// Go: zoneinfo_read.go:loadLocation
/// The Location `name` from the first source that has it; the embedded
/// tzdata step is skipped (neohugo imports time/tzdata only on Windows).
pub(crate) fn load_location_sources(
    name: &str,
    sources: &[&str],
    goroot: Option<&str>,
) -> Result<Arc<Location>, LoadErr> {
    let mut first_err: Option<LoadErr> = None;
    for source in sources {
        let err = match load_tzinfo(name, source) {
            Ok(zone_data) => match load_location_from_tz_data_(name, &zone_data) {
                Ok(z) => return Ok(z),
                Err(e) => e,
            },
            Err(e) => e,
        };
        if first_err.is_none() && !err.is_enoent() {
            first_err = Some(err);
        }
    }
    // (loadFromEmbeddedTZData is nil: time/tzdata is not linked on darwin.)
    if let Some(source) = goroot_zone_source(goroot) {
        let err = match load_tzinfo(name, &source) {
            Ok(zone_data) => match load_location_from_tz_data_(name, &zone_data) {
                Ok(z) => return Ok(z),
                Err(e) => e,
            },
            Err(e) => e,
        };
        if first_err.is_none() && !err.is_enoent() {
            first_err = Some(err);
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Err(LoadErr::Msg(format!("unknown time zone {}", name)))
}

// Go: zoneinfo_goroot.go:gorootZoneSource
/// `$GOROOT/lib/time/zoneinfo.zip` for a non-empty GOROOT.
fn goroot_zone_source(goroot: Option<&str>) -> Option<String> {
    let goroot = goroot.unwrap_or("");
    if goroot.is_empty() {
        return None;
    }
    Some(format!("{}/lib/time/zoneinfo.zip", goroot))
}

/// Go: `runtime.GOROOT()`: the `GOROOT` environment variable, else the
/// build-time GOROOT. Only the environment variable is available to the
/// Rust port (see PORTING.md).
pub(crate) fn runtime_goroot() -> Option<String> {
    std::env::var("GOROOT").ok()
}

// Go: zoneinfo_read.go:readFile
/// The content of the named file (at most `maxFileSize` bytes).
pub(crate) fn read_file(name: &str) -> Result<Vec<u8>, LoadErr> {
    let f = open(name)?;
    let mut buf = [0u8; 4096];
    let mut ret: Vec<u8> = Vec::new();
    let result;
    loop {
        let (n, err) = read(&f, &mut buf);
        if n > 0 {
            ret.extend_from_slice(&buf[..n as usize]);
        }
        if n == 0 || err.is_some() {
            result = match err {
                Some(e) => Err(e),
                None => Ok(()),
            };
            break;
        }
        if ret.len() > MAX_FILE_SIZE {
            close_fd(f);
            return Err(LoadErr::Msg(format!("time: file {} is too large", name)));
        }
    }
    close_fd(f);
    // Go returns (ret, err); callers only use ret when err == nil.
    result.map(|_| ret)
}
