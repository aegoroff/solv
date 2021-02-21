use std::str::CharIndices;

pub type Spanned<Tok, Loc, Error> = Result<(Loc, Tok, Loc), Error>;

#[derive(Copy, Clone, Debug)]
pub enum Tok<'input> {
    Comment(&'input str),
    Str(&'input str),
    Guid(&'input str),
    Id(&'input str),
    DigitsAndDots(&'input str),
    Comma,
    Dot,
    Eq,
    ParenOpen,
    ParenClose,
    Pipe,
    Skip,
}

pub struct Lexer<'input> {
    chars: std::iter::Peekable<CharIndices<'input>>,
    input: &'input str,
    inside_str: bool,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Lexer {
            chars: input.char_indices().peekable(),
            input,
            inside_str: false,
        }
    }

    fn current(&mut self, i: usize, ch: char) -> Option<Spanned<Tok<'input>, usize, ()>> {
        match ch {
            '(' => return Some(Ok((i, Tok::ParenOpen, i + 1))),
            ')' => return Some(Ok((i, Tok::ParenClose, i + 1))),
            '|' => return Some(Ok((i, Tok::Pipe, i + 1))),
            ',' => return Some(Ok((i, Tok::Comma, i + 1))),
            '.' => return Some(Ok((i, Tok::Dot, i + 1))),
            '=' => return Some(Ok((i, Tok::Eq, i + 1))),
            // Digits and dots
            '0'..='9' => loop {
                match self.chars.peek() {
                    Some((j, ' ')) | Some((j, '(')) | Some((j, ')')) | Some((j, '\r'))
                    | Some((j, '\n')) | Some((j, '\t')) => {
                        return Some(Ok((i, Tok::DigitsAndDots(&self.input[i..*j]), *j)));
                    }
                    None => {
                        return Some(Ok((
                            i,
                            Tok::DigitsAndDots(&self.input[i..]),
                            self.input.len(),
                        )));
                    }
                    _ => {
                        self.chars.next();
                    }
                }
            },
            // Guid
            '{' => loop {
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
            },
            // Quoted string
            '"' => {
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
                        Some((j, '{')) => {
                            guid = true;
                            self.chars.next();
                        }
                        Some((j, '"')) => {
                            if guid {
                                return Some(Ok((i+1, Tok::Guid(&self.input[i+1..*j]), *j-1)));
                            } else {
                                return Some(Ok((i, Tok::Str(&self.input[i..*j + 1]), *j)));
                            }
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
            // Comment
            '#' => loop {
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
            },
            // Identifier
            'a'..='z' | 'A'..='Z' => loop {
                match self.chars.peek() {
                    Some((j, ' ')) | Some((j, '(')) | Some((j, ')')) | Some((j, '\r'))
                    | Some((j, '\n')) | Some((j, '\t')) | Some((j, '|')) | Some((j, ',')) => {
                        return Some(Ok((i, Tok::Id(&self.input[i..*j]), *j)));
                    }
                    None => {
                        return Some(Ok((
                            i,
                            Tok::Id(&self.input[i..]),
                            self.input.len(),
                        )));
                    }
                    _ => {
                        self.chars.next();
                    }
                }
            },
            _ => {},
        }
        None
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Spanned<Tok<'input>, usize, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.chars.next() {
                Some((_, ' ')) | Some((_, '\n')) | Some((_, '\r')) | Some((_, '\t'))
                | Some((_, '}')) => continue,
                None => return None, // End of file
                Some((i, c)) => {
                    let r = self.current(i, c);
                    if let Some(r) = r {
                        if let Ok(t) = r {
                           if let Tok::Skip = t.1 {
                               continue;
                           }
                        };
                    };
                    return r;
                },
            }
        }
    }
}
