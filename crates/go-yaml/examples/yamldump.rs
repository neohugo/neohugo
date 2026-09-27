//! Debug helper: decode stdin (or files) and print the canonical dumps.
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    if args.is_empty() {
        let mut b = Vec::new();
        std::io::stdin().read_to_end(&mut b).unwrap();
        inputs.push(b);
    } else {
        for a in &args {
            inputs.push(std::fs::read(a).unwrap());
        }
    }
    for data in inputs {
        match go_yaml::unmarshal(&data) {
            Ok(v) => println!("iface: {}", go_yaml::dump(&v)),
            Err(e) => println!("iface.err: {}", go_yaml::dump_escape(e.message_bytes())),
        }
        match go_yaml::metadecoders::unmarshal(&data) {
            Ok(v) => println!("meta: {}", go_yaml::metadecoders::dump_value(&v)),
            Err(e) => println!("meta.err: {}", go_yaml::dump_escape(&e.message_bytes())),
        }
        match go_yaml::unmarshal_str_map(&data) {
            Ok(v) => println!("smap: {}", go_yaml::dump_str_map(&v)),
            Err(e) => println!("smap.err: {}", go_yaml::dump_escape(e.message_bytes())),
        }
    }
}
