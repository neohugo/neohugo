//! Port of `common/loggers/logger.go`, `common/loggers/loggerglobal.go`, `common/loggers/handlerdefault.go`, `common/loggers/handlersmisc.go`, `common/loggers/handlerterminal.go`.
//!
//! MINIMAL: levels + counters + stderr output; ERROR count fails the build
//!
//! Owner: Wave B task T02 (common-paths-text).

//! MINIMAL port of `common/loggers` over `bep/logg`: levelled logging to stderr with per-level
//! counters. The build fails (non-zero exit) if any ERROR was logged (Go:
//! `hugo_sites_build.go:212-223`). Message texts are not part of parity. What is kept from Go:
//! entries below the logger's level are dropped (not counted); `ignoreLogs` statement ids
//! (`Erroridf`/`Warnidf`) are dropped before counting; with a distinct level, a repeated entry
//! (same level, message and fields) at or above it is dropped before counting; messages are
//! whitespace-trimmed and printed without ANSI colours as `LEVEL message fields`. Messages are
//! formatted by the caller (Rust `format!`), not with Go's `fmt` verbs.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};

/// Go: `logg.Level` (`LevelTrace` = 1 … `LevelError` = 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    // Go: common/loggers/handlerdefault.go:levelString
    fn prefix(self) -> &'static str {
        match self {
            Level::Trace => "TRACE",
            Level::Debug => "DEBUG",
            Level::Info => "INFO ",
            Level::Warn => "WARN ",
            Level::Error => "ERROR",
        }
    }
}

/// Where log output goes (Go: `io.Writer`).
pub type LogSink = Arc<Mutex<dyn Write + Send>>;

/// Go: `loggers.Options`.
#[derive(Clone)]
pub struct Options {
    pub level: Level,
    /// Go: `StdOut` (default os.Stdout).
    pub std_out: Option<LogSink>,
    /// Go: `StdErr` (default os.Stderr); every log line goes here.
    pub std_err: Option<LogSink>,
    /// Go: `DistinctLevel` (0 = none).
    pub distinct_level: Option<Level>,
    /// Go: `StoreErrors` — keep ERROR messages for [`Logger::errors`].
    pub store_errors: bool,
    /// Go: `SuppressStatements` (the `ignoreLogs` ids).
    pub suppress_statements: BTreeSet<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            level: Level::Warn,
            std_out: None,
            std_err: None,
            distinct_level: None,
            store_errors: false,
            suppress_statements: BTreeSet::new(),
        }
    }
}

/// Go: `loggers.Logger`.
#[derive(Clone)]
pub struct Logger {
    inner: Arc<LoggerInner>,
}

struct LoggerInner {
    level: Level,
    std_out: LogSink,
    std_err: LogSink,
    distinct_level: Option<Level>,
    store_errors: bool,
    suppress_statements: BTreeSet<String>,
    counters: Mutex<BTreeMap<Level, u64>>,
    seen: Mutex<BTreeSet<(Level, String, Option<String>)>>,
    errors: Mutex<String>,
}

// Go: common/loggers/logger.go:FieldNameStatementID etc. (only the statement id field is used)
const IDF_SUPPRESS: &str = "You can suppress this";

impl Logger {
    /// The site logger (Go `hugolib/site.go`: `Level`, `DistinctLevel: Warn`,
    /// `SuppressStatements: ignoreLogs`).
    pub fn new(level: Level, ignored_ids: BTreeSet<String>) -> Self {
        Self::with_options(Options {
            level,
            distinct_level: Some(Level::Warn),
            suppress_statements: ignored_ids,
            ..Default::default()
        })
    }

    /// Go: `loggers.New(opts)`.
    // Go: common/loggers/logger.go:New
    pub fn with_options(opts: Options) -> Self {
        let std_out = opts
            .std_out
            .unwrap_or_else(|| Arc::new(Mutex::new(io::stdout())));
        let std_err = opts
            .std_err
            .unwrap_or_else(|| Arc::new(Mutex::new(io::stderr())));
        Logger {
            inner: Arc::new(LoggerInner {
                level: opts.level,
                std_out,
                std_err,
                distinct_level: opts.distinct_level,
                store_errors: opts.store_errors,
                suppress_statements: opts.suppress_statements,
                counters: Mutex::new(BTreeMap::new()),
                seen: Mutex::new(BTreeSet::new()),
                errors: Mutex::new(String::new()),
            }),
        }
    }

    /// Go: `loggers.NewDefault()`.
    // Go: common/loggers/logger.go:NewDefault
    pub fn new_default() -> Self {
        Self::with_options(Options {
            distinct_level: Some(Level::Warn),
            level: Level::Warn,
            ..Default::default()
        })
    }

    /// Go: `loggers.NewTrace()`.
    // Go: common/loggers/logger.go:NewTrace
    pub fn new_trace() -> Self {
        Self::with_options(Options {
            distinct_level: Some(Level::Warn),
            level: Level::Trace,
            ..Default::default()
        })
    }

    /// One logg entry through Go's handler chain: level filter, suppressed statements, distinct
    /// entries, counters, (trace-only filter), whitespace trimmer, output, stored errors.
    fn log(&self, level: Level, msg: &str, statement_id: Option<&str>) {
        self.log_entry(level, None, msg, statement_id);
    }

    /// Go: `logger.WithLevel(level).WithField(loggers.FieldNameCmd, cmd).Logf("%s", msg)`: the
    /// handlers print the command field as a `<cmd>: ` prefix of the message (also in the stored
    /// errors), and the distinct-entries handler hashes the fields with the message.
    // Go: common/loggers/handlerterminal.go:(*noAnsiEscapeHandler).HandleLog
    pub fn logf_cmd(&self, level: Level, cmd: &str, msg: impl AsRef<str>) {
        self.log_entry(level, Some(cmd), msg.as_ref(), None);
    }

    fn log_entry(&self, level: Level, cmd: Option<&str>, msg: &str, statement_id: Option<&str>) {
        let inner = &*self.inner;
        // logg: entries below the logger level are disabled.
        if level < inner.level {
            return;
        }
        // Go: common/loggers/handlersmisc.go:(*suppressStatementsHandler).HandleLog
        if let Some(id) = statement_id
            && inner.suppress_statements.contains(id)
        {
            return;
        }
        // Go: common/loggers/handlersmisc.go:(*logOnceHandler).HandleLog
        if let Some(threshold) = inner.distinct_level
            && level >= threshold
        {
            // (The fields are part of Go's hash: the command prefix stands for them.)
            let key = (
                level,
                format!("{}\x00{msg}", cmd.unwrap_or("")),
                statement_id.map(str::to_string),
            );
            let mut seen = inner.seen.lock().unwrap();
            if !seen.insert(key) {
                return;
            }
        }
        // Go: common/loggers/handlersmisc.go:(*logLevelCounter).HandleLog
        *inner.counters.lock().unwrap().entry(level).or_insert(0) += 1;

        if inner.level == Level::Trace && level != Level::Trace {
            // Trace is used during development only, and it's useful to
            // only see the trace messages.
            return;
        }

        // Go: common/loggers/handlersmisc.go:whiteSpaceTrimmer
        let msg = go_unicode::strings::trim_space(msg.as_bytes());

        // Go: common/loggers/handlerterminal.go:(*noAnsiEscapeHandler).HandleLog
        let prefix = match cmd {
            Some(c) if !c.is_empty() => format!("{c}: "),
            _ => String::new(),
        };
        let mut line = format!("{} {prefix}", level.prefix()).into_bytes();
        line.extend_from_slice(msg);
        line.push(b'\n');
        {
            let mut w = inner.std_err.lock().unwrap();
            let _ = w.write_all(&line);
        }

        if inner.store_errors && level >= Level::Error {
            // The no-level-prefix handler writing to Go's errors builder.
            let mut e = inner.errors.lock().unwrap();
            e.push_str(&prefix);
            e.push_str(std::str::from_utf8(msg).expect("trimmed at char boundaries"));
            e.push('\n');
        }
    }

    // Go: common/loggers/logger.go:(*logAdapter).Errorf
    pub fn errorf(&self, msg: impl AsRef<str>) {
        self.log(Level::Error, msg.as_ref(), None);
    }

    // Go: common/loggers/logger.go:(*logAdapter).Warnf
    pub fn warnf(&self, msg: impl AsRef<str>) {
        self.log(Level::Warn, msg.as_ref(), None);
    }

    // Go: common/loggers/logger.go:(*logAdapter).Infof
    pub fn infof(&self, msg: impl AsRef<str>) {
        self.log(Level::Info, msg.as_ref(), None);
    }

    // Go: common/loggers/logger.go:(*logAdapter).Debugf
    pub fn debugf(&self, msg: impl AsRef<str>) {
        self.log(Level::Debug, msg.as_ref(), None);
    }

    // Go: common/loggers/logger.go:(*logAdapter).Trace
    pub fn trace(&self, msg: impl AsRef<str>) {
        self.log(Level::Trace, msg.as_ref(), None);
    }

    /// Go: `Logger.Erroridf(id, format, args...)` — suppressible with `ignoreLogs = ['<id>']`.
    // Go: common/loggers/logger.go:(*logAdapter).Erroridf
    pub fn erroridf(&self, id: &str, msg: impl AsRef<str>) {
        self.idf(Level::Error, "error", id, msg.as_ref());
    }

    /// Go: `Logger.Warnidf`.
    // Go: common/loggers/logger.go:(*logAdapter).Warnidf
    pub fn warnidf(&self, id: &str, msg: impl AsRef<str>) {
        self.idf(Level::Warn, "warning", id, msg.as_ref());
    }

    fn idf(&self, level: Level, what: &str, id: &str, msg: &str) {
        let id = String::from_utf8(go_unicode::strings::to_lower(id.as_bytes()).into_owned())
            .expect("ToLower of valid UTF-8 is valid UTF-8");
        let msg = format!("{msg}{}", idf_info_statement(what, &id));
        self.log(level, &msg, Some(&id));
    }

    /// Go: `Deprecatef(fail, format, args...)`.
    // Go: common/loggers/logger.go:(*logAdapter).Deprecatef
    pub fn deprecatef(&self, fail: bool, msg: impl AsRef<str>) {
        let msg = format!("DEPRECATED: {}", msg.as_ref());
        if fail {
            self.errorf(msg);
        } else {
            self.warnf(msg);
        }
    }

    /// Go: `Printf` — to stdout, with a trailing newline added if not present.
    // Go: common/loggers/logger.go:(*logAdapter).Printf
    pub fn printf(&self, msg: impl AsRef<str>) {
        let mut msg = msg.as_ref().to_string();
        if !msg.ends_with('\n') {
            msg.push('\n');
        }
        let _ = self.inner.std_out.lock().unwrap().write_all(msg.as_bytes());
    }

    /// Go: `Println` — to stdout.
    // Go: common/loggers/logger.go:(*logAdapter).Println
    pub fn println(&self, msg: impl AsRef<str>) {
        let _ = writeln!(self.inner.std_out.lock().unwrap(), "{}", msg.as_ref());
    }

    /// Go: `Level()`.
    // Go: common/loggers/logger.go:(*logAdapter).Level
    pub fn level(&self) -> Level {
        self.inner.level
    }

    /// Go: `LoggCount(level)`.
    // Go: common/loggers/logger.go:(*logAdapter).LoggCount
    pub fn logg_count(&self, level: Level) -> u64 {
        self.inner
            .counters
            .lock()
            .unwrap()
            .get(&level)
            .copied()
            .unwrap_or(0)
    }

    pub fn log_counter_errors(&self) -> u64 {
        self.logg_count(Level::Error)
    }

    pub fn log_counter_warnings(&self) -> u64 {
        self.logg_count(Level::Warn)
    }

    /// Go: `Errors()` — the stored ERROR messages (with `StoreErrors`).
    // Go: common/loggers/logger.go:(*logAdapter).Errors
    pub fn errors(&self) -> String {
        self.inner.errors.lock().unwrap().clone()
    }

    /// Go: `Reset()` — clears the counters, the stored errors and the distinct entries.
    // Go: common/loggers/logger.go:(*logAdapter).Reset
    pub fn reset(&self) {
        self.inner.counters.lock().unwrap().clear();
        self.inner.errors.lock().unwrap().clear();
        self.inner.seen.lock().unwrap().clear();
    }
}

/// Go: `loggers.LevelLoggerToWriter(l)` — an `io.Writer` that logs every write as one entry.
pub struct LevelWriter {
    logger: Logger,
    level: Level,
}

impl Logger {
    // Go: common/loggers/logger.go:LevelLoggerToWriter
    pub fn level_writer(&self, level: Level) -> LevelWriter {
        LevelWriter {
            logger: self.clone(),
            level,
        }
    }

    /// Go: `PrintTimerIfDelayed` — prints `<name> in <ms> ms` to stderr when at least 500 ms
    /// have passed since `start`.
    // Go: common/loggers/logger.go:(*logAdapter).PrintTimerIfDelayed
    pub fn print_timer_if_delayed(&self, start: std::time::Instant, name: &str) {
        let milli = start.elapsed().as_millis();
        if milli < 500 {
            return;
        }
        let _ = write!(self.inner.std_err.lock().unwrap(), "{name} in {milli} ms");
    }

    /// Go: `loggers.TimeTrackf(l, start, fields, format, ...)` at `level` (timing text only).
    // Go: common/loggers/logger.go:TimeTrackf
    pub fn time_trackf(&self, level: Level, start: std::time::Instant, msg: impl AsRef<str>) {
        let msg = format!("{} duration {:?}", msg.as_ref(), start.elapsed());
        self.log(level, &msg, None);
    }
}

impl Write for LevelWriter {
    // Go: common/loggers/logger.go:(logWriter).Write
    fn write(&mut self, p: &[u8]) -> io::Result<usize> {
        // Diagnostics only (log text), never page bytes.
        self.logger
            .log(self.level, &String::from_utf8_lossy(p), None);
        Ok(p.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Go: common/loggers/logger.go:(*logAdapter).idfInfoStatement
fn idf_info_statement(what: &str, id: &str) -> String {
    format!(
        "\n{IDF_SUPPRESS} {what} by adding the following to your site configuration:\nignoreLogs = ['{id}']"
    )
}

fn global() -> &'static Mutex<Logger> {
    static LOG: OnceLock<Mutex<Logger>> = OnceLock::new();
    // Go: common/loggers/loggerglobal.go:init (initGlobalLogger(logg.LevelWarn, false))
    LOG.get_or_init(|| {
        Mutex::new(Logger::with_options(Options {
            level: Level::Warn,
            distinct_level: Some(Level::Info),
            ..Default::default()
        }))
    })
}

/// Go: `loggers.SetGlobalLogger` — used in a few places in Hugo, e.g. deprecated functions.
// Go: common/loggers/loggerglobal.go:SetGlobalLogger
pub fn set_global_logger(logger: Logger) {
    *global().lock().unwrap() = logger;
}

/// Go: `loggers.Log()` — the global logger.
// Go: common/loggers/loggerglobal.go:Log
pub fn log() -> Logger {
    global().lock().unwrap().clone()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// MINIMAL port (levels, counters, stderr); items below marked OK exist in that reduced form
// (callers format messages; the `logg.LevelLogger` accessors are the level methods).
// Source: common/loggers/logger.go (385 lines; 19/37 funcs executed)
//   types: Options, Logger, logAdapter, logWriter
// OK L50-151: New(opts Options) Logger
// OK L154-160: NewDefault() Logger
// OK L162-168: NewTrace() Logger
// OK L170-172: LevelLoggerToWriter(l logg.LevelLogger) io.Writer (LevelWriter)
// OK L220-222: (l *logAdapter) Debug() logg.LevelLogger (debugf)
// OK L224-226: (l *logAdapter) Debugf(format string, v ...any)
// OK L228-230: (l *logAdapter) Debugln(v ...any) (debugf)
// OK L232-234: (l *logAdapter) Info() logg.LevelLogger (infof)
// OK L236-238: (l *logAdapter) InfoCommand(command string) logg.LevelLogger (infof)
// OK L240-242: (l *logAdapter) Infof(format string, v ...any)
// OK L244-246: (l *logAdapter) Infoln(v ...any) (infof)
// OK L248-250: (l *logAdapter) Level() logg.Level
// OK L252-256: (l *logAdapter) LoggCount(level logg.Level) int
// OK L258-260: (l *logAdapter) Logger() logg.Logger (the Logger itself)
// OK L262-264: (l *logAdapter) StdOut() io.Writer (Options.std_out)
// OK L266-268: (l *logAdapter) StdErr() io.Writer (Options.std_err)
// OK L272-279: (l *logAdapter) PrintTimerIfDelayed(start time.Time, name string)
// OK L281-287: (l *logAdapter) Printf(format string, v ...any)
// OK L289-291: (l *logAdapter) Println(v ...any)
// OK L293-295: (l *logAdapter) Reset()
// OK L297-299: (l *logAdapter) Warn() logg.LevelLogger (warnf)
// OK L301-303: (l *logAdapter) Warnf(format string, v ...any)
// OK L305-307: (l *logAdapter) WarnCommand(command string) logg.LevelLogger (warnf)
// OK L309-311: (l *logAdapter) Warnln(v ...any) (warnf)
// OK L313-315: (l *logAdapter) Error() logg.LevelLogger (errorf)
// OK L317-319: (l *logAdapter) Errorf(format string, v ...any)
// OK L321-323: (l *logAdapter) Errorln(v ...any) (errorf)
// OK L325-327: (l *logAdapter) Errors() string
// OK L329-333: (l *logAdapter) Erroridf(id, format string, v ...any)
// OK L335-339: (l *logAdapter) Warnidf(id, format string, v ...any)
// OK L341-343: (l *logAdapter) idfInfoStatement(what, id, format string) string
// OK L345-347: (l *logAdapter) Trace(s logg.StringFunc)
// OK L349-351: (l *logAdapter) sprint(v ...any) string (callers format)
// OK L353-360: (l *logAdapter) Deprecatef(fail bool, format string, v ...any)
// OK L366-369: (w logWriter) Write(p []byte) (n int, err error)
// OK L371-377: TimeTrackf(l logg.LevelLogger, start time.Time, fields logg.Fields, format string, a ...any)
//    L379-385: TimeTrackfn(fn func() (logg.LevelLogger, error)) error (timing only)
// Source: common/loggers/loggerglobal.go (62 lines; 4/4 funcs executed)
// OK L26-30: SetGlobalLogger(logger Logger)
// OK L32-47: initGlobalLogger(level logg.Level, panicOnWarnings bool) (no panicOnWarnings hook)
// OK L51-55: Log() Logger
// OK L60-62: init()
// Source: common/loggers/handlerdefault.go (106 lines; 0/2 funcs executed)
//   types: defaultHandler
//    L49-55: newDefaultHandler(outWriter, errWriter io.Writer) logg.Handler (no ANSI colours)
//    L68-106: (h *defaultHandler) HandleLog(e *logg.Entry) error (no ANSI colours)
// Source: common/loggers/handlersmisc.go (145 lines; 4/10 funcs executed)
//   types: logLevelCounter, logOnceHandler, stopHandler, suppressStatementsHandler
// OK L35-39: newLogLevelCounter() *logLevelCounter
// OK L41-46: newLogOnceHandler(threshold logg.Level) *logOnceHandler
// OK L48-52: newStopHandler(h ...logg.Handler) *stopHandler
// OK L54-58: newSuppressStatementsHandler(statements map[string]bool) *suppressStatementsHandler
// OK L65-70: (h *logLevelCounter) HandleLog(e *logg.Entry) error
// OK L80-94: (h *logOnceHandler) HandleLog(e *logg.Entry) error
// OK L96-100: (h *logOnceHandler) reset()
// OK L107-117: (h *stopHandler) HandleLog(e *logg.Entry) error
// OK L123-132: (h *suppressStatementsHandler) HandleLog(e *logg.Entry) error
// OK L135-145: whiteSpaceTrimmer() logg.Handler
// Source: common/loggers/handlerterminal.go (100 lines; 1/3 funcs executed)
//   types: noAnsiEscapeHandler
// OK L29-39: newNoAnsiEscapeHandler(outWriter, errWriter io.Writer, noLevelPrefix bool, predicate func(*logg.Entry) bool) *noAnsiEscapeHandler
// OK L49-93: (h *noAnsiEscapeHandler) HandleLog(e *logg.Entry) error
//    L98-100: stripANSI(s string) string (messages are printed as given)
// ---------------------------------------------------------------------------
