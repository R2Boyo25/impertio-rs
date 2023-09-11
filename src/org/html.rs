use crate::org::{Document, Node};
use build_html::{Container, ContainerType, Html, HtmlContainer};

struct HtmlBuilder {
    builder: Container,
}

impl HtmlBuilder {
    pub fn new() -> Self {
        Self {
            builder: Container::new(ContainerType::Article),
        }
    }

    pub fn from_document(&mut self, doc: Document) -> String {
        for section in doc.sections {
            for node in section.nodes {
                match node {
                    Node::Heading { level, title, .. } => {
                        self.builder.add_header(level, title);
                    }
                    Node::Paragraph(content) => {
                        self.builder.add_paragraph(content.replace("\n", "<br/>"));
                    }
                    Node::LesserBlock {
                        type_,
                        args,
                        contents,
                    } => match type_.as_str() {
                        "src" => {
                            if args.len() > 0 {
                                self.builder.add_preformatted(format!(
                                    "<code class=\"hljs {}\">{}</code>",
                                    args[0],
                                    contents.replace("\n", "<br/>")
                                ));
                            } else {
                                self.builder.add_preformatted(format!(
                                    "<code>{}</code>",
                                    contents.replace("\n", "<br/>")
                                ));
                            }
                        }
                        _ => {
                            todo!();
                        }
                    },
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
                .from_document(Document::parse("* Hello, World!", "heading.org").unwrap()),
            "<article><h1>Hello, World!</h1></article>"
        )
    }

    #[test]
    fn paragraphs() {
        assert_eq!(
            HtmlBuilder::new().from_document(
                Document::parse(
                    r#"Hello,
  world!
Hewwo!

Hai!"#,
                    "paragraphs.org"
                )
                .unwrap()
            ),
            "<article><p>Hello, world!<br/>Hewwo!</p><p>Hai!</p></article>"
        )
    }

    #[test]
    fn py_src() {
        assert_eq!(
            HtmlBuilder::new().from_document(Document::parse(r#"#+BEGIN_SRC python
print('Hello, world!')
#+END_SRC"#, "py_src.org").unwrap()),
            "<article><pre><code class=\"hljs python\">print('Hello, world!')</code></pre></article>"
        )
    }
}
