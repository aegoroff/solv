use crate::ast::Solution;
use crate::msbuild;
use crate::parser::Consume;
use prettytable::format;
use prettytable::Table;
use std::collections::BTreeMap;

pub struct Print {
    path: String,
}

impl Print {
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
        }
    }
}

impl Consume for Print {
    fn consume(&self, solution: &Solution) {
        let mut projects_by_type: BTreeMap<&str, i32> = BTreeMap::new();
        for prj in &solution.projects {
            if prj.type_id == msbuild::ID_SOLUTION_FOLDER {
                continue;
            }
            *projects_by_type.entry(prj.type_descr).or_insert(0) += 1;
        }

        println!(" {}", self.path);
        println!("  Format: {}", solution.format);
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
}
