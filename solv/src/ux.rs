use comfy_table::{Attribute, Cell, ContentArrangement, Row, Table, TableComponent, presets};
use crossterm::style::{Color, Stylize, style};

#[must_use]
pub fn new_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL_CONDENSED)
        .set_style(TableComponent::BottomBorder, ' ')
        .set_style(TableComponent::BottomBorderIntersections, ' ')
        .set_style(TableComponent::TopBorder, ' ')
        .set_style(TableComponent::TopBorderIntersections, ' ')
        .set_style(TableComponent::HeaderLines, '-')
        .set_style(TableComponent::RightHeaderIntersection, ' ')
        .set_style(TableComponent::LeftHeaderIntersection, ' ')
        .set_style(TableComponent::MiddleHeaderIntersections, ' ')
        .set_style(TableComponent::LeftBorder, ' ')
        .set_style(TableComponent::RightBorder, ' ')
        .set_style(TableComponent::TopRightCorner, ' ')
        .set_style(TableComponent::TopLeftCorner, ' ')
        .set_style(TableComponent::BottomLeftCorner, ' ')
        .set_style(TableComponent::BottomRightCorner, ' ')
        .set_style(TableComponent::VerticalLines, ' ')
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub fn print_one_column_table<I: ExactSizeIterator<Item = S>, S: ToString>(
    head: &str,
    head_color: Option<comfy_table::Color>,
    rows: I,
) {
    if let Some(t) = create_one_column_table(head, head_color, rows) {
        println!("{t}");
    }
}

pub fn create_one_column_table<I: ExactSizeIterator<Item = S>, S: ToString>(
    head: &str,
    head_color: Option<comfy_table::Color>,
    rows: I,
) -> Option<Table> {
    if rows.len() == 0 {
        None
    } else {
        let mut table = new_table();
        let mut head = Cell::new(head).add_attribute(Attribute::Bold);
        if let Some(fg) = head_color {
            head = head.fg(fg);
        }
        table.set_header([head]);
        table.add_rows(rows.into_iter().map(|s| Row::from([s])));

        Some(table)
    }
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
    table.set_style(TableComponent::HeaderLines, ' ');
    table
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
