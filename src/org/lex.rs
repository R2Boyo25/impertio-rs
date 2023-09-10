use regex::{Regex, Match};
use lazy_static::lazy_static;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Location {
    file: String,
    line: u32
}

impl Location {
    pub fn incremented(&self) -> Self {
        Self {
            file: self.file.clone(),
            line: self.line + 1
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TokenKind {
    	/// Meant to be ignored, useful in some cases - RE: $\s*^
	EmptyLine,

	/// Any text that was not part of another block
	Paragraph { content: String },

	/// | cell | cell | cell |
	TableRow { cells: Vec<String> },

	/// (?stars:\*+) (?todo_state:(?:TODO)|(?:DONE))? (?priority:#\[[a-zA-Z0-9]\])? (?title:[^\n]+) (?tags:\:([a-zA-Z0-9_@#%]\:)+)
	/// level = stars.size()
	/// commented = title.starts_with(“COMMENT”)
	/// archived = tags.contains(“ARCHIVE”)
	Heading { level: u8, todo_state: Option<String>, priority: Option<String>, commented: bool, title: String, tags: Vec<String>, archived: bool },

	/// Blocks
	/// Lesser if type in ["src", "verse", "example", "export"] else Greater

	/// #+BEGIN_TYPE ... #+END_TYPE
	LesserBlock { _type: String },

	/// #+BEGIN_TYPE … #+END_TYPE
	GreaterBlock { _type: String },

	/// End Blocks

	/// #+NAME: content
	/// Note: #+INCLUDE: does not count as a Keyword and is immediately replaced with the file contents.
	Keyword { name: String, content: String },

	/// # some text
	/// #+BEGIN_COMMENT … #+END_COMMENT
	Comment {content : String},
	
	/// :NAME: … :end:
	Drawer { name: String },

  /// #+begin: NAME ARGUMENTS ... #+end
	DynBlock { _type: String, args: Vec<String> },

	/// \[(?label:[a-zA-Z0-9_-])\]: (?contents:.+)
	/// It ends at the next footnote definition, the next heading, two consecutive blank lines, or the end of buffer.
	FootNote { label: String, contents: String }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Token {
    kind: TokenKind,
    location: Location,
}

fn match_to_str(match_: Match) -> String {
    match_.as_str().trim().into()
}

pub struct Lexer {
    current_location: Location
}

lazy_static! {
    static ref heading_regex: Regex = Regex::new(r#"(?<stars>\*+)\s+(?<todo_state>(?:TODO|DONE)\s+)?(?<priority>#\[[a-zA-Z0-9]\]\s+)?(?<title>[^\n]+?)(?<tags>\s+\:([a-zA-Z0-9_@#%]+\:)+)?$"#).unwrap();
}

impl Lexer {
    pub fn new(filename: &str) -> Self {
        Self {
            current_location: Location {
                line: 0,
                file: filename.into()
            }
        }
    }

    fn wrap(&mut self, kind: TokenKind) -> Token {
        self.current_location = self.current_location.incremented();

        Token {
            location: self.current_location.clone(),
            kind
        }
    }
    
    pub fn lex(&mut self, content: &str) -> Vec<Token> {
        content.split(|char| char == '\n').map(|line| self.handle_line(line)).filter(|token| token.kind != TokenKind::EmptyLine).collect::<Vec<_>>()
    }

    fn handle_line(&mut self, line: &str) -> Token {
        if line.trim() == "" {
            self.wrap(TokenKind::EmptyLine)
        } else if let Some(caps) = heading_regex.captures(line) {
            let tags: Vec<String> = caps.name("tags").map(|x| match_to_str(x).trim_matches(':').to_owned()).unwrap_or("".into()).split(":").map(|x| x.to_owned()).collect();
            
            self.wrap(TokenKind::Heading {
                level: u8::try_from(caps["stars"].len()).unwrap(),
                todo_state: caps.name("todo_state").map(match_to_str),
                priority: caps.name("priority").map(match_to_str).map(|x| (x[2..x.len()-1]).to_owned()),
                commented: caps["title"].starts_with("COMMENT"),
                title: caps["title"].into(),
                archived: tags.contains(&"ARCHIVED".to_owned()),
                tags,
            })
        } else {
            println!("{}", line);
            todo!()            
        }
    }
}

#[cfg(test)]
mod test {
    use crate::org::lex::Lexer;
    
    #[test]
    fn test_lexer() {
        assert_eq!(Lexer::new("test.org").lex(r#"
* TODO #[A] COMMENT test :abc:
"#
        ), vec![
            
        ])
    }
}
