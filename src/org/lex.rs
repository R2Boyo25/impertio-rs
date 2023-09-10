use lazy_static::lazy_static;
use regex::{Match, Regex};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Location {
    file: String,
    line: u32,
}

impl Location {
    pub fn incremented(&self) -> Self {
        Self {
            file: self.file.clone(),
            line: self.line + 1,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum TokenKind {
    /// Meant to be ignored, useful in some cases - RE: $\s*^
    EmptyLine,

    /// Any text that was not part of another block
    Paragraph {
        content: String,
    },

    /// | cell | cell | cell |
    TableRow {
        cells: Vec<String>,
    },

    /// (?stars:\*+) (?todo_state:(?:TODO)|(?:DONE))? (?priority:#\[[a-zA-Z0-9]\])? (?title:[^\n]+) (?tags:\:([a-zA-Z0-9_@#%]\:)+)
    /// level = stars.size()
    /// commented = title.starts_with(“COMMENT”)
    /// archived = tags.contains(“ARCHIVE”)
    Heading {
        level: u8,
        todo_state: Option<String>,
        priority: Option<String>,
        commented: bool,
        title: String,
        tags: Vec<String>,
        archived: bool,
    },

    Planning {
        _type: String,
        value: String,
    },

    /// Blocks
    /// Lesser if type in ["src", "verse", "example", "export"] else Greater

    /// #+BEGIN_TYPE ... #+END_TYPE
    LesserBlock {
        _type: String,
    },

    /// #+BEGIN_TYPE … #+END_TYPE
    GreaterBlock {
        _type: String,
    },

    /// End Blocks

    /// #+NAME: content
    /// Note: #+INCLUDE: does not count as a Keyword and is immediately replaced with the file contents.
    Keyword {
        name: String,
        content: String,
    },

    /// # some text
    /// #+BEGIN_COMMENT … #+END_COMMENT
    Comment {
        content: String,
    },

    /// :NAME: … :end:
    Drawer {
        name: String,
        contents: Vec<String>
    },

    /// #+begin: NAME ARGUMENTS ... #+end
    DynBlock {
        _type: String,
        args: Vec<String>,
    },

    /// \[(?label:[a-zA-Z0-9_-])\]: (?contents:.+)
    /// It ends at the next footnote definition, the next heading, two consecutive blank lines, or the end of buffer.
    FootNote {
        label: String,
        contents: String,
    },
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Token {
    kind: TokenKind,
    location: Location,
}

fn match_to_str(match_: Match) -> String {
    match_.as_str().trim().into()
}

fn variant_eq<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

#[derive(Eq, PartialEq)]
enum State {
    Default,
    Drawer { lines: Vec<String>, name: String },
    Block { _type: String, args: String, lines: Vec<String> }
}

pub struct Lexer {
    current_location: Location,
    last: Option<Token>,
    valid_for_initial_drawer: bool,
    state: State,
}

lazy_static! {
    static ref HEADING_REGEX: Regex = Regex::new(r#"(?<stars>\*+)\s+(?<todo_state>(?:TODO|DONE)\s+)?(?<priority>#\[[a-zA-Z0-9]\]\s+)?(?<title>[^\n]+?)(?<tags>\s+\:([a-zA-Z0-9_@#%]+\:)+)?$"#).unwrap();
    static ref PLANNING_REGEX: Regex = Regex::new(r"^\s+(?<type>\w+):\s*(?<value>[^\n]+)").unwrap();
    static ref DRAWER_REGEX: Regex = Regex::new(r"^\s+:(?<name>[\w_-]+):").unwrap();
    static ref CLOSE_DRAWER_REGEX: Regex = Regex::new(r"(?i)^\s+:end:").unwrap();
}

impl Lexer {
    pub fn new(filename: &str) -> Self {
        Self {
            current_location: Location {
                line: 1,
                file: filename.into(),
            },
            last: None,
            valid_for_initial_drawer: true,
            state: State::Default
        }
    }

    fn wrap(&mut self, kind: TokenKind) -> Option<Token> {
        Some(Token {
            location: self.current_location.clone(),
            kind,
        })
    }

    pub fn lex(&mut self, content: &str) -> Result<Vec<Token>, String> {
        let lines = content.split(|char| char == '\n');
        let mut tokens: Vec<Token> = vec![];

        for line in lines {
            if let Some(token) = self.handle_line(line) {
                tokens.push(token.clone());
                self.last = Some(token);
                self.valid_for_initial_drawer = matches!(
                    self.last,
                    Some(Token {
                        kind: TokenKind::Keyword { .. },
                        ..
                    }) | Some(Token {
                        kind: TokenKind::Drawer { .. },
                        ..
                    }) | None
                ) && self.valid_for_initial_drawer;
            }

            self.current_location = self.current_location.incremented();
        }

        if self.state != State::Default {
            return Err("Unexpected EOF.".into());
        }
        
        Ok(tokens
            .iter()
            .filter(|token| token.kind != TokenKind::EmptyLine)
            .map(|x| x.to_owned())
            .collect::<Vec<_>>())
    }

    fn handle_line(&mut self, line: &str) -> Option<Token> {
        match &self.state {
            State::Default => self.handle_normal(line),
            State::Drawer {
                name,
                lines
            } => {
                if let Some(caps) = CLOSE_DRAWER_REGEX.captures(line) {
                    let token = self.wrap(
                        TokenKind::Drawer {
                            name: name.to_owned(),
                            contents: lines.to_owned()
                        }
                    );
                        
                    self.state = State::Default;

                    token
                } else {
                    None
                }
            },
            State::Block {
                _type,
                lines,
                args
            } => todo!()
        }
    }

    fn handle_normal(&mut self, line: &str) -> Option<Token> {
        if line.trim() == "" {
            self.wrap(TokenKind::EmptyLine)
        } else if let Some(caps) = HEADING_REGEX.captures(line) {
            let tags: Vec<String> = caps
                .name("tags")
                .map(|x| match_to_str(x).trim_matches(':').to_owned())
                .unwrap_or("".into())
                .split(":")
                .map(|x| x.to_owned())
                .collect();

            self.wrap(TokenKind::Heading {
                level: u8::try_from(caps["stars"].len()).unwrap(),
                todo_state: caps.name("todo_state").map(match_to_str),
                priority: caps
                    .name("priority")
                    .map(match_to_str)
                    .map(|x| (x[2..x.len() - 1]).to_owned()),
                commented: caps["title"].starts_with("COMMENT"),
                title: caps["title"].into(),
                archived: tags.contains(&"ARCHIVED".to_owned()),
                tags,
            })
        } else if {
            matches!(
                self.last,
                Some(Token {
                    kind: TokenKind::Planning { .. },
                    ..
                }) | Some(Token {
                    kind: TokenKind::Heading { .. },
                    ..
                })
            ) && matches!(PLANNING_REGEX.captures(line), Some(_)) } {
            let caps = PLANNING_REGEX.captures(line).unwrap();
            self.wrap(TokenKind::Planning {
                _type: caps["type"].into(),
                value: caps["value"].into(),
            })
        } else if let Some(caps) = DRAWER_REGEX.captures(line) {
            self.state = State::Drawer {
                name: caps["name"].to_owned(),
                lines: vec![]
            };

            None
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
        assert_eq!(
            Lexer::new("test.org").lex(
                r#"
* TODO #[A] COMMENT test :abc:
    DEADLINE: tomorrow
    :drawer:
    something: nothing
    :enD:
"#
            ),
            Ok(vec![])
        )
    }
}
