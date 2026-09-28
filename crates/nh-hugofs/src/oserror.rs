//! Go `os` / `syscall` error values and texts (NEW; the hugofs layers return Go's errors, and
//! their texts are part of the acceptance tests).
//!
//! Owner: Wave B task T05 (hugofs-vfs).
//!
//! `os.PathError` prints `<op> <path>: <err>`; a `syscall.Errno` prints the text of Go's
//! `syscall.errors` table (`zerrors_linux_arm64.go` / `zerrors_darwin_arm64.go` of go1.27.1,
//! copied below; linux/amd64 lacks only entry 133). The `ErrorKind` follows `Errno.Is`:
//! `ENOENT` is `os.ErrNotExist`, `EEXIST`/`ENOTEMPTY` are `os.ErrExist`.

use std::io;

use nh_common::herrors::{Error, ErrorKind};

/// Go: `os.ErrNotExist.Error()` (`fs.ErrNotExist`).
pub const ERR_NOT_EXIST: &str = "file does not exist";
/// Go: `os.ErrPermission.Error()`.
pub const ERR_PERMISSION: &str = "permission denied";
/// Go: `os.ErrClosed.Error()`.
pub const ERR_CLOSED: &str = "file already closed";

const EPERM: i32 = 1;
const ENOENT: i32 = 2;
const EEXIST: i32 = 17;
const ENOTDIR: i32 = 20;
#[cfg(target_os = "macos")]
const ENOTEMPTY: i32 = 66;
#[cfg(not(target_os = "macos"))]
const ENOTEMPTY: i32 = 39;

/// Go: `io.EOF`.
pub fn eof() -> Error {
    Error::new("EOF")
}

/// Whether `e` is [`eof`].
pub fn is_eof(e: &Error) -> bool {
    e.kind() == ErrorKind::Generic && e.message() == "EOF"
}

/// Go: `os.ErrNotExist`.
pub fn err_not_exist() -> Error {
    Error::not_exist(ERR_NOT_EXIST)
}

/// Go: `os.ErrPermission`.
pub fn err_permission() -> Error {
    Error::new(ERR_PERMISSION)
}

/// Go: `syscall.EPERM` (returned by the read-only filesystems).
pub fn eperm() -> Error {
    errno_error(EPERM)
}

/// Go: `syscall.ENOTDIR`.
pub fn enotdir() -> Error {
    errno_error(ENOTDIR)
}

/// Go: `syscall.Errno(n)` as an error.
pub fn errno_error(n: i32) -> Error {
    Error::with_kind(errno_kind(n), errno_text(n))
}

/// Go: `&os.PathError{Op: op, Path: path, Err: err}` (`Unwrap` keeps the kind).
pub fn path_error(op: &str, path: &str, err: &Error) -> Error {
    Error::with_kind(err.kind(), format!("{op} {path}: {}", err.message()))
}

/// Go: the `*os.PathError` the `os` functions return for an OS error.
pub fn from_io(op: &str, path: &str, e: &io::Error) -> Error {
    path_error(op, path, &io_errno(e))
}

/// The `syscall.Errno` of an OS error (other errors keep their message).
pub fn io_errno(e: &io::Error) -> Error {
    match e.raw_os_error() {
        Some(n) => errno_error(n),
        None => {
            let kind = match e.kind() {
                io::ErrorKind::NotFound => ErrorKind::NotExist,
                io::ErrorKind::AlreadyExists => ErrorKind::Exist,
                _ => ErrorKind::Generic,
            };
            Error::with_kind(kind, e.to_string())
        }
    }
}

/// An `io::Error` carrying a hugofs error (for the `Read`/`Write`/`Seek` impls of the files).
pub fn to_io(e: Error) -> io::Error {
    let kind = match e.kind() {
        ErrorKind::NotExist => io::ErrorKind::NotFound,
        ErrorKind::Exist => io::ErrorKind::AlreadyExists,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, e)
}

/// The hugofs error inside an `io::Error` made by [`to_io`], else the OS error.
pub fn from_io_plain(e: io::Error) -> Error {
    if e.get_ref().is_some_and(|r| r.is::<Error>()) {
        let kind = e.kind();
        if let Some(inner) = e.into_inner()
            && let Ok(err) = inner.downcast::<Error>()
        {
            return *err;
        }
        return Error::new(format!("{kind}"));
    }
    io_errno(&e)
}

// Go: syscall/syscall_unix.go:Errno.Is (ErrNotExist / ErrExist)
fn errno_kind(n: i32) -> ErrorKind {
    match n {
        ENOENT => ErrorKind::NotExist,
        EEXIST | ENOTEMPTY => ErrorKind::Exist,
        _ => ErrorKind::Generic,
    }
}

// Go: syscall/syscall_unix.go:Errno.Error
/// Go: `syscall.Errno(n).Error()`.
pub fn errno_text(n: i32) -> String {
    let table: &[&str] = if cfg!(target_os = "macos") {
        &ERRNO_DARWIN
    } else if cfg!(target_arch = "aarch64") {
        &ERRNO_LINUX
    } else {
        // linux/amd64's table ends before EHWPOISON (133).
        &ERRNO_LINUX[..133]
    };
    if n >= 0 && (n as usize) < table.len() {
        let s = table[n as usize];
        if !s.is_empty() {
            return s.to_string();
        }
    }
    format!("errno {n}")
}

// Generated from go1.27.1 src/syscall/zerrors_linux_arm64.go and zerrors_darwin_arm64.go.
const ERRNO_LINUX: [&str; 134] = [
    "",
    "operation not permitted",
    "no such file or directory",
    "no such process",
    "interrupted system call",
    "input/output error",
    "no such device or address",
    "argument list too long",
    "exec format error",
    "bad file descriptor",
    "no child processes",
    "resource temporarily unavailable",
    "cannot allocate memory",
    "permission denied",
    "bad address",
    "block device required",
    "device or resource busy",
    "file exists",
    "invalid cross-device link",
    "no such device",
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
    "numerical result out of range",
    "resource deadlock avoided",
    "file name too long",
    "no locks available",
    "function not implemented",
    "directory not empty",
    "too many levels of symbolic links",
    "",
    "no message of desired type",
    "identifier removed",
    "channel number out of range",
    "level 2 not synchronized",
    "level 3 halted",
    "level 3 reset",
    "link number out of range",
    "protocol driver not attached",
    "no CSI structure available",
    "level 2 halted",
    "invalid exchange",
    "invalid request descriptor",
    "exchange full",
    "no anode",
    "invalid request code",
    "invalid slot",
    "",
    "bad font file format",
    "device not a stream",
    "no data available",
    "timer expired",
    "out of streams resources",
    "machine is not on the network",
    "package not installed",
    "object is remote",
    "link has been severed",
    "advertise error",
    "srmount error",
    "communication error on send",
    "protocol error",
    "multihop attempted",
    "RFS specific error",
    "bad message",
    "value too large for defined data type",
    "name not unique on network",
    "file descriptor in bad state",
    "remote address changed",
    "can not access a needed shared library",
    "accessing a corrupted shared library",
    ".lib section in a.out corrupted",
    "attempting to link in too many shared libraries",
    "cannot exec a shared library directly",
    "invalid or incomplete multibyte or wide character",
    "interrupted system call should be restarted",
    "streams pipe error",
    "too many users",
    "socket operation on non-socket",
    "destination address required",
    "message too long",
    "protocol wrong type for socket",
    "protocol not available",
    "protocol not supported",
    "socket type not supported",
    "operation not supported",
    "protocol family not supported",
    "address family not supported by protocol",
    "address already in use",
    "cannot assign requested address",
    "network is down",
    "network is unreachable",
    "network dropped connection on reset",
    "software caused connection abort",
    "connection reset by peer",
    "no buffer space available",
    "transport endpoint is already connected",
    "transport endpoint is not connected",
    "cannot send after transport endpoint shutdown",
    "too many references: cannot splice",
    "connection timed out",
    "connection refused",
    "host is down",
    "no route to host",
    "operation already in progress",
    "operation now in progress",
    "stale file handle",
    "structure needs cleaning",
    "not a XENIX named type file",
    "no XENIX semaphores available",
    "is a named type file",
    "remote I/O error",
    "disk quota exceeded",
    "no medium found",
    "wrong medium type",
    "operation canceled",
    "required key not available",
    "key has expired",
    "key has been revoked",
    "key was rejected by service",
    "owner died",
    "state not recoverable",
    "operation not possible due to RF-kill",
    "memory page has hardware error",
];
const ERRNO_DARWIN: [&str; 107] = [
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
