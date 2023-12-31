use std::collections::HashMap;

mod html;
mod lex;

use lex::{Lexer, TokenKind};

type Inner = String;

#[derive(Debug, Eq, PartialEq)]
pub enum Node {
    Heading {
        level: u8,
        title: Inner,
        todo_state: Option<String>,
        tags: Vec<String>,
        commented: bool,
    },
    Paragraph(String),
    LesserBlock {
        type_: String,
        args: Vec<String>,
        contents: Inner,
    },
    Table {
        rows: Vec<Vec<Inner>>,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct Section {
    pub nodes: Vec<Node>,
    pub commented: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Document {
    pub metadata: HashMap<String, String>,
    pub sections: Vec<Section>,
}

impl Document {
    pub fn parse(content: &str, filename: &str) -> Result<Self, String> {
        let mut slf = Self {
            metadata: HashMap::new(),
            sections: vec![Section {
                nodes: vec![],
                commented: false,
            }],
        };

        let lexed = Lexer::new(filename).lex(content)?;

        for token in lexed {
            match token.kind {
                TokenKind::Heading {
                    level,
                    todo_state,
                    title,
                    tags,
                    commented,
                    ..
                } => slf.add_to_last(Node::Heading {
                    level,
                    title,
                    todo_state,
                    tags,
                    commented,
                }),
                TokenKind::Paragraph { content } => slf.add_to_last(Node::Paragraph(content)),
                TokenKind::LesserBlock {
                    _type,
                    contents,
                    args,
                } => {
                    slf.add_to_last(Node::LesserBlock {
                        args: args
                            .split(" ")
                            .map(|x| x.to_owned())
                            .collect::<Vec<String>>(),
                        contents: contents.join("\n"),
                        type_: _type,
                    });
                }
                TokenKind::Table { rows } => slf.add_to_last(Node::Table { rows }),
                TokenKind::Keyword { name, content } => {
                    slf.metadata.insert(name, content);
                }
                TokenKind::Comment { .. } => {}
                _ => todo!(),
            }
        }

        Ok(slf)
    }

    fn add_to_last(&mut self, node: Node) {
        match node {
            Node::Heading { commented, .. } => {
                self.sections.push(Section {
                    nodes: vec![node],
                    commented,
                });
            }
            _ => {
                let len = self.sections.len() - 1;
                self.sections[len].nodes.push(node);
            }
        }
    }

    pub fn parse_file(filename: &str) -> Result<Self, String> {
        Self::parse(
            &std::fs::read_to_string(filename).map_err(|_| "IO error of some kind".to_owned())?,
            filename,
        )
    }

    pub fn to_html(&self) -> String {
        super::org::html::HtmlBuilder::new().from_document(self)
    }
}

#[cfg(test)]
mod test {
    use crate::org::{Document, Node, Section};
    use std::collections::HashMap;

    #[test]
    fn title() {
        assert_eq!(
            Document::parse("#+TITLE: hello", "hello.org"),
            Ok(Document {
                metadata: HashMap::<String, String>::from_iter(vec![(
                    "title".into(),
                    "hello".into()
                )]),
                sections: vec![Section {
                    nodes: vec![],
                    commented: false
                }]
            })
        );
    }

    #[test]
    fn heading() {
        assert_eq!(
            Document::parse("* test", "heading.org"),
            Ok(Document {
                metadata: HashMap::new(),
                sections: vec![
                    Section {
                        nodes: vec![],
                        commented: false
                    },
                    Section {
                        nodes: vec![Node::Heading {
                            level: 1,
                            title: "test".into(),
                            todo_state: None,
                            tags: vec![],
                            commented: false
                        }],
                        commented: false
                    }
                ]
            })
        )
    }

    #[test]
    fn py_src() {
        assert_eq!(
            Document::parse(
                "#+BEGIN_SRC python\nprint('Hello, world!')\n#+END_SRC",
                "py_hello.org"
            ),
            Ok(Document {
                metadata: HashMap::new(),
                sections: vec![Section {
                    nodes: vec![Node::LesserBlock {
                        type_: "src".into(),
                        args: vec!["python".into()],
                        contents: "print('Hello, world!')".into()
                    }],
                    commented: false
                }]
            })
        );
    }

    #[test]
    fn comment_heading() {
        assert_eq!(
            Document::parse(
                "* TODO COMMENT something\n\nsome text",
                "comment_heading.org"
            ),
            Ok(Document {
                metadata: HashMap::new(),
                sections: vec![]
            })
        )
    }
}
