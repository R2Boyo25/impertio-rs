use std::path::Path;

use crate::files::FileDispatcher;

pub trait FileHandler {
    fn new(dispatcher: FileDispatcher);
    fn handle_file(file: &Path);
}

pub struct OrgHandler {
    dispatcher: Arc<FileDispatcher>
}

impl FileHandler for OrgHandler {
    fn new(dispatcher: Arc<FileDispatcher>) {

    }

    fn handle_file(file: &Path) {
        
    }
}