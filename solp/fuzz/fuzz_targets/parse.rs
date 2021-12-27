#![no_main]
use libfuzzer_sys::fuzz_target;
use solp::ast::Solution;
use solp::Consume;

fuzz_target!(|data: &str| {
    let mut c = Consumer {};
    solp::parse(&mut c, data);
});

struct Consumer {}

impl Consume for Consumer {
    fn ok(&mut self, _path: &str, _solution: &Solution) {}

    fn err(&self, _path: &str) {}

    fn is_debug(&self) -> bool {
        return false;
    }
}
