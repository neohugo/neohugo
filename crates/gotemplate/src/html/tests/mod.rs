//! Tests of the html/template leaf functions: ports of Go's own tests
//! (`go_tests`) and replays of the Go oracle fixtures (`oracle`,
//! `tests/fixtures/html/*.txt.gz`, written by
//! `tools/go-oracle/gotemplate/escfuncs.go`).
//!
//! These are unit tests because the leaf functions are crate-private; the
//! shared value-spec parser lives with the integration tests.

#[path = "../../../tests/common/mod.rs"]
pub(crate) mod common;

mod go_tests;
mod oracle;
