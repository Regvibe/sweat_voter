use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, web::ServiceConfig, App, HttpServer, Responder};
use actix_web::http::{KeepAlive};
use actix_web::middleware::Logger;
use tracing_subscriber::EnvFilter;
use common::{Group, Nickname};
use common::packets::c2s::{AddNickname, AskForPersonProfile, DeleteNickname, RequestKind, VoteNickname};
use common::packets::s2c::{ClassList, PersonProfileResponse, VoteCount};

extern crate tracing;

type State = Arc<AppState>;

struct Class {
    path: PathBuf,
    participants: Group,
}

impl Class {
    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let json = File::open(&path)?;
        let participants: Group = serde_json::from_reader(json)?;
        Ok(Self {
            path,
            participants,
        })
    }

    fn save(&self) {
        let file = File::create(&self.path).expect(format!("Failed to create {}", self.path.display()).as_str());
        serde_json::to_writer_pretty(file, &self.participants).expect(format!("Failed to write {}", self.path.display()).as_str());
    }
}

struct AppState {
    classes: HashMap<String, Mutex<Class>>, //class name -> Class
}

impl AppState {
    fn new() -> Self {
        println!("Creating new AppState");

        let mut groups = HashMap::new();

        let files = std::fs::read_dir("./classes").expect("Failed to read dir");
        for file in files.flatten() {
            let path = file.path();
            if path.is_file() && path.extension() == Some("json".as_ref()) {
                let name = path.file_stem().get_or_insert("unknown".as_ref()).to_string_lossy().to_string();
                println!("found: {} at {:?}", name, path);

                match Class::new(path) {
                    Ok(class) => {
                        groups.insert(name, Mutex::new(class));
                    }
                    Err(e) => println!("Failed to load class: {:?}", e),
                }
            }
        }

        AppState { classes: groups }
    }

    fn list_classes(&self) -> ClassList {
        let names = self.classes.keys().cloned().collect::<Vec<String>>();
        ClassList { names }
    }

    fn make_nickname_map(nickname_list: &Vec<Nickname>, editor_name: &str) -> BTreeMap<String, VoteCount> {
        let mut map = BTreeMap::new();
        for nickname in nickname_list {
            map.insert(nickname.nickname.clone(), VoteCount {
                count: nickname.votes.len(),
                contain_you: nickname.votes.iter().any(|v| *v == editor_name)
            });
        }
        return map;
    }

    fn convert_group(group: &Group, editor_name: &str) -> BTreeMap<String, BTreeMap<String, VoteCount>> {
        let mut map = BTreeMap::new();
        for (name, (_, nicknames)) in &group.profiles {
            map.insert(name.clone(), Self::make_nickname_map(nicknames, editor_name));
        }
        return map;
    }

    fn convert_group_custom(group: &Group, editor_name: &str, requested: &Vec<String>) -> BTreeMap<String, BTreeMap<String, VoteCount>> {
        let mut map = BTreeMap::new();
        for requested_name in requested {
            if let Some(( _,nicknames)) = group.profiles.get(requested_name) {
                map.insert(requested_name.clone(), Self::make_nickname_map(nicknames, editor_name));
            }
        }
        return map;
    }

    fn group_to_response(group: &Group, editor_name: &str, password: &str) -> PersonProfileResponse {
        let allowed_to_modify = group.profiles.get(editor_name).map_or(false, |(p, _)| p == password);
        let editor_name = if allowed_to_modify { editor_name } else { "" };
        PersonProfileResponse {
            partial_response: false,
            allowed_to_modify,
            profiles: Self::convert_group(group, editor_name),
        }
    }

    fn group_to_response_custom(group: &Group, editor_name: &str, password: &str, requested: &Vec<String>) -> PersonProfileResponse {
        let allowed_to_modify = group.profiles.get(editor_name).map_or(false, |(p, _)| p == password);
        let editor_name = if allowed_to_modify { editor_name } else { "" };
        PersonProfileResponse {
            partial_response: true,
            allowed_to_modify,
            profiles: Self::convert_group_custom(group, editor_name, requested),
        }
    }

    fn person_profiles(&self, asked: &AskForPersonProfile) -> PersonProfileResponse {
        println!("asked: {:?}", asked);

        match (self.classes.get(&asked.class), &asked.kind) {
            (Some(class), RequestKind::All) => {
                let lock = class.lock().expect("Failed to lock data");
                Self::group_to_response(&lock.participants, &asked.editor, &asked.password)
            },
            (Some(class), RequestKind::Custom(requested)) => {
                let lock = class.lock().expect("Failed to lock data");
                Self::group_to_response_custom(&lock.participants, &asked.editor, &asked.password, &requested)
            },
            (None, _) => {
                PersonProfileResponse {
                    partial_response: false,
                    allowed_to_modify: false,
                    profiles: BTreeMap::new(),
                }
            }
        }
    }

fn add_nickname(&self, add: &AddNickname) -> PersonProfileResponse {
        let AddNickname {
            class,
            editor,
            password,
            name,
            nickname
        } = add;
        println!("add_nickname: {} to {} by {} in class {}", nickname, name, editor, class);

        match self.classes.get(class) {
            None => PersonProfileResponse::default(),
            Some(class) => { //class exists
                //check if editor is allowed to modify
                let mut lock = class.lock().expect("Failed to lock data");
                let allowed_to_modify = lock.participants.profiles.get(editor).map_or(false, |(p, _)| p == password);
                if !allowed_to_modify {
                    return PersonProfileResponse::default();
                }

                let (_, nicknames) = lock.participants.profiles.get_mut(name).expect("Failed to find name");

                //check if nickname is not already present and add it
                let trim = nickname.trim();
                if !trim.is_empty() && nicknames.iter().find(|n| n.nickname == trim).is_none() { //add only if not already present
                    nicknames.push(Nickname {
                        nickname: nickname.trim().to_string(),
                        votes: Vec::new(),
                    });

                    lock.save();
                }

                Self::group_to_response_custom(&lock.participants, editor, password, &vec![name.clone()])
            }
        }
    }

    fn vote_nickname(&self, vote: &VoteNickname) -> PersonProfileResponse {
        let VoteNickname {
            class,
            name,
            nickname,
            voter,
            password,
        } = vote;
        println!("vote_nickname: name: {}, nickname: {}, voter: {}", name, nickname, voter);

        match self.classes.get(class) {
            None => PersonProfileResponse::default(),
            Some(class) => { //class exists
                //check if editor is allowed to modify
                let mut lock = class.lock().expect("Failed to lock data");
                let allowed_to_modify = lock.participants.profiles.get(voter).map_or(false, |(p, _)| password == p);
                if !allowed_to_modify {
                    return PersonProfileResponse::default();
                }

                let (_, nicknames) = lock.participants.profiles.get_mut(name).expect("Failed to find name");

                //remove from all other nicknames
                for nickname in nicknames.iter_mut() {
                    nickname.votes.retain(|v| *v != *voter);
                }

                if let Some(nickname) = nicknames.iter_mut().find(|n| n.nickname == *nickname) {
                    nickname.votes.push(voter.clone());
                }
                lock.save();

                Self::group_to_response_custom(&lock.participants, voter, password, &vec![name.clone()])
            }
        }
    }

    fn delete_nickname(&self, delete: &DeleteNickname) -> PersonProfileResponse {
        let DeleteNickname {
            class,
            editor,
            password,
            nickname
        } = delete;

        println!("delete_nickname: name: {}, nickname: {}", editor, nickname);

        match self.classes.get(class) {
            None => PersonProfileResponse::default(),
            Some(class) => { //class exists
                let mut lock = class.lock().expect("Failed to lock data");
                let allowed_to_modify = lock.participants.profiles.get(editor).map_or(false, |(p, _)| p == password);
                if !allowed_to_modify {
                    return PersonProfileResponse::default();
                }

                let (_ , nicknames) = lock.participants.profiles.get_mut(editor).expect("Failed to find name");
                nicknames.retain(|n| n.nickname != *nickname);
                lock.save();

                Self::group_to_response_custom(&lock.participants, &editor, &password, &vec![editor.clone()])
            }
        }
    }
}

#[actix_web::get("/class_list")]
async fn list_class(state: web::Data<State>) -> impl Responder {
    web::Json(state.list_classes())
}

#[actix_web::post("/person_profile")]
async fn person_profiles(asked: web::Json<AskForPersonProfile>, state: web::Data<State>) -> impl Responder {
    web::Json(state.person_profiles(&asked))
}

#[actix_web::post("/add_nickname")]
async fn add_nickname(add_nickname: web::Json<AddNickname>, state:  web::Data<State>) -> impl Responder {
    web::Json(state.add_nickname(&add_nickname))
}

#[actix_web::post("/vote_nickname")]
async fn vote_nickname(vote_nickname: web::Json<VoteNickname>, state:  web::Data<State>) -> impl Responder {
    web::Json(state.vote_nickname(&vote_nickname))
}

#[actix_web::post("/delete_nickname")]
async fn delete_nickname(delete_nickname: web::Json<DeleteNickname>, state:  web::Data<State>) -> impl Responder {
    web::Json(state.delete_nickname(&delete_nickname))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // install global subscriber configured based on RUST_LOG envvar.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state = Arc::new(AppState::new());

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes)
            .service(Files::new("assets", "client/dist/assets").show_files_listing())
            .service(Files::new("", "client/dist/").index_file("index.html"))

    })
        .keep_alive(KeepAlive::Os)
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(list_class);
    cfg.service(person_profiles);
    cfg.service(add_nickname);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
}