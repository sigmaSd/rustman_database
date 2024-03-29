use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::{Arc, Mutex};
use unchained::Unchained;

const CRATES_URL_TEMPLATE: &str = "https://crates.io/api/v1/crates?per_page=100&page=";
const MAX_NET_TRY: usize = 10;

#[derive(Default)]
pub struct Database {
    crates: Arc<Mutex<Vec<Crate>>>,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Crates {
    crates: Vec<Crate>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct Crate {
    pub name: String,
    pub version: String,
    pub description: String,
}

impl Crate {
    fn contains(&self, needles: &[String]) -> bool {
        needles
            .iter()
            .all(|needle| self.name.contains(needle) || self.description.contains(needle))
    }
}

impl Database {
    pub fn read_black_list() -> Vec<String> {
        let blacklist_path = "blacklist";
        let blacklist = match std::fs::read_to_string(blacklist_path) {
            Ok(bl) => bl,
            Err(_) => String::new(),
        };

        blacklist.lines().map(ToOwned::to_owned).collect()
    }

    pub fn add_to_blaklist(mut blacklist: Vec<String>, s: &str) {
        if blacklist.contains(&s.to_string()) {
            return;
        }

        blacklist.push(s.to_string());

        let blacklist_path = "blacklist";
        let mut blacklist_file = std::fs::File::create(blacklist_path).unwrap();

        writeln!(
            blacklist_file,
            "{}",
            blacklist
                .iter()
                .map(|p| {
                    let mut p = p.to_string();
                    p.push('\n');
                    p
                })
                .collect::<String>()
        )
        .unwrap();
    }
    pub fn update(&mut self) {
        self.crates.lock().unwrap().clear();

        let crates = self.crates.clone();

        (1..300).unchained_for_each(move |page_idx| {
            let mut crates_url = CRATES_URL_TEMPLATE.to_string();
            crates_url.push_str(&page_idx.to_string());

            let crates_url: http_req::uri::Uri = crates_url.parse().unwrap();
            let mut crate_metadata = Vec::new();

            let mut send_request = || {
                http_req::request::Request::new(&crates_url)
                    .header("User-Agent", "https://github.com/sigmaSd/rustman")
                    .send(&mut crate_metadata)
            };

            let mut counter = 0;
            while let Err(_) = send_request() {
                counter += 1;
                if counter == MAX_NET_TRY {
                    panic!("Network error");
                }
            }

            let crates_json = String::from_utf8(crate_metadata).unwrap();
            let crates_json = json::parse(&crates_json).unwrap();

            let crates = crates.clone();
            (0..100).unchained_for_each(move |i| {
                let name = crates_json["crates"][i]["name"].to_string().to_lowercase();
                let version = crates_json["crates"][i]["max_version"].to_string();
                let description = crates_json["crates"][i]["description"]
                    .to_string()
                    .to_lowercase();

                if version == "null" {
                    return;
                }

                let mut crates = loop {
                    if let Ok(crates) = crates.try_lock() {
                        break crates;
                    }
                };

                crates.push(Crate {
                    name,
                    version,
                    description,
                });
            });
        });
    }

    pub fn save(&self) {
        let database_path = "database.toml";
        let mut database_file = std::fs::File::create(database_path).unwrap();

        let database = self.crates.lock().unwrap().clone();
        let database = Crates { crates: database };
        let database_toml = toml::to_string(&database).unwrap();

        writeln!(database_file, "{}", database_toml).unwrap();
    }

    pub fn search(&self, needles: &[String]) -> Vec<Crate> {
        let needles: Vec<String> = needles.iter().map(|s| s.to_lowercase()).collect();

        self.crates
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.contains(&needles))
            .cloned()
            .collect()
    }
}

#[test]
fn database_check() {
    let mut database = Database::default();
    database.update();
    database.save();
}
