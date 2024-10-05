use core::time;
use serde_json::Result;
use std::{f64, fs, path::Path, thread::Thread};
use zip::ZipWriter;

struct FileData {
    filename: String,
    time: f64,
}

pub struct SaveData {
    files: Vec<FileData>,
}

fn get_filenames(directory: &Path) -> Vec<String> {
    let mut filenames = Vec::new();
    let paths = fs::read_dir(&directory).unwrap();
    for path_result in paths {
        let path = path_result.unwrap().path();
        if path.is_dir() {
            filenames.extend(get_filenames(&path));
        }
        if path.is_file() {
            filenames.push(path.display().to_string());
        }
    }
    filenames
}

pub fn sync_save(savename: String, directory: String) {
    let mut tmp = home::home_dir().unwrap();
    tmp.push(".rc"); // TODO: link this to constant
    tmp.push("tmp");
    if !tmp.exists() {
        fs::create_dir(&tmp);
    }
    let dirpath = Path::new(&directory);
    if !dirpath.exists() {
        return;
    }
    let zip_name = savename.to_string();
    tmp.push(zip_name + ".zip");
    let zip_path = fs::File::create(&tmp).unwrap();
    let zip_file = ZipWriter::new(zip_path);
    for p in get_filenames(&dirpath) {
        let path = Path::new(&p);
        let data = fs::read(&path).unwrap();
        // TODO: finish zipping
    }
    zip_file.finish().unwrap();
}
