// SPDX-FileCopyrightText: 2024 Ohin "Kazani" Taylor <kazani@kazani.dev>
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

mod html;
mod lex;
pub mod inline;

use build_html::{Container, ContainerType, Html, HtmlContainer};
use lex::{Lexer, TokenKind};

use crate::{handler::FileContext, metadata::Metadata};

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
    pub fn parse(content: &str, filename: &str, ctx: FileContext) -> Result<Self, String> {
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
                TokenKind::Macro { name, args } => match name.as_str() {
                    "listing" => slf.sections.push(Section {
                        nodes: vec![
                            Node::Heading {
                                level: 1,
                                title: "Articles".into(),
                                todo_state: None,
                                tags: vec![],
                                commented: false,
                            },
                            Node::LesserBlock {
                                type_: "export".into(),
                                args: vec!["html".into()],
                                contents: Container::new(ContainerType::Div)
                                    .with_attributes([("class", "articles")])
                                    .with_raw(
                                        ctx.metadata
                                            .lock()
                                            .unwrap()
                                            .iter()
                                            .filter_map(|meta| match meta {
                                                Metadata::Article {
                                                    title,
                                                    description,
                                                    author,
                                                    tags,
                                                    modified,
                                                    url,
                                                } => {
                                                    if url.starts_with(
                                                        &(ctx.site_url.clone() + &args[0]),
                                                    ) {
                                                        let mut attributes = vec![
                                                            (
                                                                "data-title".into(),
                                                                title.to_string(),
                                                            ),
                                                            (
                                                                "data-last-modified".into(),
                                                                modified.to_rfc3339(),
                                                            ),
                                                        ];

                                                        if let Some(description) = description {
                                                            attributes.push((
                                                                "data-description".into(),
                                                                description.to_string(),
                                                            ));
                                                        }

                                                        if let Some(author) = author {
                                                            attributes.push((
                                                                "data-author".into(),
                                                                author.to_string(),
                                                            ));
                                                        }

                                                        if tags.len() > 0 {
                                                            attributes.push((
                                                                "data-tags".into(),
                                                                tags.join(", "),
                                                            ));
                                                        }

                                                        let mut container: Container =
                                                            Container::new(ContainerType::Div)
                                                                .with_attributes(attributes);

                                                        container.add_paragraph_attr(
                                                            title,
                                                            [("class", "card-title")],
                                                        );

                                                        if let Some(description) = description {
                                                            container.add_paragraph(description);
                                                        }

                                                        let mut end_container =
                                                            Container::new(ContainerType::Div)
                                                                .with_raw(format!(
                                                    "<span class=\"card-time\">{}</span>",
                                                    build_html::escape_html(&modified.to_rfc3339())
                                                ));

                                                        if let Some(author) = author {
                                                            end_container.add_raw(format!(
                                                        "<span class=\"card-author\">{}</span>",
                                                        build_html::escape_html(author)
                                                    ));
                                                        }

                                                        container.add_container(end_container);

                                                        Some(format!(
                                                    "<a href=\"{}\" class=\"article-card\">{}</a>",
                                                    url,
                                                    container.to_html_string()
                                                ))
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => None,
                                            })
                                            .map(|a| a.to_html_string())
                                            .collect::<Vec<String>>()
                                            .join(""),
                                    )
                                    .to_html_string(),
                            },
                        ],
                        commented: false,
                    }),
                    _ => todo!("Macro `{}` not defined.", name),
                },
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

    pub fn parse_file(filename: &str, ctx: FileContext) -> Result<Self, String> {
        Self::parse(
            &std::fs::read_to_string(filename).map_err(|_| "IO error of some kind".to_owned())?,
            filename,
            ctx,
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
            Document::parse("#+TITLE: hello", "hello.org", Default::default()),
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
            Document::parse("* test", "heading.org", Default::default()),
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
                "py_hello.org",
                Default::default()
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
                "comment_heading.org",
                Default::default()
            ),
            Ok(Document {
                metadata: HashMap::new(),
                sections: vec![]
            })
        )
    }
}
