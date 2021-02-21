use std::str::CharIndices;

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

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

pub struct Lexer<'input> {
    chars: std::iter::Peekable<CharIndices<'input>>,
    input: &'input str,
    inside_str: bool,
    tab_count: u32,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Lexer {
            chars: input.char_indices().peekable(),
            input,
            inside_str: false,
            tab_count: 0,
        }
    }

    fn identifier(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        loop {
            match self.chars.peek() {
                Some((j, c)) => match *c {
                    'a'..='z' | 'A'..='Z' => {
                        self.chars.next();
                    }
                    '(' => return Some(Ok((i, Tok::OpenElement(&self.input[i..*j]), *j))),
                    _ => return Some(Ok((i, Tok::Id(&self.input[i..*j]), *j))),
                },
                None => {
                    return Some(Ok((i, Tok::Id(&self.input[i..]), self.input.len())));
                }
            }
        }
    }

    fn comment(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        loop {
            match self.chars.peek() {
                Some((j, '\n')) | Some((j, '\r')) => {
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

    fn guid(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        loop {
            match self.chars.peek() {
                Some((j, '}')) => {
                    return Some(Ok((i, Tok::Guid(&self.input[i..*j + 1]), *j)));
                }
                None => {
                    return Some(Ok((i, Tok::Guid(&self.input[i..]), self.input.len())));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn digits_with_dots(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
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

    fn string(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        if self.inside_str {
            // Skip trailing
            self.inside_str = false;
            return Some(Ok((i, Tok::Skip, i + 1)));
        } else {
            self.inside_str = true;
        }
        let mut guid = false;
        loop {
            match self.chars.peek() {
                // Guid start
                Some((_, '{')) => {
                    guid = true;
                    self.chars.next();
                }
                Some((j, '"')) => {
                    return if guid {
                        Some(Ok((i + 1, Tok::Guid(&self.input[i + 1..*j]), *j - 1)))
                    } else {
                        Some(Ok((i, Tok::Str(&self.input[i..*j + 1]), *j)))
                    };
                }
                None => {
                    return Some(Ok((i, Tok::Str(&self.input[i..]), self.input.len())));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn section_key(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        self.tab_count += 1;

        loop {
            // Skip first
            if self.tab_count == 1 {
                return Some(Ok((i, Tok::Skip, i + 1)));
            }
            let start = i + 1;
            match self.chars.peek() {
                Some((j, '=')) => {
                    let finish = Lexer::trim_end(&self.input, *j);

                    return Some(Ok((
                        start,
                        Tok::SectionKey(&self.input[start..finish]),
                        finish,
                    )));
                }
                None => {
                    return Some(Ok((
                        start,
                        Tok::SectionKey(&self.input[start..]),
                        self.input.len(),
                    )));
                }
                _ => {
                    self.chars.next();
                }
            }
        }
    }

    fn section_value(&mut self, i: usize) -> Option<Spanned<Tok<'input>, usize, ()>> {
        if self.tab_count <= 1 {
            return Some(Ok((i, Tok::Eq, i + 1)));
        } else {
            let start = self.trim_start(i + 1);

            loop {
                match self.chars.peek() {
                    Some((j, '\r')) | Some((j, '\n')) => {
                        self.tab_count = 0;
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
    }

    fn trim_start(&self, mut i: usize) -> usize {
        loop {
            match &self.input[i..i + 1] {
                " " | "\t" => i += 1,
                _ => break,
            }
        }
        i
    }

    fn trim_end(s: &str, mut i: usize) -> usize {
        loop {
            match &s[i - 1..i] {
                " " | "\t" => i -= 1,
                _ => break,
            }
        }
        i
    }

    fn current(&mut self, i: usize, ch: char) -> Option<Spanned<Tok<'input>, usize, ()>> {
        match ch {
            '\t' => return self.section_key(i),
            '=' => return self.section_value(i),
            '(' => return Some(Ok((i, Tok::ParenOpen, i + 1))),
            ')' => return Some(Ok((i, Tok::ParenClose, i + 1))),
            ',' => return Some(Ok((i, Tok::Comma, i + 1))),
            '0'..='9' => return self.digits_with_dots(i),
            '{' => return self.guid(i),
            '"' => return self.string(i),
            '#' => return self.comment(i),
            'a'..='z' | 'A'..='Z' => return self.identifier(i),
            _ => {}
        }
        None
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Tok<'input>, usize, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (i, c) = self.chars.next()?;

            match c {
                ' ' | '\n' | '\r' | '}' => {
                    self.tab_count = 0;
                    continue;
                }
                _ => {}
            }

            let tok = self.current(i, c)?;

            if let Ok(t) = tok {
                if let (_, Tok::Skip, _) = t {
                    continue;
                }
            };

            return Some(tok);
        }
    }
}
