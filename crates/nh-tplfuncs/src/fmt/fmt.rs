//! Port of `tpl/fmt/fmt.go`.
//!
//! Owner: Wave B task T18 (tplfuncs-data).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, Value};
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: Exact Go `fmt` semantics (go-fmt crate): `%!s(<nil>)`, Sprint spacing rule, `%q` of Stringers; `warnf`/`errorf` log (errorf makes the build fail).

/// Go: `fmt.Namespace` (template value `*fmt.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

impl Namespace {
    // Go: tpl/fmt:New
    /// Go also registers a build-start listener that resets the logger (watch mode only).
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    // Go: tpl/fmt:Errorf
    pub fn errorf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Errorf")?;
        let format = args::string(a, 0)?;
        // Go: tpl/fmt/fmt.go:Errorf
        self.d.log.errorf(lossy(&go_fmt::sprintf(&format, &a[1..])));
        Ok(Value::string(""))
    }

    // Go: tpl/fmt:Erroridf
    pub fn erroridf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Erroridf")?;
        let id = args::string(a, 0)?;
        let format = args::string(a, 1)?;
        // Go: tpl/fmt/fmt.go:Erroridf
        self.d.log.erroridf(
            &id.to_str_lossy(),
            lossy(&go_fmt::sprintf(&format, &a[2..])),
        );
        Ok(Value::string(""))
    }

    // Go: tpl/fmt:Errormf
    pub fn errormf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Errormf")?;
        let format = args::string(a, 1)?;
        // Go: tpl/fmt/fmt.go:Errormf
        let msg = self.logmf(&a[0], &format, &a[2..]);
        self.d.log.errorf(msg);
        Ok(Value::string(""))
    }

    // Go: tpl/fmt:Print
    pub fn print(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/fmt/fmt.go:Print
        Ok(Value::String(GoString::from(go_fmt::sprint(a))))
    }

    // Go: tpl/fmt:Printf
    pub fn printf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Printf")?;
        let format = args::string(a, 0)?;
        // Go: tpl/fmt/fmt.go:Printf
        Ok(Value::String(GoString::from(go_fmt::sprintf(
            &format,
            &a[1..],
        ))))
    }

    // Go: tpl/fmt:Println
    pub fn println(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        // Go: tpl/fmt/fmt.go:Println
        Ok(Value::String(GoString::from(go_fmt::sprintln(a))))
    }

    // Go: tpl/fmt:Warnf
    pub fn warnf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Warnf")?;
        let format = args::string(a, 0)?;
        // Go: tpl/fmt/fmt.go:Warnf
        self.d.log.warnf(lossy(&go_fmt::sprintf(&format, &a[1..])));
        Ok(Value::string(""))
    }

    // Go: tpl/fmt:Warnidf
    pub fn warnidf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Warnidf")?;
        let id = args::string(a, 0)?;
        let format = args::string(a, 1)?;
        // Go: tpl/fmt/fmt.go:Warnidf
        self.d.log.warnidf(
            &id.to_str_lossy(),
            lossy(&go_fmt::sprintf(&format, &a[2..])),
        );
        Ok(Value::string(""))
    }

    // Go: tpl/fmt:Warnmf
    pub fn warnmf(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "Warnmf")?;
        let format = args::string(a, 1)?;
        // Go: tpl/fmt/fmt.go:Warnmf
        let msg = self.logmf(&a[0], &format, &a[2..]);
        self.d.log.warnf(msg);
        Ok(Value::string(""))
    }

    // Go: tpl/fmt/fmt.go:logmf
    /// The message with the fields of `m` (sorted by name). The minimal logger has no
    /// structured fields: they are appended as ` name=value` (PORTING.md).
    fn logmf(&self, m: &Value, format: &[u8], a: &[Value]) -> String {
        let mm = nh_common::cast::caste::to_string_map(m);
        let mut msg = lossy(&go_fmt::sprintf(format, a));
        for (k, v) in &mm.entries {
            msg.push(' ');
            msg.push_str(&k.to_str_lossy());
            msg.push('=');
            msg.push_str(&lossy(&go_fmt::sprint(std::slice::from_ref(v))));
        }
        msg
    }
}

nh_common::go_methods!(Namespace {
    "Errorf" => |n, ctx, a| n.errorf(ctx, a),
    "Erroridf" => |n, ctx, a| n.erroridf(ctx, a),
    "Errormf" => |n, ctx, a| n.errormf(ctx, a),
    "Print" => |n, ctx, a| n.print(ctx, a),
    "Printf" => |n, ctx, a| n.printf(ctx, a),
    "Println" => |n, ctx, a| n.println(ctx, a),
    "Warnf" => |n, ctx, a| n.warnf(ctx, a),
    "Warnidf" => |n, ctx, a| n.warnidf(ctx, a),
    "Warnmf" => |n, ctx, a| n.warnmf(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*fmt.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/fmt/fmt.go (117 lines; 3/11 funcs executed)
//   types: Namespace
// OK L28-39: New(d *deps.Deps) *Namespace
// OK L47-49: (ns *Namespace) Print(args ...any) string
// OK L52-54: (ns *Namespace) Printf(format string, args ...any) string
// OK L57-59: (ns *Namespace) Println(args ...any) string
// OK L63-66: (ns *Namespace) Errorf(format string, args ...any) string
// OK L71-74: (ns *Namespace) Erroridf(id, format string, args ...any) string
// OK L78-81: (ns *Namespace) Warnf(format string, args ...any) string
// OK L86-89: (ns *Namespace) Warnidf(id, format string, args ...any) string
// OK L92-94: (ns *Namespace) Warnmf(m any, format string, args ...any) string
// OK L97-99: (ns *Namespace) Errormf(m any, format string, args ...any) string
// OK L101-117: (ns *Namespace) logmf(l logg.LevelLogger, m any, format string, args ...any) string
// ---------------------------------------------------------------------------
