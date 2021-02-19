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
    ProjectBegin(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
    ProjectEnd,
    ProjectType(String),
    ProjectSectionBegin(Box<Expr>, Box<Expr>),
    ProjectSectionEnd,
    ProjectSectionContent(Box<Expr>, Box<Expr>),
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
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install", "logviewer.install\logviewer.install.wixproj", "{27060CA7-FB29-42BC-BA66-7FC80D498354}"
	ProjectSection(ProjectDependencies) = postProject
		{405827CB-84E1-46F3-82C9-D889892645AC} = {405827CB-84E1-46F3-82C9-D889892645AC}
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D} = {CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}
	EndProjectSection
EndProject
Project("{930C7802-8A8C-48F9-8165-68863BCCD9DD}") = "logviewer.install.bootstrap", "logviewer.install.bootstrap\logviewer.install.bootstrap.wixproj", "{1C0ED62B-D506-4E72-BBC2-A50D3926466E}"
	ProjectSection(ProjectDependencies) = postProject
		{27060CA7-FB29-42BC-BA66-7FC80D498354} = {27060CA7-FB29-42BC-BA66-7FC80D498354}
	EndProjectSection
EndProject
"#).unwrap();
        println!("{:#?}", result)
        //assert_eq!(result, "AB");
    }
}
