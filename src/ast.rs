use crate::msbuild;

#[derive(Debug)]
pub enum Expr<'input> {
    Comment(&'input str),
    DigitOrDot(&'input str),
    Guid(&'input str),
    Identifier(&'input str),
    Str(&'input str),
    Version(Box<Expr<'input>>, Box<Expr<'input>>),
    FirstLine(Box<Expr<'input>>),
    Global(Vec<Expr<'input>>),
    Project(Box<Expr<'input>>, Vec<Expr<'input>>),
    ProjectBegin(
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
        Box<Expr<'input>>,
    ),
    Section(Box<Expr<'input>>, Vec<Expr<'input>>),
    SectionBegin(Vec<Expr<'input>>, Box<Expr<'input>>),
    SectionContent(Box<Expr<'input>>, Box<Expr<'input>>),
    SectionKey(Box<Expr<'input>>),
    SectionValue(Box<Expr<'input>>),
}

#[derive(Debug, Clone)]
pub struct Solution<'input> {
    pub format: &'input str,
    pub product: &'input str,
    pub projects: Vec<Project<'input>>,
    pub versions: Vec<Version<'input>>,
    pub configurations: Vec<Configuration<'input>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Version<'input> {
    pub name: &'input str,
    pub ver: &'input str,
}

#[derive(Debug, Copy, Clone)]
pub struct Configuration<'input> {
    pub configuration: &'input str,
    pub platform: &'input str,
}

#[derive(Debug, Copy, Clone)]
pub struct Project<'input> {
    pub type_id: &'input str,
    pub type_descr: &'input str,
    pub id: &'input str,
    pub name: &'input str,
    pub path: &'input str,
}

impl<'input> Solution<'input> {
    pub fn new() -> Self {
        Self {
            format: "",
            product: "",
            projects: Vec::new(),
            versions: Vec::new(),
            configurations: Vec::new(),
        }
    }
}

impl<'input> Project<'input> {
    pub fn new(id: &'input str, type_id: &'input str) -> Self {
        let type_descr;
        if let Some(type_name) = msbuild::PROJECT_TYPES.get(type_id) {
            type_descr = *type_name;
        } else {
            type_descr = type_id;
        }

        Self {
            id,
            type_id,
            type_descr,
            name: "",
            path: "",
        }
    }
}

impl<'input> Version<'input> {
    pub fn new(name: &'input str, ver: &'input str) -> Self {
        Self { name, ver }
    }
}

impl<'input> Configuration<'input> {
    pub fn new(s: &'input str) -> Self {
        let parts: Vec<&str> = s.split("|").collect();
        let mut configuration = "";
        let mut platform = "";
        if parts.len() == 2 {
            configuration = parts[0];
            platform = parts[1];
        }
        Self {
            configuration,
            platform,
        }
    }
}
