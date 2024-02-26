// SPDX-FileCopyrightText: 2024 Ohin "Kazani" Taylor <kazani@kazani.dev>
// SPDX-License-Identifier: MIT

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tera::{Context, Tera};

#[derive(Clone, Debug)]
pub struct Templates {
    dir: PathBuf,
}

impl Templates {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            dir: data_dir.to_owned(),
        }
    }

    /// Creates a Tera instance with the files and dirs
    /// Also disables autoescape
    fn create_tera(files: Vec<&Path>, dirs: Vec<&Path>) -> Result<Tera, tera::Error> {
        let mut tera: Tera = Tera::default();

        tera.add_template_files(
            files
                .iter()
                .map(|path| (*path, path.file_name().unwrap().to_str()))
                .collect::<Vec<(&Path, Option<&str>)>>(),
        )?;
        for dir in dirs {
            let mut pb = dir.to_owned();
            pb.push("**");
            pb.push("*");

            tera.extend(&Tera::parse(pb.to_str().unwrap())?)?;
        }

        tera.autoescape_on(vec![]); // I trust the page-writer not to XSS themself with a static site.

        Ok(tera)
    }

    /// Render a page.
    pub fn render(
        &self,
        template: &str,
        file: &Path,
        contents: &str,
        ctx: Option<HashMap<&str, String>>,
    ) -> Result<String, tera::Error> {
        let mut context: Context = Context::new();
        context.insert("content", contents);

        if let Some(ctx) = ctx {
            for (key, value) in ctx.iter() {
                context.insert(*key, value);
            }
        }

        let tera = Self::create_tera(
            Self::find_upwards(
                file.parent().expect("Somehow the parent doesn't exist."),
                "root.html",
                Some(&self.dir),
            )
            .iter()
            .map(|path| path.as_path())
            .collect(),
            vec![],
        )?;

        tera.render(template, &context)
    }

    /// Find every instance of a file or directory upwards in the directory tree.
    fn find_upwards(dir: &Path, entry_name: &str, until: Option<&Path>) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = vec![];
        let mut dir: PathBuf = dir.to_owned();

        loop {
            if Self::concat_pathbuf(&dir, entry_name).exists() {
                found.push(Self::concat_pathbuf(&dir, entry_name));
            }

            if let Some(parent) = dir.parent() {
                dir = parent.to_owned();

                if dir.parent() == until {
                    break;
                }
            } else {
                break;
            }
        }

        found.iter().rev().map(|path| path.to_owned()).collect()
    }

    /// Helper method for concatenating a str to a path
    fn concat_pathbuf(pb: &Path, s: &str) -> PathBuf {
        let mut new_pb: PathBuf = pb.to_owned();
        new_pb.push(s);
        new_pb
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::Path};

    use crate::template::Templates;

    #[test]
    fn test() {
        let templates = Templates::new(&Path::new("data"));

        assert_eq!(
            templates
                .render(
                    "root.html",
                    &Path::new("data/index.org"),
                    "<h1>This is a test!</h1>",
                    Some(HashMap::from_iter(vec![("title", "yes".into())]))
                )
                .unwrap(),
            "<html>\n  <head><title>yes</title></head>\n  <body><h1>This is a test!</h1></body>\n</html>\n"
                .to_owned()
        )
    }
}
