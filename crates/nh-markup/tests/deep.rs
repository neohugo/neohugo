//! Deeply nested documents (the `deep` part of the `convert` oracle) render like Go on a 2 MiB
//! stack (Go grows its goroutine stacks; the port must not recurse on the document depth).

mod common;

use common::*;

/// `deepDocs` of the Go oracle (tools/go-oracle/nh-markup/convert).
fn deep_docs() -> Vec<String> {
    vec![
        "> ".repeat(3000) + "deep *quote*\n",
        "- ".repeat(3000) + "item\n",
        "*".repeat(5000) + "x" + &"*".repeat(5000) + "\n",
        "## ".to_string() + &"**a ".repeat(2000) + &"**".repeat(2000) + "\n",
        "![".repeat(1000) + "x" + &"](i)".repeat(1000) + "\n",
        "[".repeat(3000) + "x" + &"](u)".repeat(3000) + "\n",
        "# ".to_string()
            + &"[a ".repeat(1500)
            + &"](u)".repeat(1500)
            + "\n\n"
            + &"> ".repeat(500)
            + "| a |\n"
            + &"> ".repeat(500)
            + "|---|\n",
    ]
}

#[test]
fn deep_nesting_on_a_small_stack() {
    let fx = read_fixture("convert/convert.json.gz");
    let deep = fx["deep"].as_array().unwrap().clone();
    let toml = fx["configs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "seeksnack")
        .unwrap()["toml"]
        .as_str()
        .unwrap()
        .to_string();
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let (p, _) = provider(&toml);
            for (i, (d, want)) in deep_docs().iter().zip(deep.iter()).enumerate() {
                let conv = converter(&p, document_context("deep"));
                match convert(&*conv, d.as_bytes(), i % 2 == 0, replica_renderers()) {
                    Outcome::Html(rr) => {
                        assert!(rr.bytes == bytes(&want["html"]), "deep doc {i} differs")
                    }
                    Outcome::Err(e) => panic!("deep doc {i}: error {e}"),
                    Outcome::Panic(p) => panic!("deep doc {i}: panic {p}"),
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
