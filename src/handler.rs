// SPDX-FileCopyrightText: 2024 Ohin "Kazani" Taylor <kazani@kazani.dev>
// SPDX-License-Identifier: MIT

use dyn_clone::{clone_trait_object, DynClone};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{config::Config, metadata::Metadata, org::Document, template::Templates};

fn file_changed(old: &Path, new: &Path) -> std::io::Result<bool> {
    Ok(!new.exists() || new.metadata()?.modified()? < old.metadata()?.modified()?)
}

fn writeable(path: &Path) -> std::io::Result<std::fs::File> {
    use std::fs::{create_dir_all, File};

    create_dir_all(path.parent().unwrap())?;
    File::create(path)
}

#[derive(Clone, Debug)]
pub struct FileContext {
    pub relative_path: PathBuf,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub site_url: String,
    pub ext: String,

    pub templates: Templates,
}

impl FileContext {
    pub fn new(
        config: &Config,
        relative: &Path,
        source: &Path,
        output: &Path,
        templates: &Templates,
    ) -> Self {
        Self {
            relative_path: relative.to_owned(),
            source_path: source.to_owned(),
            output_path: output.to_owned(),
            ext: source
                .extension()
                .unwrap_or(&OsStr::new(""))
                .to_str()
                .unwrap_or("")
                .to_string(),
            site_url: config.site_url.clone(),
            templates: templates.clone(),
        }
    }
}

pub trait FileHandler: DynClone {
    fn new() -> Self
    where
        Self: Sized;
    fn handle_file(&mut self, ctx: FileContext) -> anyhow::Result<()>;
    fn extract_metadata(&mut self, ctx: FileContext) -> anyhow::Result<Metadata>;
}

clone_trait_object!(FileHandler);

#[derive(Clone)]
pub struct OrgHandler {}

impl OrgHandler {
    fn parse_file(ctx: &FileContext) -> anyhow::Result<Document> {
        Ok(crate::org::Document::parse_file(ctx.source_path.to_str().unwrap()).unwrap())
    }
}

impl FileHandler for OrgHandler {
    fn new() -> Self {
        Self {}
    }

    fn handle_file(&mut self, ctx: FileContext) -> anyhow::Result<()> {
        let file = ctx.source_path.clone();
        let html_file = ctx.output_path.with_extension("html");
        let source_file: PathBuf = ctx.output_path.with_extension("org");

        if !file_changed(&file, &html_file)? && !file_changed(&file, &source_file)? {
            return Ok(());
        }

        match file
            .file_stem()
            .unwrap_or(file.as_os_str())
            .to_str()
            .unwrap_or(file.to_str().unwrap())
        {
            "index" => log::info!(
                "Parsing index of {:?}",
                file.parent().unwrap_or(&Path::new("<root>"))
            ),
            _ => log::info!("Parsing Org file {:?}", file),
        }

        let parsed = Self::parse_file(&ctx)?;

        let out = ctx.templates.render(
            "root.html",
            &file,
            &parsed.to_html(),
            Some(
                parsed
                    .metadata
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.to_owned()))
                    .collect(),
            ),
        )?;

        writeable(&html_file)?.write_all(out.as_bytes())?;
        writeable(&source_file)?.write_all(std::fs::read(file.clone())?.as_slice())?;

        Ok(())
    }

    fn extract_metadata(&mut self, ctx: FileContext) -> anyhow::Result<Metadata> {
        let parsed = Self::parse_file(&ctx)?;

        Ok(Metadata::Article {
            title: parsed
                .metadata
                .get("title")
                .unwrap_or(
                    &ctx.output_path
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                )
                .to_string(),
            author: parsed.metadata.get("author").cloned(),
            description: parsed.metadata.get("desc").cloned(),
            modified: std::fs::metadata(ctx.source_path.clone())?
                .modified()?
                .into(),
            // created: std::fs::metadata(ctx.source_path.clone())?.created()?.into(),
            url: format!(
                "{}/{}",
                ctx.site_url,
                ctx.relative_path.clone().with_extension("html").display()
            ),
            tags: if let Some(tags) = parsed.metadata.get("tags") {
                tags.split(if tags.contains(",") {
                    |c: char| c == ','
                } else {
                    |c: char| c.is_whitespace()
                })
                .map(|tag| tag.to_owned())
                .collect()
            } else {
                vec![]
            },
        })
    }
}

#[derive(Clone)]
pub struct CopyHandler {}

impl FileHandler for CopyHandler {
    fn new() -> Self {
        Self {}
    }

    fn handle_file(&mut self, ctx: FileContext) -> anyhow::Result<()> {
        if !file_changed(&ctx.source_path, &ctx.output_path)? {
            return Ok(());
        }

        log::warn!(
            "File {:?} not recognized. Copying as-is...",
            ctx.source_path
        );

        writeable(&ctx.output_path)?.write_all(std::fs::read(ctx.source_path)?.as_slice())?;

        Ok(())
    }

    fn extract_metadata(&mut self, ctx: FileContext) -> anyhow::Result<Metadata> {
        if let Some(ext) = ctx.source_path.extension() {
            match ext.to_str().unwrap() {
                "png" | "jpg" | "jpeg" | "webm" | "gif" => Ok(Metadata::Image {
                    url: format!("{}/{}", ctx.site_url, ctx.relative_path.display()),
                }),
                _ => Err(anyhow::anyhow!("File type not extractable to metadata.")),
            }
        } else {
            Err(anyhow::anyhow!("File has no extension. /shrug"))
        }
    }
}
