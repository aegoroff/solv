#[macro_use] extern crate lalrpop_util;

lalrpop_mod!(pub solt);

#[derive(Debug)]
pub enum OpCode {
    Comma,
    Dot,
    Eq,
    ParenOpen,
    ParenClose,
    Comment(String),
    DigitOrDot(String),
    Guid(String),
    Identifier(String),
    Str(String),
    BareStr(String),
    Version(String, String),
    FirstLine,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

        let result = solt::SolutionParser::new().parse(r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio 2013
VisualStudioVersion = 12.0.31101.0
MinimumVisualStudioVersion = 10.0.40219.1
"#).unwrap();
        println!("{:#?}", result)
        //assert_eq!(result, "AB");
    }
}
