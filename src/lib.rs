pub enum Token {
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
