use std::io::Read;
fn main() {
    let cfg = std::env::args().nth(1).unwrap_or("default".into());
    let mut src = Vec::new();
    std::io::stdin().read_to_end(&mut src).unwrap();
    let md = goldmark_cfg(&cfg);
    let mut out: Vec<u8> = Vec::new();
    md.convert(&src, &mut out).unwrap();
    use std::io::Write;
    std::io::stdout().write_all(&out).unwrap();
}
#[path = "../tests/common/mod.rs"]
mod common;
fn goldmark_cfg(c: &str) -> goldmark::Markdown {
    common::new_markdown(c)
}
