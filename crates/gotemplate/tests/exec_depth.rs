//! Go: exec_test.go:TestMaxExecDepth — a template invoking itself stops
//! with "exceeded maximum template depth" instead of overflowing the stack.
//!
//! Go's goroutine stacks grow on demand; a Rust thread's stack is fixed.
//! Reaching Go's `maxExecDepth` (100000 nested `{{template}}` calls) needs
//! roughly 1 KiB of stack per level (between 64 and 128 MiB in the test
//! profile), so hosts must run executions on threads with enough stack.

use go_value::Value;
use gotemplate::text::Template;

fn run_depth_test() -> String {
    let t = Template::new("tmpl");
    t.parse(r#"{{template "tmpl" .}}"#).unwrap();
    let mut out = Vec::new();
    match t.execute(&mut out, &Value::Invalid) {
        Ok(()) => "<nil>".to_string(),
        Err(e) => e.to_string(),
    }
}

#[test]
fn test_max_exec_depth() {
    let got = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(run_depth_test)
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(
        got,
        "template: tmpl:1:11: executing \"tmpl\" at <{{template \"tmpl\" .}}>: exceeded maximum template depth (100000)"
    );
}
