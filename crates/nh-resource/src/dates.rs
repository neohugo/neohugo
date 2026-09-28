//! Port of `resources/resource/dates.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use go_time::GoTimeExt;
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
        Dates {
            date: Time::zero(),
            lastmod: Time::zero(),
            publish_date: Time::zero(),
            expiry_date: Time::zero(),
        }
    }
}

impl Dated for Dates {
    fn date(&self) -> Time {
        self.date.clone()
    }
    fn lastmod(&self) -> Time {
        self.lastmod.clone()
    }
    fn publish_date(&self) -> Time {
        self.publish_date.clone()
    }
    fn expiry_date(&self) -> Time {
        self.expiry_date.clone()
    }
}

/// Go: `resource.IsFuture(d)` — the publish date is after `htime.Now()`.
// Go: resources/resource/dates.go:IsFuture
pub fn is_future(d: &dyn Dated) -> bool {
    if d.publish_date().go_is_zero() {
        return false;
    }

    d.publish_date().go_after(&nh_common::htime::now())
}

/// Go: `resource.IsExpired(d)` — the expiry date is before `htime.Now()`.
// Go: resources/resource/dates.go:IsExpired
pub fn is_expired(d: &dyn Dated) -> bool {
    if d.expiry_date().go_is_zero() {
        return false;
    }
    d.expiry_date().go_before(&nh_common::htime::now())
}

/// Go: `resource.IsZeroDates(d)`.
// Go: resources/resource/dates.go:IsZeroDates
pub fn is_zero_dates(d: &dyn Dated) -> bool {
    d.date().go_is_zero()
        && d.lastmod().go_is_zero()
        && d.expiry_date().go_is_zero()
        && d.publish_date().go_is_zero()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource/dates.go (58 lines; 0/3 funcs executed)
//   types: Dated
// OK L39-45: IsFuture(d Dated) bool
// OK L48-53: IsExpired(d Dated) bool
// OK L56-58: IsZeroDates(d Dated) bool
// ---------------------------------------------------------------------------
