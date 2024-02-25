use crate::config::Config;
use crate::handler::{CopyHandler, FileContext, FileHandler, OrgHandler};
use crate::metadata::Metadata;
use crate::template::Templates;
use sitemap_rs::url::Url;
use sitemap_rs::url_set::UrlSet;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

fn path_to_rel_path(root: PathBuf, path: PathBuf) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(stripped_path) => stripped_path.to_path_buf(),
        Err(err) => {
            log::warn!("{}", err);
            panic!();
        }
    }
}

fn filter_file(file: &PathBuf) -> bool {
    let filename = file.file_name().unwrap().to_str().unwrap();

    let is_backup = filename.ends_with("~");
    let is_buffer = filename.ends_with("#") && filename.starts_with("#");

    file.is_file()
        && !is_buffer
        && !is_backup
        && !file
            .components()
            .any(|s| AsRef::<OsStr>::as_ref(&s).to_str() == Some(".git"))
}

pub struct FileDispatcher {
    pub templates: Templates,
    handlers: HashMap<String, Box<dyn FileHandler>>,
    config: Config,
}

impl FileDispatcher {
    pub fn new(data_dir: &str, config: Config) -> Self {
        let mut a = Self {
            templates: Templates::new(Path::new(data_dir)),
            handlers: HashMap::new(),
            config,
        };

        a.register_handlers();

        a
    }

    fn register_handlers(&mut self) {
        self.register_handler::<OrgHandler>("org");
        self.register_handler::<CopyHandler>("_default");
    }

    fn register_handler<H: FileHandler + 'static>(&mut self, extension: &str) {
        self.handlers.insert(
            extension.to_owned(),
            Box::new(H::new()),
        );
    }

    fn handle<T, F: FnOnce(&mut Box<dyn FileHandler>, &FileContext) -> anyhow::Result<T>>(
        &mut self,
        ctx: &FileContext,
        f: F,
    ) -> anyhow::Result<T> {
        let mut handler = self
            .handlers
            .get_mut(&ctx.ext)
            .cloned()
            .unwrap_or_else(|| self.handlers.get_mut("_default").unwrap().clone());

        f(&mut handler, ctx)
    }

    fn create_context(
        &mut self,
        data_dir: PathBuf,
        root: PathBuf,
        rel_file: PathBuf,
    ) -> FileContext {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file.clone()]);

        FileContext::new(&self.config, &rel_file, &file, &new_file, &self.templates)
    }

    pub fn handle_files(&mut self, data_dir: String, dir: String) {
        let root_path = Path::new(&dir).canonicalize().unwrap();
        let data_path = Path::new(&data_dir).canonicalize().unwrap();

        let files: Vec<FileContext> = walkdir::WalkDir::new(dir.clone())
            .into_iter()
            .map(|file| file.as_ref().unwrap().path().canonicalize().unwrap())
            .filter(filter_file)
            .map(|file| {
                self.create_context(
                    data_path.clone(),
                    root_path.clone(),
                    path_to_rel_path(root_path.clone(), file.clone()),
                )
            })
            .collect();

        let urls: Vec<Url> = files
            .iter()
            .map(|ctx| self.handle(ctx, |handler, ctx| handler.extract_metadata(ctx.clone())))
            .filter_map(|res| res.ok())
            .filter_map(|meta| match meta {
                Metadata::Article { modified, url, .. } => {
                    let mut builder = Url::builder(url);
                    builder.last_modified(modified.into());
                    builder.build().ok()
                }
                Metadata::Image { .. } => None,
            })
            .collect();

        files.iter().for_each(|ctx| {
            self.handle(ctx, |handler, ctx| handler.handle_file(ctx.clone()))
                .unwrap()
        });

        if urls.len() > 0 {
            let sitemap_path = format!("{}/sitemap.xml", data_path.clone().display());
            log::info!("Generating `{}`", sitemap_path);
            let sitemap_file =
                std::fs::File::create(sitemap_path).expect("Unable to write sitemap.xml");
            let url_set = UrlSet::new(urls.clone()).expect("failed a <urlset> validation");
            url_set.write(sitemap_file).unwrap();
        }
    }
}
