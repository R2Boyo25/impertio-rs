use crate::template::Templates;
use sitemap_rs::url::Url;
use sitemap_rs::url_set::UrlSet;
use std::ffi::OsStr;
use std::io::Write;
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

pub struct FileHandler {
    templates: Templates,
    pages: Vec<Url>,
}

fn file_changed(old: &Path, new: &Path) -> std::io::Result<bool> {
    Ok(!new.exists() || new.metadata()?.modified()? < old.metadata()?.modified()?)
}

fn writeable(path: &Path) -> std::io::Result<std::fs::File> {
    use std::fs::{create_dir_all, File};

    create_dir_all(path.parent().unwrap())?;
    File::create(path)
}

impl FileHandler {
    pub fn new(data_dir: &str) -> Self {
        Self {
            templates: Templates::new(Path::new(data_dir)),
            pages: vec![],
        }
    }

    fn handle_file(&mut self, data_dir: PathBuf, root: PathBuf, rel_file: PathBuf) -> anyhow::Result<()> {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let mut new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file.clone()]);

        match file
            .extension()
            .unwrap_or(&OsStr::new(""))
            .to_str()
            .unwrap_or("")
        {
            "org" => {
                new_file.set_extension("html");

                let mut source_file: PathBuf = new_file.clone();
                source_file.set_extension("org");

                if !file_changed(&file, &new_file)?
                    && !file_changed(&file, &source_file)?
                {
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

                let parsed = crate::org::Document::parse_file(file.to_str().unwrap()).unwrap();
                let out = self
                    .templates
                    .render(
                        "root.html",
                        file.as_path(),
                        &parsed.to_html(),
                        Some(
                            parsed
                                .metadata
                                .iter()
                                .map(|(key, value)| (key.as_str(), value.to_owned()))
                                .collect(),
                        ),
                    )?;

                log::debug!("{}: {}", file.to_str().unwrap(), out);

                writeable(&new_file)?
                    .write_all(out.as_bytes())?;
                writeable(&source_file)?
                    .write_all(std::fs::read(file.clone())?.as_slice())?;

                match std::env::var("IMPERTIO_SITE_URL") {
                    Ok(url) => {
                        let mut url_path = rel_file.clone();
                        url_path.set_extension("html");

                        let mut builder = Url::builder(format!("{}/{}", url, url_path.display()));

                        if let Ok(modtime) = std::fs::metadata(file)?.modified() {
                            builder.last_modified(
                                chrono::DateTime::<chrono::offset::Local>::from(modtime).into(),
                            );
                        }

                        self.pages
                            .push(builder.build().expect("failed a <url> validation"));

                        Ok(())
                    }
                    _ => Ok(())
                }
            },
            _ => {
                if !file_changed(&file, &new_file)? {
                    return Ok(());
                }

                log::warn!("File {:?} not recognized. Copying as-is...", file);

                writeable(&new_file)?
                    .write_all(std::fs::read(file)?.as_slice())?;

                Ok(())
            }
        }
    }

    pub fn handle_files(&mut self, data_dir: String, dir: String) {
        let root_path = Path::new(&dir).canonicalize().unwrap();
        let data_path = Path::new(&data_dir).canonicalize().unwrap();

        for file in walkdir::WalkDir::new(dir.clone())
            .into_iter()
            .map(|file| file.as_ref().unwrap().path().canonicalize().unwrap())
            .filter(filter_file)
        {
            self.handle_file(
                data_path.clone(),
                root_path.clone(),
                path_to_rel_path(root_path.clone(), file),
            ).unwrap()
        }

        if self.pages.len() > 0 {
            let sitemap_path = format!("{}/sitemap.xml", data_path.clone().display());
            log::info!("Generating `{}`", sitemap_path);
            let sitemap_file = std::fs::File::create(sitemap_path)
                .expect("Unable to write sitemap.xml");
            let url_set = UrlSet::new(self.pages.clone()).expect("failed a <urlset> validation");
            url_set.write(sitemap_file).unwrap();
        }
    }
}
