//! Port of `common/loggers/logger.go`, `common/loggers/loggerglobal.go`, `common/loggers/handlerdefault.go`, `common/loggers/handlersmisc.go`, `common/loggers/handlerterminal.go`.
//!
//! MINIMAL: levels + counters + stderr output; ERROR count fails the build
//!
//! Owner: Wave B task T02 (common-paths-text).


//! MINIMAL port of `common/loggers`: levelled logging to stderr with counters. The build fails
//! (non-zero exit) if any ERROR was logged (Go: `hugo_sites_build.go:212-223`). Message texts are
//! not part of parity. `warnidf`/`erroridf` ids can be suppressed with `ignoreLogs`.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Go: `loggers.Logger`.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

struct LoggerInner {
    level: Level,
    errors: AtomicU64,
    warnings: AtomicU64,
    ignored_ids: BTreeSet<String>,
    once_ids: Mutex<BTreeSet<String>>,
}

impl Logger {
    pub fn new(level: Level, ignored_ids: BTreeSet<String>) -> Self {
        Logger {
            inner: Arc::new(LoggerInner {
                level,
                errors: AtomicU64::new(0),
                warnings: AtomicU64::new(0),
                ignored_ids,
                once_ids: Mutex::new(BTreeSet::new()),
            }),
        }
    }

    pub fn errorf(&self, msg: impl AsRef<str>) {
        self.inner.errors.fetch_add(1, Ordering::Relaxed);
        eprintln!("ERROR {}", msg.as_ref());
    }

    pub fn warnf(&self, msg: impl AsRef<str>) {
        self.inner.warnings.fetch_add(1, Ordering::Relaxed);
        if self.inner.level <= Level::Warn {
            eprintln!("WARN  {}", msg.as_ref());
        }
    }

    pub fn infof(&self, msg: impl AsRef<str>) {
        if self.inner.level <= Level::Info {
            eprintln!("INFO  {}", msg.as_ref());
        }
    }

    pub fn debugf(&self, msg: impl AsRef<str>) {
        if self.inner.level <= Level::Debug {
            eprintln!("DEBUG {}", msg.as_ref());
        }
    }

    /// Go: `Logger.Erroridf(id, format, args...)`.
    pub fn erroridf(&self, id: &str, msg: impl AsRef<str>) {
        if !self.inner.ignored_ids.contains(id) {
            self.errorf(msg);
        }
    }

    /// Go: `Logger.Warnidf`.
    pub fn warnidf(&self, id: &str, msg: impl AsRef<str>) {
        if !self.inner.ignored_ids.contains(id) {
            self.warnf(msg);
        }
    }

    pub fn log_counter_errors(&self) -> u64 {
        self.inner.errors.load(Ordering::Relaxed)
    }

    pub fn log_counter_warnings(&self) -> u64 {
        self.inner.warnings.load(Ordering::Relaxed)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/loggers/logger.go (385 lines; 19/37 funcs executed)
//   types: Options, Logger, logAdapter, logWriter
// EX L50-151: New(opts Options) Logger
// EX L154-160: NewDefault() Logger
//    L162-168: NewTrace() Logger
// EX L170-172: LevelLoggerToWriter(l logg.LevelLogger) io.Writer
//    L220-222: (l *logAdapter) Debug() logg.LevelLogger
// EX L224-226: (l *logAdapter) Debugf(format string, v ...any)
//    L228-230: (l *logAdapter) Debugln(v ...any)
//    L232-234: (l *logAdapter) Info() logg.LevelLogger
// EX L236-238: (l *logAdapter) InfoCommand(command string) logg.LevelLogger
//    L240-242: (l *logAdapter) Infof(format string, v ...any)
// EX L244-246: (l *logAdapter) Infoln(v ...any)
// EX L248-250: (l *logAdapter) Level() logg.Level
// EX L252-256: (l *logAdapter) LoggCount(level logg.Level) int
// EX L258-260: (l *logAdapter) Logger() logg.Logger
// EX L262-264: (l *logAdapter) StdOut() io.Writer
// EX L266-268: (l *logAdapter) StdErr() io.Writer
// EX L272-279: (l *logAdapter) PrintTimerIfDelayed(start time.Time, name string)
//    L281-287: (l *logAdapter) Printf(format string, v ...any)
// EX L289-291: (l *logAdapter) Println(v ...any)
//    L293-295: (l *logAdapter) Reset()
//    L297-299: (l *logAdapter) Warn() logg.LevelLogger
//    L301-303: (l *logAdapter) Warnf(format string, v ...any)
// EX L305-307: (l *logAdapter) WarnCommand(command string) logg.LevelLogger
//    L309-311: (l *logAdapter) Warnln(v ...any)
// EX L313-315: (l *logAdapter) Error() logg.LevelLogger
//    L317-319: (l *logAdapter) Errorf(format string, v ...any)
//    L321-323: (l *logAdapter) Errorln(v ...any)
//    L325-327: (l *logAdapter) Errors() string
//    L329-333: (l *logAdapter) Erroridf(id, format string, v ...any)
//    L335-339: (l *logAdapter) Warnidf(id, format string, v ...any)
//    L341-343: (l *logAdapter) idfInfoStatement(what, id, format string) string
// EX L345-347: (l *logAdapter) Trace(s logg.StringFunc)
// EX L349-351: (l *logAdapter) sprint(v ...any) string
//    L353-360: (l *logAdapter) Deprecatef(fail bool, format string, v ...any)
// EX L366-369: (w logWriter) Write(p []byte) (n int, err error)
// EX L371-377: TimeTrackf(l logg.LevelLogger, start time.Time, fields logg.Fields, format string, a ...any)
//    L379-385: TimeTrackfn(fn func() (logg.LevelLogger, error)) error
// Source: common/loggers/loggerglobal.go (62 lines; 4/4 funcs executed)
// EX L26-30: SetGlobalLogger(logger Logger)
// EX L32-47: initGlobalLogger(level logg.Level, panicOnWarnings bool)
// EX L51-55: Log() Logger
// EX L60-62: init()
// Source: common/loggers/handlerdefault.go (106 lines; 0/2 funcs executed)
//   types: defaultHandler
//    L49-55: newDefaultHandler(outWriter, errWriter io.Writer) logg.Handler
//    L68-106: (h *defaultHandler) HandleLog(e *logg.Entry) error
// Source: common/loggers/handlersmisc.go (145 lines; 4/10 funcs executed)
//   types: logLevelCounter, logOnceHandler, stopHandler, suppressStatementsHandler
// EX L35-39: newLogLevelCounter() *logLevelCounter
// EX L41-46: newLogOnceHandler(threshold logg.Level) *logOnceHandler
// EX L48-52: newStopHandler(h ...logg.Handler) *stopHandler
//    L54-58: newSuppressStatementsHandler(statements map[string]bool) *suppressStatementsHandler
//    L65-70: (h *logLevelCounter) HandleLog(e *logg.Entry) error
//    L80-94: (h *logOnceHandler) HandleLog(e *logg.Entry) error
//    L96-100: (h *logOnceHandler) reset()
//    L107-117: (h *stopHandler) HandleLog(e *logg.Entry) error
//    L123-132: (h *suppressStatementsHandler) HandleLog(e *logg.Entry) error
// EX L135-145: whiteSpaceTrimmer() logg.Handler
// Source: common/loggers/handlerterminal.go (100 lines; 1/3 funcs executed)
//   types: noAnsiEscapeHandler
// EX L29-39: newNoAnsiEscapeHandler(outWriter, errWriter io.Writer, noLevelPrefix bool, predicate func(*logg.Entry) bool) *noAnsiEscapeHandler
//    L49-93: (h *noAnsiEscapeHandler) HandleLog(e *logg.Entry) error
//    L98-100: stripANSI(s string) string
// ---------------------------------------------------------------------------
