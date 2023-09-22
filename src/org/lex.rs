use fancy_regex::{Match, Regex};
use lazy_static::lazy_static;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Location {
    pub file: String,
    pub line: u32,
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
    /// | cell | cell | cell |
    Table {
        rows: Vec<Vec<String>>,
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
        completion_amount: Option<String>,
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
    /*
    /// \[(?label:[a-zA-Z0-9_-])\]: (?contents:.+)
    /// It ends at the next footnote definition, the next heading, two consecutive blank lines, or the end of buffer.
    FootNote {
        label: String,
        contents: String,
    },*/
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub location: Location,
}

fn match_to_str(match_: Match) -> String {
    match_.as_str().trim().into()
}

#[derive(Debug, Eq, PartialEq)]
enum State {
    Default,
    Drawer {
        lines: Vec<String>,
        name: String,
        start: Location
    },
    Block {
        _type: Option<String>,
        args: String,
        lines: Vec<String>,
        start: Location
    },
}

pub struct Lexer {
    current_location: Location,
    valid_for_initial_drawer: bool,
    state: State,
    tokens: Vec<Token>,
}

lazy_static! {
    static ref HEADING_REGEX: Regex = Regex::new(r#"(?<stars>\*+)\s+(?<todo_state>(?:(?!COMMENT)[A-Z]{2,})\s+)?(?<priority>#\[[a-zA-Z0-9]\]\s+)?(?<title>[^\n]+?)(?<tags>\s+\:([a-zA-Z0-9_@#%]+\:)+)?(?:\s+\[(?<completion_amount>(?:\d+\/\d+)|(?:[\d.]+%))\])?$"#).unwrap();
    static ref PLANNING_REGEX: Regex = Regex::new(r"^\s+(?<type>\w+):\s*(?<value>[^\n]+)").unwrap();
    static ref DRAWER_REGEX: Regex = Regex::new(r"^\s+:(?<name>[\w_-]+):").unwrap();
    static ref CLOSE_DRAWER_REGEX: Regex = Regex::new(r"(?i)^\s+:end:").unwrap();
    static ref BLOCK_REGEX: Regex = Regex::new(r"(?i)^#\+BEGIN(?:_(?<type>[a-zA-Z]+))?:?\s*(?<args>(?:.+)?)$").unwrap();
    static ref CLOSE_BLOCK_REGEX: Regex = Regex::new(r"(?i)^#\+END(?:_(?<type>[a-zA-Z]+))").unwrap();
    static ref COMMENT_REGEX: Regex = Regex::new(r"^#\s+(?<content>.+)").unwrap();
    static ref INDENTED: Regex = Regex::new(r"^\s+").unwrap();
    static ref TABLE_ROW: Regex = Regex::new(r"^(?<cells>\|.+)+\|?").unwrap();
    static ref KEYWORD: Regex = Regex::new(r"^#\+(?<name>[a-zA-Z_]+):\s*(?<value>.+)$").unwrap();
}

impl Lexer {
    pub fn new(filename: &str) -> Self {
        Self {
            current_location: Location {
                line: 1,
                file: filename.into(),
            },
            valid_for_initial_drawer: true,
            state: State::Default,
            tokens: vec![],
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

        for line in lines {
            if let Some(token) = self.handle_line(line) {
                self.tokens.push(token.clone());
                self.valid_for_initial_drawer = matches!(
                    self.tokens.last(),
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

        Ok(self
            .tokens
            .iter()
            .filter(|token| token.kind != TokenKind::EmptyLine)
            .map(|x| x.to_owned())
            .collect::<Vec<_>>())
    }

    fn lstrip_equally(lines: Vec<String>) -> Vec<String> {
        let shared_indent = lines.iter().map(|line| {
            INDENTED.find(line).unwrap().map_or_else(|| 0, |mtch: Match| mtch.end())
        }).reduce(std::cmp::min).unwrap();

        lines.iter().map(|line| (&line[shared_indent..]).to_owned()).collect()
    }

    fn construct_block(
        &self,
        type_: Option<String>,
        lines: Vec<String>,
        args: String,
        start: Location
    ) -> Option<Token> {
        Some(Token {
            kind: match type_ {
                Some(type_) => match type_.as_str() {
                    "comment" => TokenKind::Comment {
                        content: lines.join("\n"),
                    },
                    "src" | "verse" | "example" | "export" => TokenKind::LesserBlock {
                        _type: type_,
                        contents: Self::lstrip_equally(lines),
                        args,
                    },
                    _ => TokenKind::GreaterBlock {
                        _type: type_,
                        contents: lines,
                        args,
                    },
                },
                None => TokenKind::DynBlock {
                    args,
                    contents: lines,
                },
            },
            location: start
        })
    }

    fn handle_line(&mut self, line: &str) -> Option<Token> {
        match &self.state {
            State::Default => self.handle_normal(line),
            State::Drawer { name, lines, start } => {
                if let Ok(Some(_)) = CLOSE_DRAWER_REGEX.captures(line) {
                    let token = Token {
                        kind: TokenKind::Drawer {
                            name: name.to_owned(),
                            contents: lines.to_owned(),
                        },
                        location: start.clone()
                    };

                    self.state = State::Default;

                    Some(token)
                } else {
                    let mut tmp_lines: Vec<String> = lines.to_owned();

                    tmp_lines.push(line.to_owned());

                    self.state = State::Drawer {
                        lines: tmp_lines,
                        name: name.to_owned(),
                        start: start.to_owned()
                    };

                    None
                }
            }
            State::Block { _type, lines, args, start } => {
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
                        self.construct_block(_type.to_owned(), lines.to_owned(), args.to_owned(), start.clone());

                    self.state = State::Default;

                    token
                } else {
                    let mut tmp_lines: Vec<String> = lines.to_owned();

                    tmp_lines.push(line.to_owned());

                    self.state = State::Block {
                        lines: tmp_lines,
                        _type: _type.to_owned(),
                        args: args.to_owned(),
                        start: start.to_owned()
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
                .map(|x| match_to_str(x))
                .unwrap_or("".into())
                .split(":")
                .filter(|x| x != &"")
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
                completion_amount: caps.name("completion_amount").map(match_to_str),
            })
        } else if {
            matches!(
                self.tokens.last(),
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
                start: self.current_location.clone()
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
                start: self.current_location.clone()
            };

            None
        } else if let Ok(Some(caps)) = COMMENT_REGEX.captures(line) {
            self.wrap(TokenKind::Comment {
                content: caps["content"].to_owned(),
            })
        } else if let Ok(Some(caps)) = KEYWORD.captures(line) {
            self.wrap(TokenKind::Keyword {
                name: caps["name"].to_ascii_lowercase().into(),
                content: caps["value"].into(),
            })
        } else if TABLE_ROW.is_match(line).unwrap() {
            match self.tokens.last().clone() {
                Some(Token {
                    kind: TokenKind::Table { rows },
                    ..
                }) => {
                    let len = self.tokens.len() - 1;

                    let mut tmp_rows = rows.to_owned();
                    tmp_rows.push(
                        line.trim()
                            .split("|")
                            .map(|x| x.trim().to_owned())
                            .collect::<Vec<_>>(),
                    );

                    self.tokens[len] = Token {
                        kind: TokenKind::Table { rows: tmp_rows },
                        ..self.tokens.last().unwrap().to_owned()
                    };

                    None
                }
                _ => self.wrap(TokenKind::Table {
                    rows: vec![line
                        .trim()
                        .split("|")
                        .map(|x| x.trim().to_owned())
                        .collect::<Vec<_>>()],
                }),
            }
        } else {
            // if last == paragraph, add to paragraph
            //  if line.starts_with("\s"), merge lines
            //  else, newline
            // else, new paragraph

            match self.tokens.last().clone() {
                Some(Token {
                    kind: TokenKind::Paragraph { content },
                    ..
                }) => {
                    let len = self.tokens.len() - 1;
                    self.tokens[len] = Token {
                        kind: TokenKind::Paragraph {
                            content: if let Ok(Some(_)) = INDENTED.captures(line) {
                                content.trim_end().to_owned() + " " + line.trim_start()
                            } else {
                                content.trim_end().to_owned() + "\n" + line
                            },
                        },
                        ..self.tokens.last().unwrap().to_owned()
                    };

                    None
                }
                _ => self.wrap(TokenKind::Paragraph {
                    content: line.trim_start().into(),
                }),
            }
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
                        line: 1
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

    #[test]
    fn paragraphs() {
        assert_eq!(
            Lexer::new("paragraphs.org").lex(
                r#"Hewwo!
  How goes it?

Yoooooooo
noooo"#
            ),
            Ok(vec![
                Token {
                    kind: TokenKind::Paragraph {
                        content: "Hewwo! How goes it?".into()
                    },
                    location: Location {
                        file: "paragraphs.org".into(),
                        line: 1
                    }
                },
                Token {
                    kind: TokenKind::Paragraph {
                        content: "Yoooooooo\nnoooo".into()
                    },
                    location: Location {
                        file: "paragraphs.org".into(),
                        line: 4
                    }
                }
            ])
        )
    }

    #[test]
    fn lstrip_src() {
        assert_eq!(
            Lexer::new("src.org").lex(
                r#"#+BEGIN_SRC py
  normal
    indented
#+END_SRC"#
            ),
            Ok(vec![
                Token {
                    kind: TokenKind::LesserBlock {
                        _type: "src".into(),
                        contents: vec!["normal".into(), "  indented".into()],
                        args: "py".into()
                    },
                    location: Location {
                        file: "src.org".into(),
                        line: 1
                    }
                }
            ])
        )
    }
}
