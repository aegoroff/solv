use std::path::Path;
use std::time::Instant;

use crate::print::Print;
use jwalk::WalkDir;
use std::option::Option::Some;

mod ast;
mod lex;
mod msbuild;
pub mod parser;
pub mod print;

#[macro_use]
extern crate lalrpop_util;
extern crate humantime;
extern crate jwalk;
#[macro_use]
extern crate prettytable;

lalrpop_mod!(
    #[allow(clippy::all)]
    #[allow(unused)]
    pub solt
);

pub fn scan(path: &str, print_ast: bool) {
    let now = Instant::now();

    for entry in WalkDir::new(path).skip_hidden(false).follow_links(false) {
        if let Ok(e) = entry {
            if !e.file_type().is_file() {
                continue;
            }
            let file_name = e.file_name.to_str().unwrap();
            if let Some(ext) = get_extension_from_filename(file_name) {
                if ext == "sln" {
                    let full_path = e.path();
                    let full_path = full_path.to_str().unwrap();
                    let prn = Print::new(full_path);
                    parser::parse(full_path, prn, print_ast);
                }
            }
        }
    }
    println!(
        "elapsed: {}",
        humantime::format_duration(now.elapsed()).to_string()
    );
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
}
