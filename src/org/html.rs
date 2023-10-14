use crate::org::{Document, Node};
use build_html::{Container, ContainerType, Html, HtmlContainer, Table};

pub struct HtmlBuilder {
    builder: Container,
}

impl HtmlBuilder {
    pub fn new() -> Self {
        Self {
            builder: Container::new(ContainerType::Div).with_attributes(vec![("class", "article")]),
        }
    }

    pub fn from_document(&mut self, doc: &Document) -> String {
        for section in &doc.sections {
            if section.commented {
                continue;
            }

            for node in &section.nodes {
                match node {
                    Node::Heading { level, title, .. } => {
                        self.builder.add_header(*level, title);
                    }
                    Node::Paragraph(content) => {
                        self.builder.add_paragraph(content.replace("\n", "<br />"));
                    }
                    Node::LesserBlock {
                        type_,
                        args,
                        contents,
                    } => match type_.as_str() {
                        "src" => {
                            if args.len() > 0 {
                                self.builder.add_preformatted(format!(
                                    "<code class=\"language-{}\">{}</code>",
                                    args[0],
                                    contents
                                ));
                            } else {
                                self.builder.add_preformatted(format!(
                                    "<code>{}</code>",
                                    contents
                                ));
                            }
                        },
                        "export" => {
                            if args.last() == Some(&"html".to_owned()) {
                                self.builder.add_raw(contents);
                            }
                        },
                        _ => {
                            todo!();
                        }
                    },
                    Node::Table { rows } => {
                        self.builder.add_table(Table::from(rows));
                    }
                }
            }
        }

        self.builder.to_html_string()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use crate::org::{html::HtmlBuilder, Document};

    #[test]
    fn headings() {
        assert_eq!(
            HtmlBuilder::new()
                .from_document(&Document::parse("* Hello, World!", "heading.org").unwrap()),
            "<div class=\"article\"><h1>Hello, World!</h1></div>"
        )
    }

    #[test]
    fn paragraphs() {
        assert_eq!(
            HtmlBuilder::new().from_document(
                &Document::parse(
                    r#"Hello,
  world!
Hewwo!

Hai!"#,
                    "paragraphs.org"
                )
                .unwrap()
            ),
            "<div class=\"article\"><p>Hello, world!<br />Hewwo!</p><p>Hai!</p></div>"
        )
    }

    #[test]
    fn py_src() {
        assert_eq!(
            HtmlBuilder::new().from_document(&Document::parse(r#"#+BEGIN_SRC python
print('Hello, world!')
#+END_SRC"#, "py_src.org").unwrap()),
            "<div class=\"article\"><pre><code class=\"language-python\">print('Hello, world!')</code></pre></div>"
        )
    }

    #[test]
    fn table() {
        assert_eq!(
            HtmlBuilder::new().from_document(&Document::parse(r#"
| a | b | c |
| 1 | 2 | 3 |
"#, "table.org").unwrap()),
            "<div class=\"article\"><table><thead></thead><tbody><tr><td></td><td>a</td><td>b</td><td>c</td><td></td></tr><tr><td></td><td>1</td><td>2</td><td>3</td><td></td></tr></tbody></table></div>"
        )
    }
}
