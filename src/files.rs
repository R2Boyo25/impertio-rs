// SPDX-FileCopyrightText: 2024 Ohin "Kazani" Taylor <kazani@kazani.dev>
// SPDX-License-Identifier: MIT

use crate::config::Config;
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
        self.handlers
            .insert(extension.to_owned(), Box::new(H::new()));
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
        metadata: Arc<Mutex<Vec<Metadata>>>
    ) -> FileContext {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file.clone()]);

        FileContext::new(&self.config, &rel_file, &file, &new_file, &self.templates, metadata)
    }

    pub fn handle_files(&mut self, data_dir: String, dir: String) -> anyhow::Result<()> {
        let root_path = Path::new(&dir).canonicalize().unwrap();
        let data_path = Path::new(&data_dir).canonicalize().unwrap();
        let metadata_vec: Arc<Mutex<Vec<Metadata>>> = Arc::new(Mutex::new(vec![]));

        let files: Vec<FileContext> = walkdir::WalkDir::new(dir.clone())
            .into_iter()
            .map(|file| file.as_ref().unwrap().path().canonicalize().unwrap())
            .filter(filter_file)
            .map(|file| {
                self.create_context(
                    data_path.clone(),
                    root_path.clone(),
                    path_to_rel_path(root_path.clone(), file.clone()),
                    metadata_vec.clone()
                )
            })
            .collect();

        let metadata: Vec<Metadata> = files
            .iter()
            .map(|ctx| self.handle(ctx, |handler, ctx| handler.extract_metadata(ctx.clone())))
            .filter_map(|res| res.ok())
            .collect();

        metadata_vec.lock().unwrap().extend(metadata.clone());

        let urls: Vec<Url> = metadata
            .iter()
            .filter_map(|meta| match meta {
                Metadata::Article { modified, url, .. } => {
                    let mut builder = Url::builder(url.to_string());
                    builder.last_modified((*modified).into());
                    builder.build().ok()
                }
                _ => None,
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
            url_set.write(sitemap_file)?;
        }

        if let Some(rss_config) = self.config.rss.clone() {
            let rss_builder = rss::Channel {
                title: rss_config.title,
                link: rss_config.link,
                description: rss_config.description,
                language: rss_config.language,
                copyright: rss_config.copyright,
                managing_editor: rss_config.managing_editor,
                webmaster: rss_config.webmaster,
                pub_date: None,
                last_build_date: None,
                categories: rss_config
                    .categories
                    .unwrap_or_else(|| vec![])
                    .iter()
                    .map(|category| rss::Category {
                        name: category.name.clone(),
                        domain: category.domain.clone(),
                    })
                    .collect(),
                generator: Some(format!(
                    "Impertio {} ({}), RSS Crate (https://crates.io/crates/rss)",
                    env!("CARGO_PKG_VERSION"),
                    env!("CARGO_PKG_HOMEPAGE")
                )),
                docs: Some("https://www.rssboard.org/rss-specification".to_owned()),
                cloud: None,
                ttl: rss_config.ttl.or(Some(60)).map(|ttl| ttl.to_string()),
                image: rss_config.image.map(|img| rss::Image {
                    url: img.url,
                    title: img.title,
                    link: img.link,
                    width: img.width,
                    height: img.height,
                    description: img.description,
                }),
                rating: rss_config.rating,
                text_input: rss_config.text_input.map(|ti| rss::TextInput {
                    title: ti.title,
                    description: ti.description,
                    name: ti.name,
                    link: ti.link,
                }),
                skip_hours: rss_config.skip_hours.unwrap_or_else(|| vec![]),
                skip_days: rss_config.skip_days.unwrap_or_else(|| vec![]),
                extensions: Default::default(),
                itunes_ext: None,
                dublin_core_ext: None,
                syndication_ext: None,
                namespaces: Default::default(),
                items: metadata
                    .iter()
                    .filter_map(|meta| match meta {
                        Metadata::Article {
                            title,
                            description,
                            modified,
                            url,
                            author,
                            tags,
                        } => Some(rss::Item {
                            title: Some(title.to_string()),
                            link: Some(url.to_string()),
                            guid: Some(rss::Guid {
                                value: url.to_string(),
                                permalink: true,
                            }),
                            description: description.to_owned(),
                            author: author.to_owned(),
                            categories: tags
                                .to_owned()
                                .iter()
                                .map(|tag| rss::Category {
                                    name: tag.to_string(),
                                    domain: None,
                                })
                                .collect(),
                            comments: None,
                            enclosure: None,
                            pub_date: Some(modified.to_rfc2822()),
                            source: None,
                            content: None,
                            extensions: Default::default(),
                            itunes_ext: None,
                            dublin_core_ext: None,
                        }),
                        _ => None,
                    })
                    .collect(),
            };
            
            let rss_path = format!("{}/feed", data_path.clone().display());
            log::info!("Generating `{}` (RSS)", rss_path);

            let rss_file = std::fs::File::create(rss_path).expect("Unable to write RSS feed");

            rss_builder.pretty_write_to(rss_file, b'\t', 1)?;
        }

        Ok(())
    }
}
