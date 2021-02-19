#[macro_use] extern crate lalrpop_util;

lalrpop_mod!(pub solt);

#[derive(Debug)]
pub enum OpCode {
    Comma,
    Quote,
    Dot,
    Eq,
    ParenOpen,
    ParenClose,
}

#[derive(Debug)]
pub enum Expr {
    Comment(String),
    DigitOrDot(String),
    Guid(String),
    Identifier(String),
    Str(String),
    BareStr(String),
    Version(String, String),
    FirstLine,
    ProjectBegin,
    ProjectEnd,
    ProjectType(String),
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
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "DataVirtualization", "DataVirtualization\DataVirtualization.csproj", "{8102706C-AA37-4250-8889-1240FEB6F92F}"
EndProject
"#).unwrap();
        println!("{:#?}", result)
        //assert_eq!(result, "AB");
    }
}
