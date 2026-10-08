use std::fmt::Display;

use crossterm::style::Stylize;

use crate::ux;

#[derive(Default)]
pub struct Collector {
    paths: Vec<String>,
}

impl Collector {
    pub fn add_path(&mut self, path: &str) {
        self.paths.push(path.to_owned());
    }

    #[must_use]
    pub fn count(&self) -> u64 {
        self.paths.len() as u64
    }
}

impl Display for Collector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.paths.is_empty() {
            writeln!(
                f,
                " {}",
                "These solutions cannot be parsed:".dark_red().bold()
            )?;

            ux::write_one_column_table(
                f,
                "Path",
                None,
                self.paths.iter().map(std::string::String::as_str),
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_contains_header_and_paths() {
        // Arrange
        let mut collector = Collector::default();
        collector.add_path("/a/bad.sln");

        // Act
        let actual = collector.to_string();

        // Assert
        let header = actual.find("These solutions cannot be parsed:").unwrap();
        let path = actual.find("/a/bad.sln").unwrap();
        assert!(header < path);
    }
}
