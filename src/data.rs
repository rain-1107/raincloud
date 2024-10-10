use std::{error::Error, fs, result::Result};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveUI {
    pub name: String,
    pub path: String,
}

const CONFIG_DIR: &str = ".rc";

pub fn purge_tmp_folder() -> Result<(), Box<dyn Error>> {
    let mut path = home::home_dir().unwrap();
    path.push(CONFIG_DIR);
    path.push("tmp");
    fs::remove_dir_all(&path)?;
    fs::create_dir(&path)?;
    Ok(())
}
pub fn check_config_folder() {
    let mut path = home::home_dir().unwrap();
    path.push(CONFIG_DIR);
    if !path.exists() {
        let _ = fs::create_dir(&path);
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Json {
    pub server: String,
    pub ftp_config: FtpDetails,
    pub saves: Vec<SaveUI>,
}

impl Default for Json {
    fn default() -> Self {
        Self {
            server: "ftp".to_string(),
            ftp_config: FtpDetails {
                ip: "".to_owned(),
                user: "".to_owned(),
                passwd: "".to_owned(),
                port: 21,
            },
            saves: Vec::new(),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct FtpDetails {
    pub ip: String,
    pub user: String,
    pub passwd: String,
    pub port: u16,
}

pub fn save_config_data(
    server: String,
    ftp_details: &FtpDetails,
    saves: &Vec<SaveUI>,
) -> Result<(), Box<dyn Error>> {
    let mut path = home::home_dir().unwrap();
    path.push(CONFIG_DIR);
    path.push("config.json");
    let json_data = Json {
        server,
        ftp_config: ftp_details.clone(),
        saves: saves.to_vec(),
    };
    let j = serde_json::to_string(&json_data)?;
    fs::write(&path, &j).expect("Unable to write file");
    Ok(())
}

pub fn load_config_data() -> Json {
    let mut path = home::home_dir().unwrap();
    path.push(CONFIG_DIR);
    path.push("config.json");
    let file_result = fs::read(&path);
    let file_slice = match file_result {
        Ok(file) => file,
        Err(_error) => return Json::default(),
    };
    serde_json::from_slice(&file_slice).unwrap()
}
