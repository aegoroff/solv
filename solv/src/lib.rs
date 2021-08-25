mod info;
mod validate;

use crate::info::Info;
use crate::validate::Validate;
use solp::Consume;

use ansi_term::Colour::Red;

extern crate humantime;
extern crate solp;
#[macro_use]
extern crate prettytable;
extern crate ansi_term;

pub trait ConsumeDisplay: Consume + std::fmt::Display {
    fn as_consume(&mut self) -> &mut dyn Consume;
}

// Trait casting code begin

impl ConsumeDisplay for Info {
    fn as_consume(&mut self) -> &mut dyn Consume {
        self
    }
}

impl ConsumeDisplay for Validate {
    fn as_consume(&mut self) -> &mut dyn Consume {
        self
    }
}

// Trait casting code end

// Factory method
pub fn new_consumer(
    debug: bool,
    only_validate: bool,
    only_problems: bool,
) -> Box<dyn ConsumeDisplay> {
    if only_validate {
        Validate::new_box(debug, only_problems)
    } else {
        Info::new_box(debug)
    }
}

fn err(debug: bool, path: &str) {
    if debug {
        return;
    }
    let path = Red.paint(path);
    eprintln!("Error parsing {} solution", path);
}
