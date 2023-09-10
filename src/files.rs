use rayon::prelude::*;
use std::path::{PathBuf, Path};
use std::ffi::OsStr;
//use std::process::Command;

fn path_to_rel_path(root: PathBuf, path: PathBuf) -> PathBuf {
    match path.strip_prefix(root) {
        Ok(stripped_path) => stripped_path.to_path_buf(),
        Err(err) => {
            log::warn!("{}", err);
            panic!();
        }
    }
}

enum FileType {
    Org,
    HTML,
    Image,
    Other
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

fn handle_file(root: PathBuf, rel_file: PathBuf) {
    let mut file: PathBuf = PathBuf::from(root);
    file.push(rel_file);
    
    match file.extension().unwrap_or(&OsStr::new("")).to_str().unwrap_or("") {
        "org" => {
            match file.file_stem().unwrap_or(file.as_os_str()).to_str().unwrap_or(file.to_str().unwrap()) {
                "index" => log::info!("Parsing index of {:?}", file.parent().unwrap_or(&Path::new("<root>"))),
                _ => log::info!("Parsing Org file {:?}", file)
            }
        },
        "html" => log::info!("Found template file {:?}", file),
        _ => log::warn!("File {:?} not recognized.", file)
    }
}

fn filter_file(file: &PathBuf) -> bool {
    let filename = file.file_name().unwrap().to_str().unwrap();
    
    let is_backup = filename.ends_with("~");
    let is_buffer = filename.ends_with("#") && filename.starts_with("#");
        
    file.is_file() && !is_buffer && !is_backup
}

pub fn get_files(dir: String) {
    let root_path = Path::new(&dir).canonicalize().unwrap();
    
    walkdir::WalkDir::new(dir.clone())
        .into_iter()
        .collect::<Vec<_>>()
        .par_iter()
        .map(|file| file.as_ref().unwrap().path().canonicalize().unwrap())
        .filter(filter_file)
        .map(|file| handle_file(root_path.clone(), path_to_rel_path(root_path.clone(), file)))
        .collect::<Vec<_>>();
}
