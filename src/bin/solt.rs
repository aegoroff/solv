use std::env;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let path = &args[0];

    solt_rs::scan(path);
}
