//! Port of `resources/resource/dates.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


use go_value::Time;

/// Go: `resource.Dated`.
pub trait Dated {
    fn date(&self) -> Time;
    fn lastmod(&self) -> Time;
    fn publish_date(&self) -> Time;
    fn expiry_date(&self) -> Time;
}

/// Go: `resource.Dates` (value struct).
#[derive(Clone, Debug)]
pub struct Dates {
    pub date: Time,
    pub lastmod: Time,
    pub publish_date: Time,
    pub expiry_date: Time,
}

impl Default for Dates {
    fn default() -> Self {
        Dates { date: Time::zero(), lastmod: Time::zero(), publish_date: Time::zero(), expiry_date: Time::zero() }
    }
}

/// Go: `resource.IsFuture(d)` / `IsExpired(d)` (against `htime.Now()`).
pub fn is_future(d: &dyn Dated) -> bool {
    todo!()
}

pub fn is_expired(d: &dyn Dated) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/dates.go (58 lines; 0/3 funcs executed)
//   types: Dated
//    L39-45: IsFuture(d Dated) bool
//    L48-53: IsExpired(d Dated) bool
//    L56-58: IsZeroDates(d Dated) bool
// ---------------------------------------------------------------------------
