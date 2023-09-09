/// Represents AST node type
#[derive(Debug)]
pub enum Node<'a> {
    Comment(&'a str),
    DigitOrDot(&'a str),
    Guid(&'a str),
    Identifier(&'a str),
    Str(&'a str),
    Version(Box<Node<'a>>, Box<Node<'a>>),
    FirstLine(Box<Node<'a>>),
    Global(Vec<Node<'a>>),
    Project(Box<Node<'a>>, Vec<Node<'a>>),
    ProjectBegin(Box<Node<'a>>, Box<Node<'a>>, Box<Node<'a>>, Box<Node<'a>>),
    Section(Box<Node<'a>>, Vec<Node<'a>>),
    SectionBegin(Vec<Node<'a>>, Box<Node<'a>>),
    SectionContent(Box<Node<'a>>, Box<Node<'a>>),
    SectionKey(Box<Node<'a>>),
    SectionValue(Box<Node<'a>>),
    Solution(Box<Node<'a>>, Vec<Node<'a>>),
}

/// Generates simple &str getters from Node variants
macro_rules! impl_str_getters {
    ($(($name:ident, $variant:ident)),*) => {
        $(
            #[must_use] pub fn $name(&self) -> &'a str {
                if let Node::$variant(s) = self {
                    *s
                } else {
                    ""
                }
            }
        )*
    };
}

impl<'a> Node<'a> {
    impl_str_getters!(
        (identifier, Identifier),
        (digit_or_dot, DigitOrDot),
        (string, Str),
        (guid, Guid)
    );

    #[must_use]
    pub fn is_section(&self, name: &str) -> bool {
        if let Node::SectionBegin(names, _) = self {
            names.iter().any(|n| n.identifier() == name)
        } else {
            false
        }
    }
}