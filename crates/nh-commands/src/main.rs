//! `neohugo` binary entry point. Go: main.go:main

fn main() {
    std::process::exit(nh_commands::commandeer::execute(std::env::args().skip(1).collect()));
}
