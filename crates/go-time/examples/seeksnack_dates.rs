//! Prints the seeksnack date renderings computed by go-time, for a manual
//! cross-check against the golden site output (see PORTING.md).
use go_time::GoTimeExt;

fn main() {
    let utc = go_time::utc();
    for line in std::io::stdin().lines() {
        let v = line.unwrap();
        let t = match go_time::parse(go_time::RFC3339, &v) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let (y, m, d) = t.date();
        let (h, mi, s) = t.clock();
        let t = go_time::date(y, m, d, h, mi, s, t.nanosecond(), &utc);
        println!(
            "{}\t{}\t{}\t{}",
            t.format("Mon, 02 Jan 2006 15:04:05 -0700"),
            t.format("2006-01-02T15:04:05-07:00"),
            t.string(),
            t.format("Jan 2, 2006")
        );
    }
}
