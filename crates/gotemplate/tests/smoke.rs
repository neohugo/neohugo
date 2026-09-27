use go_value::{Map, MapType, Value};
use gotemplate::text::Template;

fn run(src: &str, data: &Value) -> String {
    let t = Template::new("x");
    match t.parse(src) {
        Err(e) => return format!("PARSE ERR: {e}"),
        Ok(_) => {}
    }
    let mut out = Vec::new();
    match t.execute(&mut out, data) {
        Ok(()) => String::from_utf8_lossy(&out).into_owned(),
        Err(e) => format!("{}|ERR: {e}", String::from_utf8_lossy(&out)),
    }
}

#[test]
fn smoke() {
    let mut m = Map::new(MapType::StringAny);
    m.insert("a", Value::int(1));
    m.insert("B", Value::string("x"));
    m.insert("l", Value::string_list(["p", "q"]));
    let d = Value::map(m);
    let cases = [
        ("hello {{.a}} {{.B}}", "hello 1 x"),
        ("{{range $i, $e := .l}}{{$i}}={{$e}};{{end}}", "0=p;1=q;"),
        ("{{if .missing}}y{{else}}n{{end}}", "n"),
        ("{{.missing}}", "<no value>"),
        ("{{printf \"%d-%s\" 3 .B}}", "3-x"),
        ("{{and 1 0 2}}|{{or 0 \"\" \"x\"}}|{{or 0 \"\"}}", "0|x|"),
        ("{{len \"héllo\"}}", "6"),
        ("{{range 3}}{{.}}{{end}}", "012"),
        ("{{define \"t\"}}[{{.}}]{{end}}{{template \"t\" .B}}", "[x]"),
        ("{{- 1 -}} {{2}}", "12"),
        ("{{ 017 }}|{{ 0b101 }}|{{ 1_000 }}|{{ 1.5e10 }}|{{ -0.0 }}|{{ 'a' }}", "15|5|1000|1.5e+10|-0|97"),
        ("{{html \"<a href='x'>\"}}", "&lt;a href=&#39;x&#39;&gt;"),
        ("{{urlquery \"a b&c\"}}", "a+b%26c"),
        ("{{index .l 1}}", "q"),
        ("{{slice \"abcdef\" 1 3}}", "bc"),
        ("{{eq 1 1}} {{lt 1 2}} {{ne \"a\" \"b\"}}", "true true true"),
        ("{{with .B}}w{{.}}{{end}}", "wx"),
        ("{{range .l}}{{if eq . \"q\"}}{{break}}{{end}}{{.}}{{end}}", "p"),
        ("{{.a.b}}", "|ERR: template: x:1:4: executing \"x\" at <.a.b>: can't evaluate field b in type int"),
        ("{{nofunc}}", "PARSE ERR: template: x:1: function \"nofunc\" not defined"),
        ("{{$x := 1}}{{$x = 2}}{{$x}}", "2"),
        ("{{print 1 2 \"a\" 3}}", "1 2a3"),
        ("{{js \"a'b<\"}}", "a\\'b\\u003C"),
    ];
    let mut bad = 0;
    for (src, want) in cases {
        let got = run(src, &d);
        if got != want {
            bad += 1;
            eprintln!("src {src:?}\n  got  {got:?}\n  want {want:?}");
        }
    }
    assert_eq!(bad, 0);
}
