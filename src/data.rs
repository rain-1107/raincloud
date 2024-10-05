use serde_json::Result;
use std::fs;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveUI {
    pub name: String,
    pub path: String,
}

const CONFIG_DIR: &str = ".rc";

pub fn check_config_folder() {
    let mut home = home::home_dir().unwrap();
    home.push(CONFIG_DIR);
    if !home.exists() {
        let _ = fs::create_dir(&home);
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
) -> Result<()> {
    let mut home = home::home_dir().unwrap();
    home.push(CONFIG_DIR);
    home.push("config.json");
    let json_data = Json {
        server: server,
        ftp_config: ftp_details.clone(),
        saves: saves.to_vec(),
    };
    let j = serde_json::to_string(&json_data)?;
    let path = &home;
    fs::write(path, &j).expect("Unable to write file");
    Ok(())
}

pub fn load_config_data() -> Json {
    let mut home = home::home_dir().unwrap();
    home.push(CONFIG_DIR);
    home.push("config.json");
    let file_result = fs::read(&home);
    let file_slice = match file_result {
        Ok(file) => file,
        Err(_error) => return Json::default(),
    };
    serde_json::from_slice(&file_slice).unwrap()
}
