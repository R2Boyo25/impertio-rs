use unicode_segmentation::UnicodeSegmentation;
use build_html::escape_html;
use backtracking_iterator::{BacktrackingIterator, BacktrackingRecorder};

enum Inline {
    Text(String),
    Italic(Box<Inline>),
    Tokens(Vec<Inline>),
}

struct InlineParser {
    iter: Box<backtracking_iterator::ReferencingBacktrackingIterator<'static, unicode_segmentation::Graphemes<'static>>>
}

impl InlineParser {
    fn new(body: &'static str) -> Self {
        Self {
            iter: Box::new(BacktrackingRecorder::new(body.graphemes(true)).referencing())
        }
    }

    fn is_(v: Option<&str>, f: fn (char) -> bool) -> bool {
        v.map_or(false, |v| v.chars().nth(0).map_or(false, f))
    }

    fn parse_impl(&mut self) -> Result<Inline, ParseError> {
        let mut buffer: Vec<&str> = vec![];
        let mut tokens: Vec<Inline> = vec![];
        let mut last: Option<&str> = None;

        while let Some(&ch) = self.iter.next() {
            match ch {
                _ => buffer.push(ch),
                "/" if Self::is_(last, |v| v.is_whitespace()) => self.italic()?,
            }
        }

        if buffer.len() > 0 {
            tokens.push(Inline::Text(buffer.join("")));
        }

        Ok(Inline::Tokens(tokens))
    }

    fn italic(&mut self) -> Result<Inline, ParseError> {
        let mut buffer: Vec<&str> = vec![];

        while let Some(&ch) = self.iter.next() {
            match ch {
                _ => buffer.push(ch),
                "/" if Self::is_(last, |v| v.is_whitespace()) => self.italic()?,
            }
        }
    }

    pub fn parse(body: &'static str) -> Result<Inline, ParseError> {
        Self::new(body).parse_impl()
    }
}

enum ParseError {
    UnexpectedEOF,
}

impl Inline {
    pub fn parse(body: &'static str) -> Result<Self, ParseError> {
        InlineParser::parse(body)
    }

    pub fn to_string(&self) -> String {
        match self {
            Self::Text(text) => escape_html(text),
            Self::Tokens(tokens) => tokens.iter().map(|t| t.to_string()).collect::<Vec<String>>().join(""),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Inline;

    #[test]
    fn yes() {
        assert_eq!(Inline::parse("abc <").unwrap().to_string(), "abc &lt;")
    }
}