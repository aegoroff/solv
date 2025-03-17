use std::{fmt::Display, str::CharIndices};

pub type Spanned<Tok, Loc, Error> = miette::Result<(Loc, Tok, Loc), Error>;

#[derive(Copy, Clone, Debug)]
pub enum LexicalError {
    /// Occurs when end of stream is reached when a next token is expected or no correct token end found
    PrematureEndOfStream(usize),
}

#[derive(Copy, Clone, Debug)]
pub enum Tok<'a> {
    Comment(&'a str),
    Str(&'a str),
    SectionKey(&'a str),
    SectionValue(&'a str),
    Guid(&'a str),
    Id(&'a str),
    DigitsAndDots(&'a str),
    OpenElement(&'a str),
    CloseElement(&'a str),
    Comma,
    Eq,
    Skip,
}

impl Display for Tok<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tok::Comment(c) => write!(f, "Comment({c})")?,
            Tok::Str(s) => write!(f, "String({s})")?,
            Tok::SectionKey(k) => write!(f, "SectionKey({k})")?,
            Tok::SectionValue(v) => write!(f, "SectionValue({v})")?,
            Tok::Guid(g) => write!(f, "Guild({g})")?,
            Tok::Id(id) => write!(f, "Identifier({id})")?,
            Tok::DigitsAndDots(d) => write!(f, "DigitsAndDots({d})")?,
            Tok::OpenElement(elt) => write!(f, "OpenElement({elt})")?,
            Tok::CloseElement(elt) => write!(f, "CloseElement({elt})")?,
            Tok::Comma => write!(f, "Comma")?,
            Tok::Eq => write!(f, "Eq")?,
            Tok::Skip => write!(f, "Skip")?,
        }
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

/// A lexer for parsing a configuration file.
///
/// This lexer is designed to be used in conjunction with the `ast` module, which will perform the actual
/// parsing of the configuration file. The purpose of this lexer is to provide a stream of tokens that can
/// be processed by the `ast` module.
pub struct Lexer<'a> {
    chars: std::iter::Peekable<CharIndices<'a>>,
    input: &'a str,
    context: LexerContext,
}

const SECTION_SUFFIX: &str = "Section";

impl<'a> Lexer<'a> {
    /// Create a new lexer for parsing the given configuration file.
    pub fn new(input: &'a str) -> Self {
        Lexer {
            chars: input.char_indices().peekable(),
            input,
            context: LexerContext::None,
        }
    }

    #[inline]
    fn id_or_close_element(
        &mut self,
        collected: &'a str,
        start: usize,
        end: usize,
    ) -> (usize, Tok<'a>, usize) {
        if Lexer::is_close_element(collected) {
            self.context = LexerContext::None;
            (start, Tok::CloseElement(collected), end)
        } else {
            (start, Tok::Id(collected), end)
        }
    }

    fn identifier(&mut self, i: usize) -> (usize, Tok<'a>, usize) {
        while let Some((j, c)) = self.chars.peek() {
            let finish = *j;
            match *c {
                'a'..='z' | 'A'..='Z' => {
                    self.chars.next();
                }
                '(' => {
                    // Skip '('
                    self.chars.next();

                    let collected = &self.input[i..finish];
                    // Check if identifier is suffixed with 'Section' and update context if so
                    if collected.ends_with(SECTION_SUFFIX) {
                        self.context = LexerContext::SectionDefinition;
                    }
                    return (i, Tok::OpenElement(collected), finish);
                }
                _ => return self.id_or_close_element(&self.input[i..finish], i, finish),
            }
        }
        self.id_or_close_element(&self.input[i..], i, self.input.len())
    }

    fn comment(&mut self, i: usize) -> (usize, Tok<'a>, usize) {
        while let Some((j, c)) = self.chars.peek() {
            match *c {
                '\n' | '\r' => {
                    return (i, Tok::Comment(&self.input[i..*j]), *j);
                }
                _ => {
                    self.chars.next();
                }
            }
        }
        // If comment last file line
        (i, Tok::Comment(&self.input[i..]), self.input.len())
    }

    /// UUID parsing only inside string, i.e. chars between double quotes.
    /// Guids in section keys parsed on Ast visiting stage using nom crate. See ast module for details
    fn guid(&mut self, i: usize) -> Spanned<Tok<'a>, usize, LexicalError> {
        while let Some((j, c)) = self.chars.peek() {
            match *c {
                '}' => {
                    // include '}' char so increment j and advance chars
                    let finish = *j + 1;
                    self.chars.next();
                    return Ok((i, Tok::Guid(&self.input[i..finish]), finish));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
        Err(LexicalError::PrematureEndOfStream(i))
    }

    fn digits_with_dots(&mut self, i: usize) -> Spanned<Tok<'a>, usize, LexicalError> {
        while let Some((j, c)) = self.chars.peek() {
            match *c {
                '0'..='9' | '.' => {
                    self.chars.next();
                }
                _ => return Ok((i, Tok::DigitsAndDots(&self.input[i..*j]), *j)),
            }
        }
        Err(LexicalError::PrematureEndOfStream(i))
    }

    fn string(&mut self, i: usize) -> Spanned<Tok<'a>, usize, LexicalError> {
        match self.context {
            LexerContext::InsideString => {
                self.context = LexerContext::None;
                return Ok((i, Tok::Skip, i + 1));
            }
            _ => {
                self.context = LexerContext::InsideString;
            }
        }

        while let Some((j, c)) = self.chars.peek() {
            match *c {
                '"' => {
                    let start = i + 1;
                    let val = &self.input[start..*j];
                    return Ok((start, Tok::Str(val), *j));
                }
                '{' => {
                    // Guid start
                    let start = *j;
                    return self.guid(start);
                }
                _ => {
                    self.chars.next();
                }
            }
        }
        Err(LexicalError::PrematureEndOfStream(i))
    }

    /// REMARK: Guid inside section key will be parsed by nom crate on Ast visiting stage
    fn section_key(&mut self, i: usize) -> Spanned<Tok<'a>, usize, LexicalError> {
        let mut start = i;

        // skip whitespaces
        while let Some((j, '\r' | '\n' | '\t' | ' ')) = self.chars.peek() {
            start = *j + 1;
            self.chars.next();
        }

        // If close element just return Skip token
        if Lexer::is_close_element(&self.input[start..]) {
            self.context = LexerContext::None;
            return Ok((start, Tok::Skip, start));
        }

        // If section definition context just return Skip token
        match self.context {
            LexerContext::InsideSection => {}
            LexerContext::SectionDefinition => self.context = LexerContext::InsideSection,
            _ => return Ok((start, Tok::Skip, start)),
        }

        while let Some((j, c)) = self.chars.peek() {
            match *c {
                '=' => {
                    // trim space before '=' char
                    let finish = Lexer::trim_end(self.input, *j);
                    let val = if finish < start {
                        ""
                    } else {
                        &self.input[start..finish]
                    };
                    return Ok((start, Tok::SectionKey(val), finish));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
        Err(LexicalError::PrematureEndOfStream(i))
    }

    fn section_value(&mut self, i: usize) -> Spanned<Tok<'a>, usize, LexicalError> {
        match self.context {
            LexerContext::InsideSection => {
                // i + 1 skips '=' char
                let start = Lexer::trim_start(self.input, i + 1);

                // advance iterator until end of line and return section value if line end
                while let Some((j, c)) = self.chars.peek() {
                    match *c {
                        '\r' | '\n' => {
                            let finish = *j;
                            return Ok((
                                start,
                                Tok::SectionValue(&self.input[start..finish]),
                                finish,
                            ));
                        }
                        _ => {
                            self.chars.next();
                        }
                    }
                }
                Err(LexicalError::PrematureEndOfStream(i))
            }
            _ => Ok((i, Tok::Eq, i + 1)),
        }
    }

    #[inline]
    fn is_close_element(val: &str) -> bool {
        val.starts_with("End")
    }

    #[inline]
    fn trim_start(s: &str, i: usize) -> usize {
        i + Lexer::count_whitespaces(s[i..].chars())
    }

    #[inline]
    fn trim_end(s: &str, i: usize) -> usize {
        i - Lexer::count_whitespaces(s[..i].chars().rev())
    }

    #[inline]
    fn count_whitespaces<I: Iterator<Item = char>>(it: I) -> usize {
        it.take_while(|c| matches!(*c, ' ' | '\t')).count()
    }

    fn current(&mut self, i: usize, ch: char) -> Option<Spanned<Tok<'a>, usize, LexicalError>> {
        let spanned = match ch {
            '\r' | '\n' => self.section_key(i),
            '=' => self.section_value(i),
            ',' => Ok((i, Tok::Comma, i + 1)),
            ')' | ' ' | '\t' => Ok((i, Tok::Skip, i + 1)),
            '0'..='9' => self.digits_with_dots(i),
            '"' => self.string(i),
            '#' => Ok(self.comment(i)),
            'a'..='z' | 'A'..='Z' => Ok(self.identifier(i)),
            _ => return None,
        };
        Some(spanned)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Spanned<Tok<'a>, usize, LexicalError>;

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
