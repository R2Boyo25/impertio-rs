use crate::handler::{CopyHandler, FileContext, FileHandler, OrgHandler};
use crate::metadata::Metadata;
use crate::template::Templates;
use sitemap_rs::url::Url;
use sitemap_rs::url_set::UrlSet;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    pub rc: Option<Arc<Mutex<Self>>>,
}

impl FileDispatcher {
    pub fn new(data_dir: &str) -> Self {
        Self {
            templates: Templates::new(Path::new(data_dir)),
            handlers: HashMap::new(),
            rc: None,
        }
    }

    pub fn register_handlers(&mut self) {
        self.register_handler::<OrgHandler>("org");
        self.register_handler::<CopyHandler>("_default");
    }

    fn register_handler<H: FileHandler + 'static>(&mut self, extension: &str) {
        self.handlers.insert(
            extension.to_owned(),
            Box::new(H::new(self.rc.clone().unwrap())),
        );
    }

    fn handle_file(
        &mut self,
        data_dir: PathBuf,
        root: PathBuf,
        rel_file: PathBuf,
    ) -> anyhow::Result<()> {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file.clone()]);

        let ext = file
            .extension()
            .unwrap_or(&OsStr::new(""))
            .to_str()
            .unwrap_or("");

        let ctx = FileContext::new(&rel_file, &file, &new_file, &self.templates);

        if let Some(handler) = self.handlers.get_mut(ext) {
            handler.handle_file(ctx)
        } else {
            let handler = self.handlers.get_mut("_default").unwrap();
            handler.handle_file(ctx)
        }
    }

    fn extract_metadata(
        &mut self,
        data_dir: PathBuf,
        root: PathBuf,
        rel_file: PathBuf,
    ) -> anyhow::Result<Metadata> {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file.clone()]);

        let ext = file
            .extension()
            .unwrap_or(&OsStr::new(""))
            .to_str()
            .unwrap_or("");

        let ctx = FileContext::new(&rel_file, &file, &new_file, &self.templates);

        if let Some(handler) = self.handlers.get_mut(ext) {
            handler.extract_metadata(ctx)
        } else {
            let handler = self.handlers.get_mut("_default").unwrap();
            handler.extract_metadata(ctx)
        }
    }

    pub fn handle_files(&mut self, data_dir: String, dir: String) {
        let root_path = Path::new(&dir).canonicalize().unwrap();
        let data_path = Path::new(&data_dir).canonicalize().unwrap();

        let files: Vec<PathBuf> = walkdir::WalkDir::new(dir.clone())
            .into_iter()
            .map(|file| file.as_ref().unwrap().path().canonicalize().unwrap())
            .filter(filter_file)
            .collect();

        let urls: Vec<Url> = files
            .iter()
            .map(|file| {
                self.extract_metadata(
                    data_path.clone(),
                    root_path.clone(),
                    path_to_rel_path(root_path.clone(), file.clone()),
                )
            })
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

        files.iter().for_each(|file| {
            self.handle_file(
                data_path.clone(),
                root_path.clone(),
                path_to_rel_path(root_path.clone(), file.clone()),
            )
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
