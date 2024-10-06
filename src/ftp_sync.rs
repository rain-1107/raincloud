use chrono::offset::Local;
use egui::TextBuffer;
use ftp::FtpStream;
use pathdiff;
use std::{
    error::Error,
    f64, fs,
    io::{self, Read, Write},
    path::Path,
    result::Result,
    str::from_utf8,
    time::UNIX_EPOCH,
};
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveData {
    time: f64,
}

fn get_filenames(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut filenames = Vec::new();
    let paths = fs::read_dir(&directory)?;
    for path_result in paths {
        let path = path_result?.path();
        if path.is_dir() {
            filenames.extend(get_filenames(&path)?);
        } else if path.is_file() {
            filenames.push(path.display().to_string());
        }
    }
    Ok(filenames)
}

fn get_max_mod_time(filenames: &Vec<String>) -> Result<f64, Box<dyn Error>> {
    let mut max = 0.0;
    for p in filenames {
        let path = Path::new(&p);
        let file = fs::File::open(&path)?;
        let file_max = file
            .metadata()?
            .modified()?
            .duration_since(UNIX_EPOCH)?
            .as_secs_f64();
        if file_max > max {
            max = file_max;
        }
    }
    Ok(max)
}

fn create_zip_archive(
    name: &String,
    source: &String,
    destination: &String,
) -> Result<(), Box<dyn Error>> {
    let mut srcpath = std::path::PathBuf::new();
    srcpath.push(source);
    let mut tmp = std::path::PathBuf::new();
    tmp.push(destination);
    tmp.push(name);
    let zip_path = fs::File::create(&tmp)?;
    let mut zip_file = ZipWriter::new(zip_path);
    let options: zip::write::FileOptions<zip::write::ExtendedFileOptions> =
        FileOptions::default().compression_method(CompressionMethod::DEFLATE);
    let filenames = get_filenames(&srcpath)?;
    for p in &filenames {
        let path = Path::new(&p);
        let local_path = pathdiff::diff_paths(&path, &srcpath).unwrap();
        zip_file.start_file(
            local_path.into_os_string().into_string().unwrap(),
            options.clone(),
        )?;
        let mut buffer = Vec::new();
        let file = fs::File::open(&path)?;
        io::copy(&mut file.take(u64::MAX), &mut buffer)?;
        zip_file.write_all(&buffer)?;
    }
    zip_file.finish()?;
    Ok(())
}

pub fn sync_save(
    savename: &String,
    directory: &String,
    address: &String,
    username: &String,
    password: &String,
    port: u16,
) -> Result<(), Box<dyn Error>> {
    println!("Creating zip archive for {}", savename);
    let mut tmp = home::home_dir().unwrap();
    tmp.push(".rc"); // TODO: link this to constant
    tmp.push("tmp");
    if !tmp.exists() {
        fs::create_dir(&tmp)?;
    }
    let dirpath = Path::new(&directory);
    if !dirpath.exists() {
        return Ok(());
    }
    // TODO: fixxxxxxxxxxxxxxxxxxxx
    let filenames = get_filenames(dirpath)?;
    let max_mod_time = get_max_mod_time(&filenames)?;
    let data = SaveData { time: max_mod_time };
    let j = serde_json::to_string(&data)?;
    tmp.push(savename.to_string() + "-" + &Local::now().date_naive().to_string() + ".json");
    fs::write(&tmp, &j).expect("Unable to write file");
    let mut ftp_stream = FtpStream::connect(address.to_string() + ":" + &port.to_string())
        .unwrap_or_else(|err| panic!("{}", err));
    ftp_stream.login(username, password)?;
    if !ftp_stream
        .nlst(None)?
        .contains(&"raincloud-saves".to_string())
    {
        ftp_stream.mkdir("raincloud-saves")?;
    }
    ftp_stream.cwd("raincloud-saves")?;
    if !ftp_stream.nlst(None)?.contains(savename) {
        println!("Making test folder");
        ftp_stream.mkdir(savename)?;
    }
    ftp_stream.cwd(savename)?;
    let list = ftp_stream.nlst(None)?;
    let mut json_f = "".to_string();
    for f in &list {
        if f.ends_with(".json") {
            json_f = f.to_string();
            break;
        }
    }
    if json_f == "".to_string() {
        let zip_name: String =
            savename.to_owned() + "-" + &Local::now().date_naive().to_string() + ".zip";
        create_zip_archive(
            &zip_name,
            &directory.to_string(),
            &tmp.clone().into_os_string().into_string().unwrap(),
        )?;
        tmp.push(&zip_name);
        let mut zip_file = fs::File::open(&tmp)?;
        ftp_stream.put(&zip_name, &mut zip_file)?;
    } else {
        let cursor = ftp_stream.simple_retr(&json_f)?;
        let vec = cursor.into_inner();
        let file = from_utf8(&vec)?;
        let data: SaveData = serde_json::from_str(&file)?;
        println!("{}", data.time);
        // TODO: Finish logic for deciding to upload or download to sync
    }
    let _ = ftp_stream.quit();
    Ok(())
}
