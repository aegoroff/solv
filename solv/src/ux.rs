use comfy_table::{presets, Attribute, Cell, ContentArrangement, Row, Table, TableComponent};
use crossterm::style::{style, Color, Stylize};

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
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_width(120);
    table
}

pub fn print_one_column_table<'a, I: ExactSizeIterator<Item = &'a str>>(head: &str, rows: I) {
    if rows.len() == 0 {
        return;
    }
    let mut table = new_table();
    table.set_header(vec![Cell::new(head).add_attribute(Attribute::Bold)]);
    table.add_rows(rows.into_iter().map(|s| Row::from(vec![s])));

    println!("{table}");
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
