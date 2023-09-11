use crate::org::{Document, Node};
use build_html::{Container, ContainerType, Html, HtmlContainer};

struct HtmlBuilder {
    builder: Container
}

impl HtmlBuilder {
    pub fn new() -> Self {
        Self {
            builder: Container::new(ContainerType::Article)
        }
    }

    pub fn from_document(&mut self, doc: Document) -> String {
        for section in doc.sections {
            for node in section.nodes {
                match node {
                    Node::Heading { level, title, .. } => {
                        self.builder.add_header(level, title);
                    },
                    Node::Paragraph (content) => {
                        self.builder.add_paragraph(content.replace("\n", "<br/>"));
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
            HtmlBuilder::new().from_document(Document::parse("* Hello, World!", "heading.org").unwrap()),
            "<article><h1>Hello, World!</h1></article>"
        )
    }

    #[test]
    fn paragraphs() {
        assert_eq!(
            HtmlBuilder::new().from_document(Document::parse("Hello,\n  world!\nHewwo!\n\nHai!", "paragraphs.org").unwrap()),
            "<article><p>Hello, world!<br/>Hewwo!</p><p>Hai!</p></article>"
        )
    }
}
