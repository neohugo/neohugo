//! Differential tests against fixtures written by tools/go-oracle/go-json
//! (Go's encoding/json from the go1.27.1 toolchain).

mod common;

use common::*;
use go_json::{Decoder, Encoder, RawMessage, Token};
use go_value::Value;

struct Failures {
    what: String,
    n: usize,
    msgs: Vec<String>,
}

impl Failures {
    fn new(what: &str) -> Failures {
        Failures {
            what: what.to_string(),
            n: 0,
            msgs: Vec::new(),
        }
    }

    fn check(&mut self, ctx: &str, field: &str, got: &[u8], want: &[u8]) {
        if got != want {
            self.n += 1;
            if self.msgs.len() < 12 {
                self.msgs.push(format!(
                    "{ctx} [{field}]\n   got: {}\n  want: {}",
                    show(got),
                    show(want)
                ));
            }
        }
    }

    fn finish(self, total: usize) {
        eprintln!("{}: {} cases, {} mismatches", self.what, total, self.n);
        if self.n > 0 {
            panic!(
                "{} mismatches in {}:\n{}",
                self.n,
                self.what,
                self.msgs.join("\n")
            );
        }
    }
}

fn split_args(b: &[u8]) -> (&[u8], &[u8]) {
    let i = b.iter().position(|&c| c == 0).unwrap();
    (&b[..i], &b[i + 1..])
}

fn encode_with(
    v: &Value,
    escape: bool,
    prefix: &[u8],
    indent: &[u8],
) -> Result<Vec<u8>, go_json::Error> {
    let mut enc = Encoder::new(Vec::new());
    enc.set_escape_html(escape);
    enc.set_indent(prefix, indent);
    enc.encode(v)?;
    Ok(enc.into_inner())
}

#[test]
fn encode_fixture() {
    check_encode("encode.rec.gz", "encode");
}

/// `-mode hugoencode -n 2000 -seed 21`: Hugo-shaped values, including
/// maps.Params fields with omitzero (the Params.IsZero regression).
#[test]
fn hugo_encode_fixture() {
    check_encode("hugoencode.rec.gz", "hugoencode");
}

/// `-mode advencode -n 500 -seed 25`: Go's TestMarshalFloat inputs and
/// adversarial floats, strings, marshalers and indent arguments.
#[test]
fn adv_encode_fixture() {
    check_encode("advencode.rec.gz", "advencode");
}

/// `-mode exhaustencode -parts 128 -part 0 -seed 27`: every 128th batch of
/// the rune and 2-byte string enumeration.
#[test]
fn exhaust_encode_fixture() {
    check_encode("exhaustencode.rec.gz", "exhaustencode");
}

/// Whether to run a replay of `name`: the checked-in fixtures always exist;
/// a `$GO_JSON_FIXTURES` directory may hold only some corpora.
fn have_fixture(name: &str) -> bool {
    if std::path::Path::new(&fixture_path(name)).exists() {
        return true;
    }
    assert!(
        std::env::var("GO_JSON_FIXTURES").is_ok_and(|d| !d.is_empty()),
        "missing fixture {name}"
    );
    eprintln!("{name}: not in $GO_JSON_FIXTURES, skipped");
    false
}

fn check_encode(name: &str, what: &str) {
    if !have_fixture(name) {
        return;
    }
    let cs = cases(read_records(name));
    let mut f = Failures::new(what);
    for (i, c) in cs.iter().enumerate() {
        let vd = get(c, "value");
        let v = parse_value(vd);
        let ctx = format!("case {i} value {}", show(vd));
        f.check(
            &ctx,
            "marshal",
            &result(go_json::marshal(&v)),
            get(c, "marshal"),
        );
        f.check(
            &ctx,
            "nohtml",
            &result(encode_with(&v, false, b"", b"")),
            get(c, "nohtml"),
        );
        // marshal_with is the Encoder's bytes without the newline.
        f.check(
            &ctx,
            "marshal_with",
            &result(go_json::marshal_with(&v, false).map(|mut b| {
                b.push(b'\n');
                b
            })),
            get(c, "nohtml"),
        );
        let (p, ind) = split_args(get(c, "indentargs"));
        f.check(
            &ctx,
            "marshalindent",
            &result(go_json::marshal_indent(&v, p, ind)),
            get(c, "marshalindent"),
        );
        let ea = get(c, "encargs");
        let (p, ind) = split_args(&ea[1..]);
        f.check(
            &ctx,
            "encoder",
            &result(encode_with(&v, ea[0] == b'1', p, ind)),
            get(c, "encoder"),
        );
    }
    f.finish(cs.len());
}

fn stream_steps(
    input: &[u8],
    chunk: usize,
    sizes: &[usize],
    use_number: bool,
) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut dec = Decoder::new(ChunkReader::new(input, chunk, sizes));
    if use_number {
        dec.use_number();
    }
    let mut steps = Vec::new();
    for _ in 0..40 {
        let r = dec.decode();
        let mut rec = format!("{}|", dec.input_offset()).into_bytes();
        let stop = match &r {
            Ok(v) => {
                rec.push(b'D');
                dump(&mut rec, v);
                false
            }
            Err(e) => {
                rec.extend_from_slice(&err_rec(e));
                !matches!(e, go_json::Error::UnmarshalType(_))
            }
        };
        steps.push(rec);
        if stop {
            break;
        }
    }
    (steps, dec.buffered().to_vec())
}

fn token_steps(input: &[u8], chunk: usize, sizes: &[usize], use_number: bool) -> Vec<Vec<u8>> {
    let mut dec = Decoder::new(ChunkReader::new(input, chunk, sizes));
    if use_number {
        dec.use_number();
    }
    let mut steps = Vec::new();
    for _ in 0..200 {
        let r = dec.token();
        let off = dec.input_offset();
        let more = dec.more();
        let mut rec = format!("{}|{}|", off, more as u8).into_bytes();
        let stop = match &r {
            Ok(Token::Delim(d)) => {
                rec.extend_from_slice(&[b'D', *d]);
                false
            }
            Ok(Token::Value(v)) => {
                rec.push(b'V');
                dump(&mut rec, v);
                false
            }
            Err(e) => {
                rec.extend_from_slice(&err_rec(e));
                !matches!(e, go_json::Error::UnmarshalType(_))
            }
        };
        steps.push(rec);
        if stop {
            break;
        }
    }
    steps
}

/// Replays the oracle's mixedOps: interleaved Token/Decode/More/InputOffset.
fn mixed_steps(
    input: &[u8],
    ops: &[u8],
    chunk: usize,
    sizes: &[usize],
    use_number: bool,
) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut dec = Decoder::new(ChunkReader::new(input, chunk, sizes));
    if use_number {
        dec.use_number();
    }
    let mut steps = Vec::new();
    for &op in ops {
        let rec = match op {
            b'T' => match dec.token() {
                Ok(Token::Delim(d)) => vec![b'D', d],
                Ok(Token::Value(v)) => {
                    let mut r = vec![b'V'];
                    dump(&mut r, &v);
                    r
                }
                Err(e) => err_rec(&e),
            },
            b'D' => {
                let (v, err) = dec.decode_partial();
                unmarshal_rec(&v, &err)
            }
            b'M' => vec![b'M', b'0' + dec.more() as u8],
            _ => format!("O{}", dec.input_offset()).into_bytes(),
        };
        steps.push(rec);
    }
    (steps, dec.buffered().to_vec())
}

fn check_mixed(
    f: &mut Failures,
    ctx: &str,
    c: &[(String, Vec<u8>)],
    input: &[u8],
    chunk: usize,
    sizes: &[usize],
    use_number: bool,
) {
    let ops = get(c, "mixargs");
    let (steps, buffered) = mixed_steps(input, ops, chunk, sizes, use_number);
    let want = get_all(c, "mstep");
    f.check(
        ctx,
        "mstep-count",
        steps.len().to_string().as_bytes(),
        want.len().to_string().as_bytes(),
    );
    for (k, (g, w)) in steps.iter().zip(want.iter()).enumerate() {
        f.check(ctx, &format!("mstep {k} ({})", ops[k] as char), g, w);
    }
    f.check(ctx, "mbuffered", &buffered, get(c, "mbuffered"));
}

#[test]
fn text_fixture() {
    check_text("text.rec.gz", "text");
}

/// `-mode exhausttext -parts 128 -part 0 -seed 23`: every 128th input of
/// the enumeration of short inputs, string escapes and number shapes.
#[test]
fn exhaust_text_fixture() {
    check_text("exhausttext.rec.gz", "exhausttext");
}

/// `-mode numtext -n 400 -seed 22`: extreme number literals.
#[test]
fn num_text_fixture() {
    check_text("numtext.rec.gz", "numtext");
}

/// `-mode bigtext -n 10 -seed 26`: 4-400 KB documents streamed with chunk
/// sizes around the decoder's buffer sizes.
#[test]
fn big_text_fixture() {
    check_text("bigtext.rec.gz", "bigtext");
}

/// `-mode vartext -n 100 -seed 28`: advtext inputs streamed through readers
/// with cycles of read sizes.
#[test]
fn var_text_fixture() {
    check_text("vartext.rec.gz", "vartext");
}

fn check_text(name: &'static str, what: &'static str) {
    if !have_fixture(name) {
        return;
    }
    // Deeply nested inputs (10000 levels) need more stack than a test thread has.
    std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || text_fixture_body(name, what))
        .unwrap()
        .join()
        .unwrap();
}

fn text_fixture_body(name: &str, what: &str) {
    let cs = cases(read_records(name));
    let mut f = Failures::new(what);
    for (i, c) in cs.iter().enumerate() {
        let input = get(c, "input");
        let ctx = format!("case {i} input {}", show(input));

        let valid = if go_json::valid(input) { b"1" } else { b"0" };
        f.check(&ctx, "valid", valid, get(c, "valid"));

        let mut dst = b"pre".to_vec();
        let r = go_json::compact(&mut dst, input);
        if r.is_err() {
            f.check(&ctx, "compact-dst", &dst, b"pre");
        }
        f.check(&ctx, "compact", &result(r.map(|_| dst)), get(c, "compact"));

        let raw = Value::object(RawMessage(Some(input.to_vec())));
        f.check(
            &ctx,
            "rawmsg",
            &result(go_json::marshal(&raw)),
            get(c, "rawmsg"),
        );

        let (p, ind) = split_args(get(c, "indentargs"));
        let mut dst = b"pre".to_vec();
        let r = go_json::indent(&mut dst, input, p, ind);
        if r.is_err() {
            f.check(&ctx, "indent-dst", &dst, b"pre");
        }
        f.check(&ctx, "indent", &result(r.map(|_| dst)), get(c, "indent"));

        let mut dst = Vec::new();
        go_json::html_escape(&mut dst, input);
        f.check(&ctx, "htmlescape", &dst, get(c, "htmlescape"));

        let (v, err) = go_json::unmarshal_partial(input);
        f.check(
            &ctx,
            "unmarshal",
            &unmarshal_rec(&v, &err),
            get(c, "unmarshal"),
        );
        let (v, err) = go_json::unmarshal_map_partial(input);
        f.check(
            &ctx,
            "unmarshalmap",
            &unmarshal_rec(&v, &err),
            get(c, "unmarshalmap"),
        );

        // "chunk useNumber [read sizes...]" (-mode vartext adds the sizes).
        let sa = std::str::from_utf8(get(c, "streamargs")).unwrap();
        let mut sa = sa.split(' ');
        let chunk: usize = sa.next().unwrap().parse().unwrap();
        let use_number = sa.next().unwrap() == "1";
        let sizes: Vec<usize> = sa.map(|s| s.parse().unwrap()).collect();
        let (steps, buffered) = stream_steps(input, chunk, &sizes, use_number);
        let want = get_all(c, "dstep");
        f.check(
            &ctx,
            "dstep-count",
            steps.len().to_string().as_bytes(),
            want.len().to_string().as_bytes(),
        );
        for (k, (g, w)) in steps.iter().zip(want.iter()).enumerate() {
            f.check(&ctx, &format!("dstep {k}"), g, w);
        }
        f.check(&ctx, "buffered", &buffered, get(c, "buffered"));

        let steps = token_steps(input, chunk, &sizes, use_number);
        let want = get_all(c, "tstep");
        f.check(
            &ctx,
            "tstep-count",
            steps.len().to_string().as_bytes(),
            want.len().to_string().as_bytes(),
        );
        for (k, (g, w)) in steps.iter().zip(want.iter()).enumerate() {
            f.check(&ctx, &format!("tstep {k}"), g, w);
        }

        check_mixed(&mut f, &ctx, c, input, chunk, &sizes, use_number);
    }
    f.finish(cs.len());
}

#[test]
fn real_fixture() {
    if !have_fixture("real.rec.gz") {
        return;
    }
    let cs = cases(read_records("real.rec.gz"));
    let mut f = Failures::new("real");
    for c in &cs {
        let name = String::from_utf8_lossy(get(c, "name")).into_owned();
        let input = get(c, "input");
        let (v, err) = go_json::unmarshal_partial(input);
        let want = get(c, "unmarshal");
        if want[0] == b'E' {
            f.check(&name, "unmarshal", &err_rec(err.as_ref().unwrap()), want);
            continue;
        }
        f.check(&name, "unmarshal", &unmarshal_rec(&v, &err), want);
        f.check(
            &name,
            "marshal",
            &result(go_json::marshal(&v)),
            get(c, "marshal"),
        );
        f.check(
            &name,
            "marshalindent",
            &result(go_json::marshal_indent(&v, "", "  ")),
            get(c, "marshalindent"),
        );
        f.check(
            &name,
            "encoder",
            &result(encode_with(&v, false, b"", b"  ")),
            get(c, "encoder"),
        );
        f.check(
            &name,
            "encoderhtml",
            &result(encode_with(&v, true, b"", b"")),
            get(c, "encoderhtml"),
        );
        let mut dst = Vec::new();
        let r = go_json::indent(&mut dst, input, "", "\t");
        f.check(&name, "indent", &result(r.map(|_| dst)), get(c, "indent"));
        let mut dst = Vec::new();
        let r = go_json::compact(&mut dst, input);
        f.check(&name, "compact", &result(r.map(|_| dst)), get(c, "compact"));
        // Streaming reads with several chunk sizes: the records come in
        // groups of mixargs, mstep*, mbuffered.
        let mut group: Vec<(String, Vec<u8>)> = Vec::new();
        let mut groups = Vec::new();
        for (n, d) in c.iter() {
            match n.as_str() {
                "mixargs" => group = vec![(n.clone(), d.clone())],
                "mstep" => group.push((n.clone(), d.clone())),
                "mbuffered" => {
                    group.push((n.clone(), d.clone()));
                    groups.push(std::mem::take(&mut group));
                }
                _ => {}
            }
        }
        for g in &groups {
            let chunk: usize = std::str::from_utf8(get(g, "mixargs"))
                .unwrap()
                .parse()
                .unwrap();
            let mut g2 = g.clone();
            g2[0].1 = b"ODOTTTMOTOD".to_vec();
            check_mixed(
                &mut f,
                &format!("{name} chunk {chunk}"),
                &g2,
                input,
                chunk,
                &[],
                chunk == 7,
            );
        }

        let m = go_json::unmarshal_map(input);
        let want = get(c, "unmarshalmap");
        match m {
            Ok(m) => {
                let mut b = vec![b'O'];
                dump(&mut b, &m);
                f.check(&name, "unmarshalmap", &b, want);
            }
            Err(e) => f.check(&name, "unmarshalmap", &err_rec(&e), want),
        }
    }
    f.finish(cs.len());
}

/// The golden site's index.json is `jsonify` output (Encoder with HTML
/// escaping, trailing newline trimmed): decoding and re-encoding it must
/// give the same bytes.
#[test]
fn golden_index_json_roundtrip() {
    if !have_fixture("real.rec.gz") {
        return;
    }
    let cs = cases(read_records("real.rec.gz"));
    let mut seen = 0;
    for c in &cs {
        let name = String::from_utf8_lossy(get(c, "name")).into_owned();
        if !name.starts_with("golden-") {
            continue;
        }
        seen += 1;
        let input = get(c, "input");
        let v = go_json::unmarshal(input).unwrap();
        let mut enc = Encoder::new(Vec::new());
        enc.set_escape_html(true);
        enc.encode(&v).unwrap();
        let mut out = enc.into_inner();
        assert_eq!(out.pop(), Some(b'\n'));
        assert!(out == input, "{name}: re-encoded bytes differ");
    }
    assert!(seen > 0, "no golden index.json in real.rec.gz");
}
