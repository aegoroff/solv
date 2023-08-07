use std::fmt::Display;

use crossterm::style::Stylize;

use crate::ux;

pub struct Collector {
    paths: Vec<String>,
}

impl Collector {
    #[must_use]
    pub fn new() -> Self {
        Self { paths: vec![] }
    }

    pub fn add_path(&mut self, path: &str) {
        self.paths.push(path.to_owned());
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.paths.len() as u64
    }
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for Collector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.paths.is_empty() {
            writeln!(
                f,
                "{}",
                " These solutions cannot be parsed:".dark_red().bold()
            )?;

            ux::print_one_column_table(
                "Path",
                None,
                self.paths.iter().map(std::string::String::as_str),
            );
        }
        Ok(())
    }
}
