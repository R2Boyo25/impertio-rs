use lazy_static::lazy_static;
use fancy_regex::{Match, Regex};

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
        completion_amount: Option<String>
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
        contents: Vec<String>,
        args: String,
    },

    /// #+BEGIN_TYPE … #+END_TYPE
    GreaterBlock {
        _type: String,
        contents: Vec<String>,
        args: String,
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
        contents: Vec<String>,
    },

    /// #+begin: NAME ARGUMENTS ... #+end
    DynBlock {
        args: String,
        contents: Vec<String>,
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

#[derive(Debug, Eq, PartialEq)]
enum State {
    Default,
    Drawer {
        lines: Vec<String>,
        name: String,
    },
    Block {
        _type: Option<String>,
        args: String,
        lines: Vec<String>,
    },
}

pub struct Lexer {
    current_location: Location,
    last: Option<Token>,
    valid_for_initial_drawer: bool,
    state: State,
}

lazy_static! {
    static ref HEADING_REGEX: Regex = Regex::new(r#"(?<stars>\*+)\s+(?<todo_state>(?:(?!COMMENT)[A-Z]{2,})\s+)?(?<priority>#\[[a-zA-Z0-9]\]\s+)?(?<title>[^\n]+?)(?<tags>\s+\:([a-zA-Z0-9_@#%]+\:)+)?(?:\s+\[(?<completion_amount>(?:\d+\/\d+)|(?:[\d.]+%))\])?$"#).unwrap();
    static ref PLANNING_REGEX: Regex = Regex::new(r"^\s+(?<type>\w+):\s*(?<value>[^\n]+)").unwrap();
    static ref DRAWER_REGEX: Regex = Regex::new(r"^\s+:(?<name>[\w_-]+):").unwrap();
    static ref CLOSE_DRAWER_REGEX: Regex = Regex::new(r"(?i)^\s+:end:").unwrap();
    static ref BLOCK_REGEX: Regex = Regex::new(r"(?i)^#\+BEGIN(?:_(?<type>[a-zA-Z]+))?:?\s*(?<args>(?:.+)?)$").unwrap();
    static ref CLOSE_BLOCK_REGEX: Regex = Regex::new(r"(?i)^#\+END(?:_(?<type>[a-zA-Z]+))").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"^#\s+(?<content>.+)").unwrap();
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
            state: State::Default,
        }
    }

    fn wrap(&self, kind: TokenKind) -> Option<Token> {
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

    fn construct_block(
        &self,
        type_: Option<String>,
        lines: Vec<String>,
        args: String,
    ) -> Option<Token> {
        match type_ {
            Some(type_) => match type_.as_str() {
                "comment" => self.wrap(TokenKind::Comment {
                    content: lines.join("\n"),
                }),
                "src" | "verse" | "example" | "export" => self.wrap(TokenKind::LesserBlock {
                    _type: type_,
                    contents: lines,
                    args,
                }),
                _ => self.wrap(TokenKind::GreaterBlock {
                    _type: type_,
                    contents: lines,
                    args,
                }),
            },
            None => self.wrap(TokenKind::DynBlock {
                args,
                contents: lines,
            }),
        }
    }

    fn handle_line(&mut self, line: &str) -> Option<Token> {
        match &self.state {
            State::Default => self.handle_normal(line),
            State::Drawer { name, lines } => {
                if let Ok(Some(_)) = CLOSE_DRAWER_REGEX.captures(line) {
                    let token = self.wrap(TokenKind::Drawer {
                        name: name.to_owned(),
                        contents: lines.to_owned(),
                    });

                    self.state = State::Default;

                    token
                } else {
                    let mut tmp_lines: Vec<String> = lines.to_owned();

                    tmp_lines.push(line.to_owned());

                    self.state = State::Drawer {
                        lines: tmp_lines,
                        name: name.to_owned(),
                    };

                    None
                }
            }
            State::Block { _type, lines, args } => {
                if let Ok(Some(caps)) = CLOSE_BLOCK_REGEX.captures(line) {
                    if caps
                        .name("type")
                        .map(match_to_str)
                        .map(|x| x.to_ascii_lowercase())
                        != *_type
                    {
                        panic!("Closing a block of a different type.")
                    }

                    let token =
                        self.construct_block(_type.to_owned(), lines.to_owned(), args.to_owned());

                    self.state = State::Default;

                    token
                } else {
                    let mut tmp_lines: Vec<String> = lines.to_owned();

                    tmp_lines.push(line.to_owned());

                    self.state = State::Block {
                        lines: tmp_lines,
                        _type: _type.to_owned(),
                        args: args.to_owned(),
                    };

                    None
                }
            }
        }
    }

    fn handle_normal(&mut self, line: &str) -> Option<Token> {
        if line.trim() == "" {
            self.wrap(TokenKind::EmptyLine)
        } else if let Ok(Some(caps)) = HEADING_REGEX.captures(line) {
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
                completion_amount: caps.name("completion_amount").map(match_to_str)
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
            ) && matches!(PLANNING_REGEX.captures(line), Ok(Some(_)))
        } {
            let caps = PLANNING_REGEX.captures(line).unwrap().unwrap();
            self.wrap(TokenKind::Planning {
                _type: caps["type"].into(),
                value: caps["value"].into(),
            })
        } else if let Ok(Some(caps)) = DRAWER_REGEX.captures(line) {
            self.state = State::Drawer {
                name: caps["name"].to_owned(),
                lines: vec![],
            };

            None
        } else if let Ok(Some(caps)) = BLOCK_REGEX.captures(line) {
            self.state = State::Block {
                _type: caps
                    .name("type")
                    .map(match_to_str)
                    .map(|x| x.to_ascii_lowercase()),
                args: caps["args"].to_owned(),
                lines: vec![],
            };

            None
        } else if let Ok(Some(caps)) = COMMENT_REGEX.captures(line) {
            self.wrap(TokenKind::Comment {
                content: caps["content"].to_owned(),
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
    use crate::org::lex::{Location, Token, TokenKind};

    #[test]
    fn test_heading() {
        assert_eq!(
            Lexer::new("headings.org").lex(
                r#"
* TODO #[A] COMMENT test :abc: [3%]
    DEADLINE: tomorrow
    :drawer:
    something: nothing
    :enD:
"#
            ),
            Ok(vec![])
        )
    }

    #[test]
    fn zeroth_section() {
        assert_eq!(
            Lexer::new("zero.org").lex(
                r#"    :drawer:
    abc: another
    :end:"#
            ),
            Ok(vec![])
        )
    }

    #[test]
    fn comments() {
        assert_eq!(
            Lexer::new("comments.org").lex(
                r#"#+BEGIN_COMMENT
hewwo
#+END_COMMENT
# another comment"#
            ),
            Ok(vec![
                Token {
                    kind: TokenKind::Comment {
                        content: "hewwo".into()
                    },
                    location: Location {
                        file: "comments.org".into(),
                        line: 3
                    }
                },
                Token {
                    kind: TokenKind::Comment {
                        content: "another comment".into()
                    },
                    location: Location {
                        file: "comments.org".into(),
                        line: 4
                    }
                }
            ])
        )
    }
}
