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
    Platform(String),
    Str(String),
    Path(String),
    BareString(String),
    BareStr(String),
    Version(Box<Expr>, Box<Expr>),
    FirstLine,
    ProjectBegin(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),
    ProjectEnd,
    ProjectType(Box<Expr>),
    ProjectSectionBegin(Box<Expr>, Box<Expr>),
    GlobalSectionBegin(Box<Expr>, Box<Expr>),
    ProjectSectionEnd,
    GlobalSectionEnd,
    ProjectSectionContent(Box<Expr>, Box<Expr>),
    GlobalSectionContent(Box<Expr>, Box<Expr>),
    Global,
    GlobalEnd,
    ConfigurationPlatform(Box<Expr>, Box<Expr>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

        let result = solt::SolutionParser::new().parse(r#"
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio 15
VisualStudioVersion = 15.0.26403.0
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
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = "solution items", "solution items", "{3B960F8F-AD5D-45E7-92C0-05B65E200AC4}"
	ProjectSection(SolutionItems) = preProject
		.editorconfig = .editorconfig
		appveyor.yml = appveyor.yml
		logviewer.xml = logviewer.xml
		WiX.msbuild = WiX.msbuild
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.tests", "logviewer.tests\logviewer.tests.csproj", "{939DD379-CDC8-47EF-8D37-0E5E71D99D30}"
	ProjectSection(ProjectDependencies) = postProject
		{383C08FC-9CAC-42E5-9B02-471561479A74} = {383C08FC-9CAC-42E5-9B02-471561479A74}
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.logic", "logviewer.logic\logviewer.logic.csproj", "{383C08FC-9CAC-42E5-9B02-471561479A74}"
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = ".nuget", ".nuget", "{B720ED85-58CF-4840-B1AE-55B0049212CC}"
	ProjectSection(SolutionItems) = preProject
		.nuget\NuGet.Config = .nuget\NuGet.Config
	EndProjectSection
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.engine", "logviewer.engine\logviewer.engine.csproj", "{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.install.mca", "logviewer.install.mca\logviewer.install.mca.csproj", "{405827CB-84E1-46F3-82C9-D889892645AC}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.ui", "logviewer.ui\logviewer.ui.csproj", "{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "logviewer.bench", "logviewer.bench\logviewer.bench.csproj", "{75E0C034-44C8-461B-A677-9A19566FE393}"
EndProject
"#).unwrap();
        println!("{:#?}", result)
        //assert_eq!(result, "AB");
    }
}
