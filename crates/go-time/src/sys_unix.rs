//! Port of `$GOROOT/src/time/sys_unix.go` (go1.27.1): the tiny file API used
//! by the zoneinfo readers, with Go's `syscall.Errno` error strings
//! (`syscall/zerrors_darwin_arm64.go`).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::zoneinfo_read::LoadErr;

pub(crate) const ENOENT: i32 = 2;
const EINVAL: i32 = 22;

/// An open file (Go: a raw `uintptr` fd).
pub(crate) type Fd = File;

fn io_err(e: std::io::Error) -> LoadErr {
    match e.raw_os_error() {
        Some(n) => LoadErr::Errno(n),
        // A path containing NUL: Go's syscall.BytePtrFromString returns EINVAL.
        None if e.kind() == std::io::ErrorKind::InvalidInput => LoadErr::Errno(EINVAL),
        None => LoadErr::Msg(e.to_string()),
    }
}

// Go: sys_unix.go:open
pub(crate) fn open(name: &str) -> Result<Fd, LoadErr> {
    File::open(name).map_err(io_err)
}

// Go: sys_unix.go:read
/// Go's `syscall.Read`: `(n, err)`, with `n == -1` on error.
pub(crate) fn read(fd: &Fd, buf: &mut [u8]) -> (i64, Option<LoadErr>) {
    let mut f: &File = fd;
    match f.read(buf) {
        Ok(n) => (n as i64, None),
        Err(e) => (-1, Some(io_err(e))),
    }
}

// Go: sys_unix.go:closefd
pub(crate) fn close_fd(fd: Fd) {
    drop(fd);
}

// Go: sys_unix.go:preadn
/// Reads exactly `buf.len()` bytes at `off` (from the end when negative).
pub(crate) fn preadn(fd: &Fd, buf: &mut [u8], off: i64) -> Result<(), LoadErr> {
    let mut f: &File = fd;
    let pos = if off < 0 {
        SeekFrom::End(off)
    } else {
        SeekFrom::Start(off as u64)
    };
    f.seek(pos).map_err(io_err)?;
    let mut buf = buf;
    while !buf.is_empty() {
        match f.read(buf) {
            Ok(0) => return Err(LoadErr::Msg("short read".to_string())),
            Ok(m) => buf = &mut buf[m..],
            Err(e) => return Err(io_err(e)),
        }
    }
    Ok(())
}

/// Go: `syscall.Errno.Error()` on darwin.
pub(crate) fn errno_string(e: i32) -> String {
    if 0 <= e && (e as usize) < ERRORS.len() {
        let s = ERRORS[e as usize];
        if !s.is_empty() {
            return s.to_string();
        }
    }
    format!("errno {}", e)
}

/// Go: `syscall/zerrors_darwin_arm64.go` `errors` table.
static ERRORS: [&str; 107] = [
    "",
    "operation not permitted",
    "no such file or directory",
    "no such process",
    "interrupted system call",
    "input/output error",
    "device not configured",
    "argument list too long",
    "exec format error",
    "bad file descriptor",
    "no child processes",
    "resource deadlock avoided",
    "cannot allocate memory",
    "permission denied",
    "bad address",
    "block device required",
    "resource busy",
    "file exists",
    "cross-device link",
    "operation not supported by device",
    "not a directory",
    "is a directory",
    "invalid argument",
    "too many open files in system",
    "too many open files",
    "inappropriate ioctl for device",
    "text file busy",
    "file too large",
    "no space left on device",
    "illegal seek",
    "read-only file system",
    "too many links",
    "broken pipe",
    "numerical argument out of domain",
    "result too large",
    "resource temporarily unavailable",
    "operation now in progress",
    "operation already in progress",
    "socket operation on non-socket",
    "destination address required",
    "message too long",
    "protocol wrong type for socket",
    "protocol not available",
    "protocol not supported",
    "socket type not supported",
    "operation not supported",
    "protocol family not supported",
    "address family not supported by protocol family",
    "address already in use",
    "can't assign requested address",
    "network is down",
    "network is unreachable",
    "network dropped connection on reset",
    "software caused connection abort",
    "connection reset by peer",
    "no buffer space available",
    "socket is already connected",
    "socket is not connected",
    "can't send after socket shutdown",
    "too many references: can't splice",
    "operation timed out",
    "connection refused",
    "too many levels of symbolic links",
    "file name too long",
    "host is down",
    "no route to host",
    "directory not empty",
    "too many processes",
    "too many users",
    "disc quota exceeded",
    "stale NFS file handle",
    "too many levels of remote in path",
    "RPC struct is bad",
    "RPC version wrong",
    "RPC prog. not avail",
    "program version wrong",
    "bad procedure for program",
    "no locks available",
    "function not implemented",
    "inappropriate file type or format",
    "authentication error",
    "need authenticator",
    "device power is off",
    "device error",
    "value too large to be stored in data type",
    "bad executable (or shared library)",
    "bad CPU type in executable",
    "shared library version mismatch",
    "malformed Mach-o file",
    "operation canceled",
    "identifier removed",
    "no message of desired type",
    "illegal byte sequence",
    "attribute not found",
    "bad message",
    "EMULTIHOP (Reserved)",
    "no message available on STREAM",
    "ENOLINK (Reserved)",
    "no STREAM resources",
    "not a STREAM",
    "protocol error",
    "STREAM ioctl timeout",
    "operation not supported on socket",
    "policy not found",
    "state not recoverable",
    "previous owner died",
    "interface output queue is full",
];
