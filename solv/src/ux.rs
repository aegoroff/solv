use std::collections::BTreeSet;

use crossterm::style::{style, Color, Stylize};
use prettytable::{
    format::{self, TableFormat},
    Table,
};

#[must_use]
pub fn new_format() -> TableFormat {
    format::FormatBuilder::new()
        .column_separator(' ')
        .borders(' ')
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('-', ' ', ' ', ' '),
        )
        .indent(3)
        .padding(0, 0)
        .build()
}

pub fn print_one_column_table(head: &str, set: &BTreeSet<&str>) {
    if set.is_empty() {
        return;
    }
    let mut table = Table::new();

    let fmt = new_format();
    table.set_format(fmt);
    table.set_titles(row![bF=> head]);

    for item in set {
        table.add_row(row![*item]);
    }

    table.printstd();
    println!();
}

pub fn print_solution_path(path: &str) {
    let path = style(path)
        .with(Color::Rgb {
            r: 0xAA,
            g: 0xAA,
            b: 0xAA,
        })
        .bold();
    println!(" {path}");
}
