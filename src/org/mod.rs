use std::collections::HashMap;

mod lex;
mod html;

use lex::{Lexer, TokenKind};

#[derive(Debug, Eq, PartialEq)]
pub enum Node {
    Heading {
        level: u8,
        title: String,
        todo_state: Option<String>,
        tags: Vec<String>
    },
    Paragraph (String)
}

#[derive(Debug, Eq, PartialEq)]
pub struct Section {
    pub nodes: Vec<Node>,
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
            sections: vec![Section { nodes: vec![] }]
        };
        
        let lexed = Lexer::new(filename).lex(content)?;

        for token in lexed {
            match token.kind {
                TokenKind::Heading { level, todo_state, title, tags, .. } => {
                    slf.add_to_last(Node::Heading {
                        level,
                        title,
                        todo_state,
                        tags
                    })
                },
                TokenKind::Paragraph { content } => {
                    slf.add_to_last(Node::Paragraph(content))
                }
                _ => todo!()
            }
        }

        Ok(slf)
    }

    fn add_to_last(&mut self, node: Node) {
        match node {
            Node::Heading {..} => {
                self.sections.push(Section { nodes: vec![node] });
            },
            _ => {
                let len = self.sections.len() - 1;
                self.sections[len].nodes.push(node);
            }
        }
    }

    pub fn parse_file(filename: &str) -> Self {
        todo!();
    }
}

#[cfg(test)]
mod test {
    use crate::org::{Document, Section, Node};
    use std::collections::HashMap;

    #[test]
    fn test_parser() {
        assert_eq!(
            Document::parse("#+TITLE: hello", "hello.org"),
            Ok(Document {
                metadata: HashMap::<String, String>::from_iter(vec![(
                    "title".into(),
                    "hello".into()
                )]),
                sections: vec![]
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
                        nodes: vec![]
                    },
                    Section {
                        nodes: vec![Node::Heading {
                            level: 1,
                            title: "test".into(),
                            todo_state: None,
                            tags: vec![]
                        }]
                    }
                ]
            })
        )
    }
}
