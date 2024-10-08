use chrono::offset::Local;
use egui::TextBuffer;
use ftp::FtpStream;
use pathdiff;
use std::{
    error::Error,
    f64, fs,
    io::{self, Read, Write},
    path::Path,
    path::PathBuf,
    result::Result,
    str::from_utf8,
    time::UNIX_EPOCH,
};
use zip::{read::ZipFile, write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SaveData {
    time: f64,
}

fn get_filenames(directory: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    println!("Fetching filenames");
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
    println!("Fetching modification time");
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
    srcpath: &PathBuf,
    destination: &mut PathBuf,
) -> Result<(), Box<dyn Error>> {
    println!("Creating zip archive for {}", name);
    destination.push(name);
    println!(
        "{}",
        &destination.clone().into_os_string().into_string().unwrap()
    );
    let zip_path = fs::File::create(&destination)?;
    println!("Created empty file");
    let mut zip_file = ZipWriter::new(zip_path);
    println!("Started zip");
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
    destination.pop();
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
    let mut tmp = home::home_dir().unwrap();
    tmp.push(".rc"); // TODO: link this to constant
    tmp.push("tmp");
    if !tmp.exists() {
        fs::create_dir(&tmp)?;
    }
    let dirpath = Path::new(&directory).to_path_buf();
    if !dirpath.exists() {
        return Ok(());
    }
    // TODO: fixxxxxxxxxxxxxxxxxxxx
    let filenames = get_filenames(&dirpath)?;
    let max_mod_time = get_max_mod_time(&filenames)?;
    let data = SaveData { time: max_mod_time };
    let j = serde_json::to_string(&data)?;
    tmp.push(savename.to_string() + "-" + &Local::now().date_naive().to_string() + ".json");
    fs::write(&tmp, &j).expect("Unable to write file");
    tmp.pop();
    println!("Logging in to FTP server");
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
    let save_filename: String = savename.to_owned() + "-" + &Local::now().date_naive().to_string();

    if json_f == "".to_string() {
        println!("Previous save not found, uploading save");
        create_zip_archive(&(save_filename.clone() + ".zip"), &dirpath, &mut tmp)?;
        println!("Zip archive created");
        tmp.push(&(save_filename.clone() + ".zip"));
        let mut zip_file = fs::File::open(&tmp)?;
        tmp.pop();
        tmp.push(&(save_filename.clone() + ".json"));
        let json_file_data = serde_json::to_string(&data)?;
        fs::write(&tmp, &json_file_data)?;
        println!("Created json file.");
        let mut json_file = fs::File::open(&tmp)?;
        ftp_stream.put(&(save_filename.clone() + ".json"), &mut json_file)?;
        ftp_stream.put(&(save_filename.clone() + ".zip"), &mut zip_file)?;
    } else {
        println!("Checking date of previous save");
        let cursor = ftp_stream.simple_retr(&json_f)?;
        let vec = cursor.into_inner();
        let file = from_utf8(&vec)?;
        let server_data: SaveData = serde_json::from_str(&file)?;
        if server_data.time > data.time {
            println!("Downloading previous save");
            tmp.push(&(save_filename.clone() + ".zip"));
            let mut zip_file = fs::File::create(&tmp)?;
            let cursor = ftp_stream.simple_retr(&(save_filename.clone() + ".zip"))?;
            let vec = cursor.into_inner();
            zip_file.write(&vec)?;
            let mut zip_archive = ZipArchive::new(&zip_file)?;
            zip_archive.extract(dirpath)?;
        } else if server_data.time == data.time {
            println!("Already up to date.")
            // Do nothing
        } else {
            println!("Uploading local save to cloud");
            for item in ftp_stream.nlst(None)? {
                let _ = ftp_stream.rm(&item);
            }
            let save_filename: String =
                savename.to_owned() + "-" + &Local::now().date_naive().to_string();
            create_zip_archive(&(save_filename.clone() + ".zip"), &dirpath, &mut tmp)?;
            tmp.push(&(save_filename.clone() + ".zip"));
            let mut zip_file = fs::File::open(&tmp)?;
            tmp.pop();
            tmp.push(&(save_filename.clone() + ".json"));
            let json_file_data = serde_json::to_string(&data)?;
            fs::write(&tmp, &json_file_data)?;
            let mut json_file = fs::File::open(&tmp)?;
            ftp_stream.put(&(save_filename.clone() + ".json"), &mut json_file)?;
            ftp_stream.put(&(save_filename.clone() + ".zip"), &mut zip_file)?;
        }
    }
    ftp_stream.quit()?;
    Ok(())
}
