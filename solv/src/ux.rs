use comfy_table::{
    Attribute, Cell, ContentArrangement, ContentLineStyle, LineStyle, Row, Table, TableStyle,
};
use crossterm::style::{Color, Stylize, style};
use std::fmt;

const TABLE_STYLE: TableStyle = TableStyle::new()
    .header_lines(ContentLineStyle::new(' ', ' ', ' '))
    .header_separator(LineStyle::new(' ', '-', ' ', ' '))
    .content_lines(ContentLineStyle::new(' ', ' ', ' '))
    .bottom_border(LineStyle::new(' ', ' ', ' ', ' '));

#[must_use]
pub fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_style(TABLE_STYLE)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Writes one column table if there are rows
pub fn write_one_column_table<I: Iterator<Item = S>, S: ToString>(
    f: &mut fmt::Formatter<'_>,
    head: &str,
    head_color: Option<comfy_table::Color>,
    rows: I,
) -> fmt::Result {
    if let Some(t) = create_one_column_table(head, head_color, rows) {
        writeln!(f, "{t}")?;
    }
    Ok(())
}

pub fn create_one_column_table<I: Iterator<Item = S>, S: ToString>(
    head: &str,
    head_color: Option<comfy_table::Color>,
    rows: I,
) -> Option<Table> {
    let mut table = new_table();
    let mut head = Cell::new(head).add_attribute(Attribute::Bold);
    if let Some(fg) = head_color {
        head = head.fg(fg);
    }
    table.set_header([head]);
    table.add_rows(rows.map(|s| Row::from([s])));

    if table.is_empty() { None } else { Some(table) }
}

#[must_use]
pub fn create_solution_table(path: &str) -> Table {
    let mut table = new_table();
    table.set_header([Cell::new(path).add_attribute(Attribute::Bold).fg(
        comfy_table::Color::Rgb {
            r: 0xAA,
            g: 0xAA,
            b: 0xAA,
        },
    )]);
    table.style_mut().header_separator = LineStyle::new(' ', ' ', ' ', ' ');
    table
}

/// Writes solution path line
pub fn write_solution_path(f: &mut fmt::Formatter<'_>, path: &str) -> fmt::Result {
    let path = style(path)
        .with(Color::Rgb {
            r: 0xAA,
            g: 0xAA,
            b: 0xAA,
        })
        .bold();
    writeln!(f, " {path}")
}
