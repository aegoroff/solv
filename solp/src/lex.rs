use std::{fmt::Display, str::CharIndices};

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

#[derive(Copy, Clone, Debug)]
pub enum LexicalError {
    // Not possible
}

#[derive(Copy, Clone, Debug)]
pub enum Tok<'input> {
    Comment(&'input str),
    Str(&'input str),
    SectionKey(&'input str),
    SectionValue(&'input str),
    Guid(&'input str),
    Id(&'input str),
    DigitsAndDots(&'input str),
    OpenElement(&'input str),
    CloseElement(&'input str),
    Comma,
    Eq,
    Skip,
}

impl<'input> Display for Tok<'input> {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Display for LexicalError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

enum LexerContext {
    None,
    SectionDefinition,
    InsideSection,
    InsideString,
}

pub struct Lexer<'input> {
    chars: std::iter::Peekable<CharIndices<'input>>,
    input: &'input str,
    context: LexerContext,
}

const SECTION_SUFFIX: &str = "Section";

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Lexer {
            chars: input.char_indices().peekable(),
            input,
            context: LexerContext::None,
        }
    }

    fn identifier(&mut self, i: usize) -> (Tok<'input>, usize) {
        let finish;
        loop {
            if let Some((j, c)) = self.chars.peek() {
                match *c {
                    'a'..='z' | 'A'..='Z' => {
                        self.chars.next();
                    }
                    '(' => {
                        finish = *j;
                        break;
                    }
                    _ => {
                        if Lexer::is_close_element(&self.input[i..]) {
                            self.context = LexerContext::None;
                            return (Tok::CloseElement(&self.input[i..*j]), *j);
                        }
                        return (Tok::Id(&self.input[i..*j]), *j);
                    }
                }
            } else {
                if Lexer::is_close_element(&self.input[i..]) {
                    return (Tok::CloseElement(&self.input[i..]), self.input.len());
                }
                return (Tok::Id(&self.input[i..]), self.input.len());
            }
        }
        // Skip (
        self.chars.next();

        let sub = &self.input[i..finish];
        if sub.len() > SECTION_SUFFIX.len() {
            let start = sub.len() - SECTION_SUFFIX.len();
            if &sub[start..] == SECTION_SUFFIX {
                self.context = LexerContext::SectionDefinition;
            };
        }
        (Tok::OpenElement(&self.input[i..finish]), finish)
    }

    fn comment(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        loop {
            match self.chars.peek() {
                Some((j, '\n' | '\r')) => {
                    return Some(Ok((i, Tok::Comment(&self.input[i..*j]), *j)));
                }
                None => {
                    return Some(Ok((i, Tok::Comment(&self.input[i..]), self.input.len())));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn guid(&mut self, i: usize) -> (usize, Tok<'input>, usize) {
        let finish;
        loop {
            match self.chars.peek() {
                Some((j, '}')) => {
                    finish = *j + 1;
                    break;
                }
                None => {
                    return (i, Tok::Guid(&self.input[i..]), self.input.len());
                }
                _ => {
                    self.chars.next();
                }
            }
        }
        // Skip {
        self.chars.next();
        (i, Tok::Guid(&self.input[i..finish]), finish)
    }

    fn digits_with_dots(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        loop {
            match self.chars.peek() {
                Some((j, c)) => match *c {
                    '0'..='9' | '.' => {
                        self.chars.next();
                    }
                    _ => return Some(Ok((i, Tok::DigitsAndDots(&self.input[i..*j]), *j))),
                },
                None => {
                    return Some(Ok((
                        i,
                        Tok::DigitsAndDots(&self.input[i..]),
                        self.input.len(),
                    )));
                }
            }
        }
    }

    fn string(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        match self.context {
            LexerContext::InsideString => {
                self.context = LexerContext::None;
                return Some(Ok((i, Tok::Skip, i + 1)));
            }
            _ => {
                self.context = LexerContext::InsideString;
            }
        }

        loop {
            match self.chars.peek() {
                // Guid start
                Some((_, '{')) => {
                    return Some(Ok(self.guid(i + 1)));
                }
                Some((j, '"')) => {
                    let start = i + 1;
                    let val = &self.input[start..*j];
                    return Some(Ok((start, Tok::Str(val), *j)));
                }
                None => return Some(Ok((i, Tok::Str(&self.input[i..]), self.input.len()))),
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn section_key(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        let mut start = i;

        while let Some((j, '\r' | '\n' | '\t' | ' ')) = self.chars.peek() {
            start = *j + 1;
            self.chars.next();
        }

        if Lexer::is_close_element(&self.input[start..]) {
            self.context = LexerContext::None;
            return Some(Ok((start, Tok::Skip, start)));
        }

        match self.context {
            LexerContext::InsideSection => {}
            LexerContext::SectionDefinition => self.context = LexerContext::InsideSection,
            _ => return Some(Ok((start, Tok::Skip, start))),
        }

        loop {
            match self.chars.peek() {
                Some((j, '=')) => {
                    let finish = Lexer::trim_end(self.input, *j);
                    let val = if finish < start {
                        ""
                    } else {
                        &self.input[start..finish]
                    };
                    return Some(Ok((start, Tok::SectionKey(val), finish)));
                }
                None => {
                    let val = &self.input[start..];

                    if Lexer::is_close_element(val) {
                        return Some(Ok((start, Tok::CloseElement(val), self.input.len())));
                    }

                    return Some(Ok((start, Tok::SectionKey(val), self.input.len())));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn section_value(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        match self.context {
            LexerContext::InsideSection => {
                // i + 1 skips equal sign (=)
                let start = Lexer::trim_start(self.input, i + 1);

                loop {
                    match self.chars.peek() {
                        Some((j, '\r' | '\n')) => {
                            let finish = *j;
                            return Some(Ok((
                                start,
                                Tok::SectionValue(&self.input[start..finish]),
                                finish,
                            )));
                        }
                        None => {
                            return Some(Ok((
                                start,
                                Tok::SectionValue(&self.input[start..]),
                                self.input.len(),
                            )));
                        }
                        _ => {
                            self.chars.next();
                        }
                    }
                }
            }
            _ => Some(Ok((i, Tok::Eq, i + 1))),
        }
    }

    fn is_close_element(val: &str) -> bool {
        // This implementation is ugly equivalent of:
        // let substr: String = val.chars().take("End".len()).collect();
        // substr == "End"
        // but without allocations
        let mut it = val.chars();

        let mut next_is = |c: char| -> bool { it.next().map(|x| x == c).unwrap_or_default() };

        next_is('E') && next_is('n') && next_is('d')
    }

    fn trim_start(s: &str, mut i: usize) -> usize {
        i += Lexer::count_whitespaces(s[i..].chars());
        i
    }

    fn trim_end(s: &str, mut i: usize) -> usize {
        i -= Lexer::count_whitespaces(s[..i].chars().rev());
        i
    }

    fn count_whitespaces<I: Iterator<Item = char>>(it: I) -> usize {
        it.take_while(|c| ' ' == *c || '\t' == *c).count()
    }

    fn current(&mut self, i: usize, ch: char) -> Option<Spanned<Tok<'input>, usize, LexicalError>> {
        match ch {
            '\r' | '\n' => return self.section_key(i),
            '=' => return self.section_value(i),
            ',' => return Some(Ok((i, Tok::Comma, i + 1))),
            ')' | ' ' | '\t' => return Some(Ok((i, Tok::Skip, i + 1))),
            '0'..='9' => return self.digits_with_dots(i),
            '{' => return Some(Ok(self.guid(i))),
            '"' => return self.string(i),
            '#' => return self.comment(i),
            'a'..='z' | 'A'..='Z' => {
                let (tok, loc) = self.identifier(i);
                return Some(Ok((i, tok, loc)));
            }
            _ => {}
        }
        None
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Tok<'input>, usize, LexicalError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, c) = self.chars.next()?;
            let tok = self.current(i, c)?;

            if let Ok(_x @ (_, Tok::Skip, _)) = tok {
                continue;
            }

            return Some(tok);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn lexer() {
        let input = r#"# comment
         12.00
         ({405827CB-84E1-46F3-82C9-D889892645AC}) . |
         "{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}" "str" x = y  "#;
        let lexer = Lexer::new(input);
        for tok in lexer {
            println!("{tok:#?}");
        }
    }

    #[test]
    fn lex_solution() {
        let lexer = Lexer::new(REAL_SOLUTION);
        for tok in lexer {
            println!("{tok:#?}");
        }
    }

    #[test]
    fn lex_solution_no_last_line_break() {
        let lexer = Lexer::new(REAL_SOLUTION.trim_end());
        for tok in lexer {
            println!("{tok:#?}");
        }
    }

    #[rstest]
    #[case("1 ", 1)]
    #[case("1", 1)]
    #[case("  1", 3)]
    #[case(" ", 0)]
    #[case("  ", 0)]
    #[case(" \t", 0)]
    #[case("", 0)]
    #[case("          ", 0)]
    #[trace]
    fn trim_end_tests(#[case] content: &str, #[case] expected: usize) {
        // Arrange
        let i: usize = content.len();

        // Act
        let actual = Lexer::trim_end(content, i);

        // Assert
        assert_eq!(actual, expected);
    }

    const REAL_SOLUTION: &str = r#"
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
Global
	GlobalSection(SolutionConfigurationPlatforms) = preSolution
		Debug|Any CPU = Debug|Any CPU
		Debug|Mixed Platforms = Debug|Mixed Platforms
		Debug|x86 = Debug|x86
		Release|Any CPU = Release|Any CPU
		Release|Mixed Platforms = Release|Mixed Platforms
		Release|x86 = Release|x86
	EndGlobalSection
	GlobalSection(ProjectConfigurationPlatforms) = postSolution
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Any CPU.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.ActiveCfg = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Debug|x86.Build.0 = Debug|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Any CPU.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|Mixed Platforms.Build.0 = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.ActiveCfg = Release|x86
		{27060CA7-FB29-42BC-BA66-7FC80D498354}.Release|x86.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Any CPU.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|Mixed Platforms.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.ActiveCfg = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Debug|x86.Build.0 = Debug|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Any CPU.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|Mixed Platforms.Build.0 = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.ActiveCfg = Release|x86
		{1C0ED62B-D506-4E72-BBC2-A50D3926466E}.Release|x86.Build.0 = Release|x86
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Debug|x86.ActiveCfg = Debug|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Any CPU.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{939DD379-CDC8-47EF-8D37-0E5E71D99D30}.Release|x86.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Debug|x86.ActiveCfg = Debug|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Any CPU.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{383C08FC-9CAC-42E5-9B02-471561479A74}.Release|x86.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Debug|x86.ActiveCfg = Debug|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Any CPU.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{90E3A68D-C96D-4764-A1D0-F73D9F474BE4}.Release|x86.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Debug|x86.ActiveCfg = Debug|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Any CPU.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{405827CB-84E1-46F3-82C9-D889892645AC}.Release|x86.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Debug|x86.ActiveCfg = Debug|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Any CPU.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{CFBAE2FB-6E3F-44CF-9FC9-372D6EA8DD3D}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Any CPU.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|Mixed Platforms.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.ActiveCfg = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Debug|x86.Build.0 = Debug|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Any CPU.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|Mixed Platforms.Build.0 = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.ActiveCfg = Release|Any CPU
		{75E0C034-44C8-461B-A677-9A19566FE393}.Release|x86.Build.0 = Release|Any CPU
	EndGlobalSection
	GlobalSection(SolutionProperties) = preSolution
		HideSolutionNode = FALSE
	EndGlobalSection
EndGlobal
"#;
}
