#[derive(Debug)]
pub enum Expr<'input> {
    Comment(&'input str),
    DigitOrDot(&'input str),
    Guid(&'input str),
    Identifier(&'input str),
    Platform(&'input str),
    Str(&'input str),
    Path(&'input str),
    BareString(&'input str),
    BareStr(&'input str),
    Version(Box<Expr<'input>>, Box<Expr<'input>>),
    FirstLine,
    ProjectBegin(Box<Expr<'input>>, Box<Expr<'input>>, Box<Expr<'input>>, Box<Expr<'input>>),
    ProjectEnd,
    ProjectType(Box<Expr<'input>>),
    ProjectSectionBegin(Box<Expr<'input>>, Box<Expr<'input>>),
    GlobalSectionBegin(Box<Expr<'input>>, Box<Expr<'input>>),
    ProjectSectionEnd,
    GlobalSectionEnd,
    ProjectSectionContent(Box<Expr<'input>>, Box<Expr<'input>>),
    GlobalSectionContent(Box<Expr<'input>>, Box<Expr<'input>>),
    Global,
    GlobalEnd,
    ConfigurationPlatform(Box<Expr<'input>>, Box<Expr<'input>>),
}