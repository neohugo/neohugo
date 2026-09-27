//! Differential tests against `adv.txt` (tools/go-oracle/go-time/adv.go):
//! randomized layouts and values, zone-abbreviation parsing, synthetic and
//! mutated TZif data (leap seconds, v1/v2/v3, slim/fat, malformed), extreme
//! `Date`/`AddDate`/`Truncate`/`Round` arguments, `MarshalBinary` v1/v2
//! round trips and `ParseDuration` fuzz. Local is pinned to Asia/Bangkok.
//!
//! `GO_TIME_FIXTURES=<dir>` runs the larger corpus written by the oracle's
//! `-advonly <dir>` / `-big <dir>` flags instead of the checked-in one.

mod common;

use std::collections::HashMap;
use std::sync::Arc;

use common::*;
use go_time::{Duration, GoTimeExt, Location, Month, Time};

struct Locs {
    defs: HashMap<String, Arc<Location>>,
}

impl Locs {
    fn get(&self, id: &str) -> Arc<Location> {
        if let Some(l) = self.defs.get(id) {
            return l.clone();
        }
        loc(id)
    }
    fn t_in(&self, sec: i64, nsec: i64, id: &str) -> Time {
        go_time::unix(sec, nsec).in_loc(&self.get(id))
    }
}

fn b2s(b: bool) -> String {
    if b { "1" } else { "0" }.to_string()
}

fn zb(t: &Time) -> String {
    if t.go_is_zero() {
        "Z".to_string()
    } else {
        t.go_unix().to_string()
    }
}

#[test]
fn adversarial_fixture() {
    setup();
    let mut locs = Locs {
        defs: HashMap::new(),
    };
    let mut c = Checker::new("adversarial");
    let mut per_tag: HashMap<String, usize> = HashMap::new();
    for r in records("adv.txt") {
        *per_tag
            .entry(String::from_utf8_lossy(r.tag()).into_owned())
            .or_default() += 1;
        match r.tag() {
            b"DEF" => {
                let id = r.s(1);
                let res = go_time::load_location_from_tz_data(&id, &unhex(&r.s(2)));
                match res {
                    Ok(l) => {
                        c.check(l.name == id, || format!("{} name {:?}", show(&r), l.name));
                        locs.defs.insert(id, l);
                    }
                    Err(e) => c.check(false, || format!("{} got error {}", show(&r), e)),
                }
            }
            b"DEFE" => {
                let res = go_time::load_location_from_tz_data(&r.s(1), &unhex(&r.s(2)));
                c.check(
                    matches!(&res, Err(e) if e.error().as_bytes() == r.b(3)),
                    || format!("{} got {:?}", show(&r), res.map(|l| l.name.clone())),
                );
            }
            b"F" => {
                let t = locs.t_in(r.i(2), r.i(3), &r.s(1));
                let got = t.format_bytes(r.b(4));
                c.check(got == r.b(5), || {
                    format!("{} got {:?}", show(&r), String::from_utf8_lossy(&got))
                });
                let mut ap = b"pre".to_vec();
                t.append_format(&mut ap, r.b(4));
                c.check(ap[3..] == *r.b(5), || format!("{} AppendFormat", show(&r)));
            }
            b"S" => {
                let t = locs.t_in(r.i(2), r.i(3), &r.s(1));
                let s = t.string();
                c.check(s.as_bytes() == r.b(4), || {
                    format!("{} String got {:?}", show(&r), s)
                });
                let g = t.go_string();
                c.check(g.as_bytes() == r.b(5), || {
                    format!("{} GoString got {:?}", show(&r), g)
                });
            }
            b"P" => {
                let res = if r.s(1) == "Parse" {
                    go_time::parse(r.b(3), r.b(4))
                } else {
                    go_time::parse_in_location(r.b(3), r.b(4), &locs.get(&r.s(2)))
                };
                match (&res, r.b(5)) {
                    (Ok(t), b"OK") => {
                        let got = time_fields(t);
                        c.check(fields_eq(&r, 6, &got), || {
                            format!("{} got {:?}", show(&r), got)
                        });
                    }
                    (Err(e), b"ERR") => {
                        let msg = e.error();
                        c.check(msg.as_bytes() == r.b(6), || {
                            format!("{} got error {:?}", show(&r), msg)
                        });
                    }
                    (Ok(t), _) => c.check(false, || {
                        format!("{} got OK {:?}", show(&r), time_fields(t))
                    }),
                    (Err(e), _) => {
                        c.check(false, || format!("{} got ERR {:?}", show(&r), e.error()))
                    }
                }
            }
            b"L" => {
                let t = go_time::unix(r.i(2), 0).in_loc(&locs.get(&r.s(1)));
                let (name, off) = t.zone();
                let (st, en) = t.zone_bounds();
                let got = vec![name, off.to_string(), b2s(t.is_dst()), zb(&st), zb(&en)];
                c.check(fields_eq(&r, 3, &got), || {
                    format!("{} got {:?}", show(&r), got)
                });
            }
            b"LQ" => {
                // Go's zone name is not UTF-8 (unrepresentable, PORTING.md):
                // everything but the name must match.
                let t = go_time::unix(r.i(2), 0).in_loc(&locs.get(&r.s(1)));
                let (_, off) = t.zone();
                let (st, en) = t.zone_bounds();
                let got = vec![off.to_string(), b2s(t.is_dst()), zb(&st), zb(&en)];
                c.check(fields_eq(&r, 4, &got), || {
                    format!("{} got {:?}", show(&r), got)
                });
            }
            b"DT" => {
                let l = locs.get(&r.s(1));
                let t = go_time::date(
                    r.i(2),
                    Month(r.i(3)),
                    r.i(4),
                    r.i(5),
                    r.i(6),
                    r.i(7),
                    r.i(8),
                    &l,
                );
                let got = time_fields(&t);
                c.check(fields_eq(&r, 9, &got), || {
                    format!("{} got {:?}", show(&r), got)
                });
            }
            b"AD" => {
                let t = locs.t_in(r.i(2), r.i(3), &r.s(1));
                let got = time_fields(&t.add_date(r.i(4), r.i(5), r.i(6)));
                c.check(fields_eq(&r, 7, &got), || {
                    format!("{} got {:?}", show(&r), got)
                });
            }
            b"TR" => {
                let t = locs.t_in(r.i(2), r.i(3), &r.s(1));
                let d = Duration(r.i(4));
                let mut got = time_fields(&t.truncate(d));
                got.extend(time_fields(&t.round(d)));
                c.check(fields_eq(&r, 5, &got), || {
                    format!("{} got {:?}", show(&r), got)
                });
            }
            b"MB" => {
                let t = locs.t_in(r.i(2), r.i(3), &r.s(1));
                let or_err = |res: Result<Vec<u8>, go_time::TimeError>, hexed: bool| match res {
                    Ok(b) if hexed => hex(&b).into_bytes(),
                    Ok(b) => b,
                    Err(e) => format!("ERR:{}", e.error()).into_bytes(),
                };
                let got = [
                    or_err(t.marshal_binary(), true),
                    or_err(t.marshal_json(), false),
                    or_err(t.marshal_text(), false),
                ];
                c.check(
                    got[0] == r.b(4) && got[1] == r.b(5) && got[2] == r.b(6),
                    || {
                        format!(
                            "{} got {:?}",
                            show(&r),
                            got.iter()
                                .map(|g| String::from_utf8_lossy(g).into_owned())
                                .collect::<Vec<_>>()
                        )
                    },
                );
            }
            b"UB" => {
                let data = unhex(&r.s(1));
                let mut t = Time::zero();
                match (go_time::unmarshal_binary(&mut t, &data), r.b(2)) {
                    (Ok(()), b"OK") => {
                        let got = time_fields(&t);
                        c.check(fields_eq(&r, 3, &got), || {
                            format!("{} got {:?}", show(&r), got)
                        });
                    }
                    (Err(e), b"ERR") => c.check(e.error().as_bytes() == r.b(3), || {
                        format!("{} got {}", show(&r), e)
                    }),
                    (res, _) => c.check(false, || format!("{} got {:?}", show(&r), res)),
                }
            }
            b"PD" => {
                let res = go_time::parse_duration(r.b(1));
                match (&res, r.b(2)) {
                    (Ok(d), b"OK") => {
                        c.check(d.0 == r.i(3), || format!("{} got {}", show(&r), d.0))
                    }
                    (Err(e), b"ERR") => c.check(e.error().as_bytes() == r.b(3), || {
                        format!("{} got {:?}", show(&r), e.error())
                    }),
                    _ => c.check(false, || format!("{} got {:?}", show(&r), res)),
                }
            }
            other => panic!("unknown tag {:?}", String::from_utf8_lossy(other)),
        }
    }
    let mut tags: Vec<_> = per_tag.into_iter().collect();
    tags.sort();
    eprintln!("adversarial records per tag: {:?}", tags);
    c.finish();
}
