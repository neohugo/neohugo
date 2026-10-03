//! `babel`: the script on stdin with `--filename`, the result read from `--out-file`, the
//! config file and switches as arguments; an external source map is published next to the
//! script and the script's `sourceMappingURL` points at it.

use std::collections::BTreeMap;

use ssg_resources::Transform;
use ssg_resources::pipes::BabelOptions;

use super::{fake_tool, have_node, mini_site, project, real_tools};
use crate::support::MemSink;

/// A fake `@babel/cli`: writes `/* babel */` + the input to `--out-file`, and with
/// `--source-maps` a map file and a `sourceMappingURL` comment naming the temporary file.
const FAKE_BABEL: &str = r"
const fs = require('fs'), path = require('path');
const args = process.argv.slice(2);
fs.appendFileSync(path.join(process.env.HOME, 'tool-calls.log'), JSON.stringify({tool: 'babel', args: args.filter((a) => !a.startsWith('--out-file='))}) + '\n');
const out = args.find((a) => a.startsWith('--out-file=')).slice(11);
let code = '/* babel */\n' + fs.readFileSync(0, 'utf8');
if (args.includes('--source-maps')) {
  fs.writeFileSync(out + '.map', JSON.stringify({version: 3, sources: ['x'], mappings: ''}));
  code += '//# sourceMappingURL=' + path.basename(out) + '.map\n';
}
fs.writeFileSync(out, code);
";

const SITE: [(&str, &str); 3] = [
    (
        "config.toml",
        "baseURL = \"https://example.org/\"\n[security.exec]\nallow = ['^babel$']\n",
    ),
    ("babel.config.js", "module.exports = {};\n"),
    ("assets/js/app.js", "const f = () => 1;\n"),
];

#[test]
fn babel_fake_tool() {
    if !have_node("babel_fake_tool") {
        return;
    }
    let site = mini_site(&SITE);
    let tmp = tempfile::tempdir().unwrap();
    let modules = tmp.path().join("node_modules");
    fake_tool(&modules, "babel", FAKE_BABEL);
    let p = project(site.path(), |env| env.tools.node_modules = vec![modules]);
    let src = p.asset("js/app.js");
    let opts = BabelOptions::from_json(&serde_json::json!({
        "sourceMap": "external", "minified": true, "noComments": true, "compact": false
    }))
    .unwrap();
    let id = p.store.transform(src, Transform::Babel(opts)).unwrap();
    let code = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
    assert_eq!(
        code,
        "/* babel */\nconst f = () => 1;\n//# sourceMappingURL=app.js.map\n"
    );
    let config = p.dir.join("babel.config.js");
    assert_eq!(
        p.tool_calls().trim(),
        format!(
            r#"{{"tool":"babel","args":["--config-file","{}","--source-maps","--minified","--no-comments","--compact=false","--filename=js/app.js"]}}"#,
            config.display()
        )
    );
    // The script and its map are published together.
    let sink = MemSink::default();
    let r = p.store.resource(id);
    p.store.publish([r.rel_permalink.as_str()], &sink).unwrap();
    let files: BTreeMap<String, Vec<u8>> = sink.0.into_inner().unwrap();
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        ["js/app.js", "js/app.js.map"]
    );
    // No temporary files are left.
    let leftovers: Vec<_> = std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&format!("ssg-babel-{}-", std::process::id()))
        })
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn babel_real_tool() {
    let Some(dirs) = real_tools("babel", "babel_real_tool") else {
        return;
    };
    let site = mini_site(&SITE);
    let p = project(site.path(), |env| env.tools.node_modules = dirs);
    let src = p.asset("js/app.js");
    let id = p
        .store
        .transform(src, Transform::Babel(BabelOptions::default()))
        .unwrap();
    let code = String::from_utf8(p.store.content(id).unwrap().to_vec()).unwrap();
    assert!(code.contains("const f = () => 1;"), "{code}");
}
