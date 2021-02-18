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

        let result = solt::SolutionParser::new().parse("AB\nCD").unwrap();
        println!("{:#?}", result)
        //assert_eq!(result, "AB");
    }
}
