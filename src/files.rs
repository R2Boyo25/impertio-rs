use rayon::prelude::*;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
//use std::process::Command;
use crate::template::Templates;

fn path_to_rel_path(root: PathBuf, path: PathBuf) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(stripped_path) => stripped_path.to_path_buf(),
        Err(err) => {
            log::warn!("{}", err);
            panic!();
        }
    }
}

/*fn file_type(file: PathBuf) {
    match String::from_utf8_lossy(Command::new("ls")
        .arg(file)
        .arg("--mime-type")
        .output().unwrap().stdout).as_str() {
            "image/png" => log::info!("image"),
            "text/x-org" => log::info!("org-file")
        }
}*/

fn filter_file(file: &PathBuf) -> bool {
    let filename = file.file_name().unwrap().to_str().unwrap();

    let is_backup = filename.ends_with("~");
    let is_buffer = filename.ends_with("#") && filename.starts_with("#");

    file.is_file() && !is_buffer && !is_backup
}

pub struct FileHandler {
    templates: Templates,
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
        }
    }

    fn handle_file(&mut self, data_dir: PathBuf, root: PathBuf, rel_file: PathBuf) {
        let file: PathBuf = PathBuf::from_iter(vec![root.clone(), rel_file.clone()]);
        let mut new_file: PathBuf = PathBuf::from_iter(vec![data_dir, rel_file]);

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

                if !file_changed(&file, &new_file).unwrap()
                    && !file_changed(&file, &source_file).unwrap()
                {
                    return;
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
                    )
                    .unwrap();

                log::info!("{}: {}", file.to_str().unwrap(), out);

                writeable(&new_file)
                    .unwrap()
                    .write_all(out.as_bytes())
                    .unwrap();
                writeable(&source_file)
                    .unwrap()
                    .write_all(std::fs::read(file).unwrap().as_slice())
                    .unwrap();
            }
            "html" => (),
            _ => {
                if !file_changed(&file, &new_file).unwrap() {
                    return;
                }

                log::warn!("File {:?} not recognized. Copying as-is...", file);

                writeable(&new_file)
                    .unwrap()
                    .write_all(std::fs::read(file).unwrap().as_slice())
                    .unwrap();
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
            )
        }
    }
}
