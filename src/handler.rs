use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use crate::{files::FileDispatcher, metadata::Metadata, org::Document, template::Templates};

fn file_changed(old: &Path, new: &Path) -> std::io::Result<bool> {
    Ok(!new.exists() || new.metadata()?.modified()? < old.metadata()?.modified()?)
}

fn writeable(path: &Path) -> std::io::Result<std::fs::File> {
    use std::fs::{create_dir_all, File};

    create_dir_all(path.parent().unwrap())?;
    File::create(path)
}

pub struct FileContext {
    relative_path: PathBuf,
    source_path: PathBuf,
    output_path: PathBuf,
    site_url: String,

    templates: Templates
}

impl FileContext {
    pub fn new(relative: &Path, source: &Path, output: &Path, templates: &Templates) -> Self {
        Self {
            relative_path: relative.to_owned(),
            source_path: source.to_owned(),
            output_path: output.to_owned(),
            site_url: std::env::var("IMPERTIO_SITE_URL").unwrap(),
            templates: templates.clone()
        }
    }
}

pub trait FileHandler {
    fn new(dispatcher: Arc<Mutex<FileDispatcher>>) -> Self
    where
        Self: Sized;
    fn handle_file(&mut self, ctx: FileContext) -> anyhow::Result<()>;
    fn extract_metadata(&mut self, ctx: FileContext) -> anyhow::Result<Metadata>;
}

pub struct OrgHandler {
    dispatcher: Arc<Mutex<FileDispatcher>>,
}

impl OrgHandler {
    fn parse_file(ctx: &FileContext) -> anyhow::Result<Document> {
        Ok(crate::org::Document::parse_file(ctx.source_path.to_str().unwrap()).unwrap())
    }
}

impl FileHandler for OrgHandler {
    fn new(dispatcher: Arc<Mutex<FileDispatcher>>) -> Self {
        Self { dispatcher }
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
            description: parsed.metadata.get("desc").cloned(),
            modified: std::fs::metadata(ctx.source_path.clone())?.modified()?.into(),
            // created: std::fs::metadata(ctx.source_path.clone())?.created()?.into(),
            url: format!("{}/{}", ctx.site_url, ctx.relative_path.clone().with_extension("html").display())
        })
    }
}

pub struct CopyHandler {
    dispatcher: Arc<Mutex<FileDispatcher>>,
}

impl FileHandler for CopyHandler {
    fn new(dispatcher: Arc<Mutex<FileDispatcher>>) -> Self {
        Self { dispatcher }
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
                    url: ctx.relative_path.to_str().unwrap().to_owned(),
                }),
                _ => Err(anyhow::anyhow!("File type not extractable to metadata.")),
            }
        } else {
            Err(anyhow::anyhow!("File has no extension. /shrug"))
        }
    }
}
