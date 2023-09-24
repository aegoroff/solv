use std::{
    cell::RefCell,
    fmt::{self, Display},
};

use solp::Consume;

use crate::error::Collector;

pub struct Json {
    serialized: Vec<String>,
    errors: RefCell<Collector>,
}

impl Json {
    #[must_use]
    pub fn new() -> Self {
        Self {
            serialized: vec![],
            errors: RefCell::new(Collector::new()),
        }
    }
}

impl Default for Json {
    fn default() -> Self {
        Self::new()
    }
}

impl Consume for Json {
    fn ok(&mut self, _path: &str, solution: &solp::api::Solution) {
        if let Ok(s) = serde_json::to_string(solution) {
            self.serialized.push(s)
        }
    }

    fn err(&self, path: &str) {
        self.errors.borrow_mut().add_path(path);
    }
}

impl Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.serialized.len() > 1 {
            write!(f, "[")?;
        }
        for (ix, s) in self.serialized.iter().enumerate() {
            write!(f, "{s}")?;
            if ix < self.serialized.len() - 1 {
                write!(f, ",")?;
            }
        }
        if self.serialized.len() > 1 {
            write!(f, "]")?;
        }

        write!(f, "{}", self.errors.borrow())
    }
}
