use std::collections::HashMap;

mod lex;

enum ASTNode {
    Heading {
        level: String,
        name: String,
        todo_status: Option<String>,
        children: Vec<ASTNode>,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct Section {}

#[derive(Debug, Eq, PartialEq)]
pub struct Document {
    metadata: HashMap<String, String>,
    sections: Vec<Section>,
}

impl Document {
    pub fn parse(content: &str) -> Self {
        todo!();
    }

    pub fn parse_file(filename: &str) -> Self {
        todo!();
    }
}

#[cfg(test)]
mod test {
    use crate::org::Document;
    use std::collections::HashMap;

    #[test]
    fn test_parser() {
        assert_eq!(
            Document::parse("#+TITLE: hello"),
            Document {
                metadata: HashMap::<String, String>::from_iter(vec![(
                    "title".into(),
                    "this is the title".into()
                )]),
                sections: vec![]
            }
        );
    }
}
