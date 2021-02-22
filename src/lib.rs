use std::path::Path;
use std::time::Instant;

use jwalk::WalkDir;
use prettytable::format;
use prettytable::Table;
use std::collections::BTreeMap;
use std::option::Option::Some;

mod ast;
mod lex;
mod msbuild;
pub mod parser;

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
            if e.file_type().is_file() {
                let file_name = e.file_name.to_str().unwrap();
                if let Some(ext) = get_extension_from_filename(file_name) {
                    if ext == "sln" {
                        let full_path = e.path();
                        let full_path = full_path.to_str().unwrap();
                        if let Some((format, projects)) = parser::parse(full_path, print_ast) {
                            print(full_path, (format, projects));
                        }
                    }
                }
            }
        }
    }
    println!(
        "elapsed: {}",
        humantime::format_duration(now.elapsed()).to_string()
    );
}

pub fn print(path: &str, solution: (String, BTreeMap<String, i32>)) {
    let (ver, projects_by_type) = solution;
    println!(" {}", path);
    println!("  Format: {}", ver);
    println!();
    println!("  Projects:");

    let mut table = Table::new();

    let format = format::FormatBuilder::new()
        .column_separator(' ')
        .borders(' ')
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('-', ' ', ' ', ' '),
        )
        .indent(3)
        .padding(0, 0)
        .build();
    table.set_format(format);
    table.set_titles(row![bF=> "Project type", "Count"]);

    for (key, value) in projects_by_type.iter() {
        table.add_row(row![*key, bFg->*value]);
    }

    table.printstd();
    println!();
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension()?.to_str()
}
