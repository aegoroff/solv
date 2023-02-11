mod info;
mod validate;

use crate::info::Info;
use crate::validate::Validate;
use crossterm::style::Stylize;
use solp::Consume;

extern crate humantime;
extern crate solp;
#[macro_use]
extern crate prettytable;
extern crate crossterm;

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
#[must_use]
pub fn new_consumer(only_validate: bool, only_problems: bool) -> Box<dyn ConsumeDisplay> {
    if only_validate {
        Validate::new_box(only_problems)
    } else {
        Info::new_box()
    }
}

fn err(path: &str) {
    eprintln!("Error parsing {} solution", path.red());
}
