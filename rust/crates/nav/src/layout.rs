//! Dates formatted with a Go time layout (`2006-01-02`), the pattern syntax of date-valued
//! related-content indices.

use std::fmt::Write as _;

use jiff::Zoned;

/// `d` formatted with the Go layout `layout`: the numeric and name elements (`2006`, `06`,
/// `01`, `1`, `Jan`, `January`, `02`, `2`, `_2`, `002`, `Mon`, `Monday`, `15`, `03`, `3`,
/// `04`, `4`, `05`, `5`, `PM`, `pm`, `MST`, `-0700`, `-07:00`, `Z0700`, `Z07:00`); other text
/// is kept.
#[must_use]
pub fn format(d: &Zoned, layout: &str) -> String {
    let mut out = String::new();
    let mut rest = layout;
    while !rest.is_empty() {
        let (text, len) = element(d, rest);
        match text {
            Some(t) => {
                out.push_str(&t);
                rest = &rest[len..];
            }
            None => {
                let c = rest.chars().next().expect("not empty");
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// The layout element at the start of `s`, formatted, and its length.
fn element(d: &Zoned, s: &str) -> (Option<String>, usize) {
    let lower_follows = |n: usize| s[n..].starts_with(|c: char| c.is_ascii_lowercase());
    let hour12 = || match d.hour() % 12 {
        0 => 12,
        h => h,
    };
    let offset = |colon: bool, z: bool| {
        let secs = d.offset().seconds();
        if z && secs == 0 {
            return "Z".to_owned();
        }
        let sign = if secs < 0 { '-' } else { '+' };
        let (h, m) = (secs.abs() / 3600, secs.abs() % 3600 / 60);
        let mut o = String::new();
        let _ = if colon {
            write!(o, "{sign}{h:02}:{m:02}")
        } else {
            write!(o, "{sign}{h:02}{m:02}")
        };
        o
    };
    // Longest elements first where one is a prefix of another.
    let table: &[(&str, &dyn Fn() -> Option<String>)] = &[
        ("January", &|| Some(d.strftime("%B").to_string())),
        ("Jan", &|| {
            (!lower_follows(3)).then(|| d.strftime("%b").to_string())
        }),
        ("Monday", &|| Some(d.strftime("%A").to_string())),
        ("Mon", &|| {
            (!lower_follows(3)).then(|| d.strftime("%a").to_string())
        }),
        ("MST", &|| Some(d.strftime("%Z").to_string())),
        ("2006", &|| Some(format!("{:04}", d.year()))),
        ("002", &|| Some(format!("{:03}", d.day_of_year()))),
        ("Z07:00", &|| Some(offset(true, true))),
        ("-07:00", &|| Some(offset(true, false))),
        ("Z0700", &|| Some(offset(false, true))),
        ("-0700", &|| Some(offset(false, false))),
        ("01", &|| Some(format!("{:02}", d.month()))),
        ("02", &|| Some(format!("{:02}", d.day()))),
        ("03", &|| Some(format!("{:02}", hour12()))),
        ("04", &|| Some(format!("{:02}", d.minute()))),
        ("05", &|| Some(format!("{:02}", d.second()))),
        ("06", &|| Some(format!("{:02}", d.year().rem_euclid(100)))),
        ("15", &|| Some(format!("{:02}", d.hour()))),
        ("_2", &|| {
            (!s.starts_with("_2006")).then(|| format!("{:>2}", d.day()))
        }),
        ("1", &|| Some(d.month().to_string())),
        ("2", &|| Some(d.day().to_string())),
        ("3", &|| Some(hour12().to_string())),
        ("4", &|| Some(d.minute().to_string())),
        ("5", &|| Some(d.second().to_string())),
        ("PM", &|| {
            Some(if d.hour() >= 12 { "PM" } else { "AM" }.to_owned())
        }),
        ("pm", &|| {
            Some(if d.hour() >= 12 { "pm" } else { "am" }.to_owned())
        }),
    ];
    for (name, f) in table {
        if s.starts_with(name)
            && let Some(t) = f()
        {
            return (Some(t), name.len());
        }
    }
    (None, 0)
}

#[cfg(test)]
mod tests {
    use super::format;

    #[test]
    fn go_layouts() {
        let d: jiff::Zoned = "2024-03-05T14:07:09+07:00[Asia/Bangkok]".parse().unwrap();
        assert_eq!(format(&d, "2006"), "2024");
        assert_eq!(format(&d, "2006-01"), "2024-03");
        assert_eq!(format(&d, "2006-01-02"), "2024-03-05");
        assert_eq!(format(&d, "Jan 2, 06 3:04PM"), "Mar 5, 24 2:07PM");
        assert_eq!(
            format(&d, "Monday 15:04:05 -07:00"),
            "Tuesday 14:07:09 +07:00"
        );
        assert_eq!(format(&d, "x_2y"), "x 5y");
    }
}
