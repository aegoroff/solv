#[macro_use] extern crate lalrpop_util;

lalrpop_mod!(pub solt);

pub enum OpCode {
    Comma,
    Dot,
    Eq,
    ParenOpen,
    ParenClose,
    Comment(String),
    DigitOrDot(String),
    Crlf,
    Guid(String),
    Identifier(String),
    Str(String),
    BareStr(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

        let result = solt::SolutionParser::new().parse("AB\nCD").unwrap();
        assert_eq!(result, "AB");
    }
}
