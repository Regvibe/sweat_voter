use actix_cors::Cors;
use actix_files::Files;
use actix_web::http::KeepAlive;
use actix_web::middleware::Logger;
use actix_web::{web, web::ServiceConfig, App, HttpServer, Responder};
use common::packets::c2s::{
    AddNickname, AskForPersonProfile, DeleteNickname, RequestKind, VoteNickname,
};
use common::packets::s2c::{ClassList, Permissions, PersonProfileResponse, VoteCount};
use common::{AdminList, Group, Nickname};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{info, warn};

extern crate tracing;

type State = Arc<Mutex<AppState>>;

struct Class {
    path: PathBuf,
    participants: Group,
    dirty: bool, // should be saved the next time the server will try to save ?
}

#[derive(Debug)]
enum NicknameOperationError {
    TargetDoesntExist,
    NicknameNotFound,
}

impl Display for NicknameOperationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NicknameOperationError::TargetDoesntExist => {
                f.write_str("this person doesn't belong to the class")
            }
            NicknameOperationError::NicknameNotFound => {
                f.write_str("this nickname isn't registered")
            }
        }
    }
}

impl Error for NicknameOperationError {}

impl Class {
    fn new(path: PathBuf) -> anyhow::Result<Self> {
        let json = File::open(&path)?;
        let participants: Group = serde_json::from_reader(json)?;

        Ok(Self {
            path,
            participants,
            dirty: false,
        })
    }

    /// check if a given person is in the class, and logged with the right password
    fn is_allowed_to_modify(&self, editor: &String, password: &String) -> bool {
        self.participants
            .profiles
            .get(editor)
            .map_or(false, |(p, _)| p == password)
    }

    fn add_nickname(
        &mut self,
        target: &String,
        nickname: &String,
    ) -> Result<(), NicknameOperationError> {
        let Some((_, nicknames)) = self.participants.profiles.get_mut(target) else {
            return Err(NicknameOperationError::TargetDoesntExist);
        };

        //check if nickname is not already present and add it
        let trim = nickname.trim();
        if !trim.is_empty() && nicknames.iter().find(|n| n.nickname == trim).is_none() {
            //add only if not already present
            nicknames.push(Nickname {
                nickname: nickname.trim().to_string(),
                votes: Vec::new(),
            });

            self.dirty = true;
        }
        Ok(())
    }

    fn vote_nickname(
        &mut self,
        target: &String,
        voter: &String,
        nickname: &String,
    ) -> Result<(), NicknameOperationError> {
        let Some((_, nicknames)) = self.participants.profiles.get_mut(target) else {
            return Err(NicknameOperationError::TargetDoesntExist);
        };

        //remove from all other nicknames
        for nickname in nicknames.iter_mut() {
            nickname.votes.retain(|v| *v != *voter);
        }

        let Some(nickname) = nicknames.iter_mut().find(|n| n.nickname == *nickname) else {
            return Err(NicknameOperationError::NicknameNotFound);
        };
        nickname.votes.push(voter.clone());
        self.dirty = true;
        Ok(())
    }

    fn delete_nickname(
        &mut self,
        target: &String,
        nickname: &String,
    ) -> Result<(), NicknameOperationError> {
        let Some((_, nicknames)) = self.participants.profiles.get_mut(target) else {
            return Err(NicknameOperationError::TargetDoesntExist);
        };

        nicknames.retain(|n| n.nickname != *nickname);
        self.dirty = true;
        Ok(())
    }

    fn save_if_dirty(&mut self) {
        if self.dirty {
            self.save();
            self.dirty = false;
        }
    }

    fn save(&self) {
        let file = File::create(&self.path)
            .expect(format!("Failed to create {}", self.path.display()).as_str());
        serde_json::to_writer_pretty(file, &self.participants)
            .expect(format!("Failed to write {}", self.path.display()).as_str());
    }
}

struct AppState {
    classes: HashMap<String, Class>, //class name -> Class
    admin_list: AdminList,
}

impl AppState {
    fn new() -> Mutex<Self> {
        let mut groups = HashMap::new();

        let files = std::fs::read_dir("./classes").expect("Failed to read dir");
        for file in files.flatten() {
            let path = file.path();
            if path.is_file() && path.extension() == Some("json".as_ref()) {
                let name = path
                    .file_stem()
                    .get_or_insert("unknown".as_ref())
                    .to_string_lossy()
                    .to_string();
                info!("found class: {} at {:?}", name, path);

                match Class::new(path) {
                    Ok(class) => {
                        groups.insert(name, class);
                    }
                    Err(e) => warn!("Failed to load class: {:?}", e),
                }
            }
        }

        let file = File::open("admins.json").expect("Failed to open admins.json");
        let admin_list: AdminList =
            serde_json::from_reader(file).expect("Failed to parse admins.json");

        Mutex::new(AppState {
            classes: groups,
            admin_list,
        })
    }

    /// Create the packet referencing all the classes
    fn list_classes(&self) -> ClassList {
        let names = self.classes.keys().cloned().collect::<Vec<String>>();
        ClassList { names }
    }

    fn is_admin(&self, name: &str, password: &str) -> bool {
        if !self.admin_list.admins.contains(name) {
            return false;
        }
        self.classes.iter().any(|(_, class)| {
            class
                .participants
                .profiles
                .get(name)
                .map_or(false, |(p, _)| p == password)
        })
    }

    fn make_nickname_map(
        nickname_list: &Vec<Nickname>,
        editor_name: Option<&str>,
        include_voter: bool,
    ) -> BTreeMap<String, VoteCount> {
        let mut map = BTreeMap::new();
        for nickname in nickname_list {
            let contain_you = editor_name.is_some_and(|s| //display if the viewer voted
                                                  nickname
                                                      .votes
                                                      .iter()
                                                      .any(|v| *v == s));

            let voters = if include_voter {
                nickname.votes.clone()
            } else {
                Vec::new()
            };

            map.insert(
                nickname.nickname.clone(),
                VoteCount {
                    count: nickname.votes.len(),
                    contain_you, //check if the editor voted
                    voters,
                },
            );
        }
        map
    }

    fn convert_group(
        group: &Group,
        editor_name: Option<&str>,
        include_voter: bool,
    ) -> BTreeMap<String, BTreeMap<String, VoteCount>> {
        let mut map = BTreeMap::new();
        for (name, (_, nicknames)) in &group.profiles {
            map.insert(
                name.clone(),
                Self::make_nickname_map(nicknames, editor_name, include_voter),
            );
        }
        map
    }

    fn convert_group_custom(
        group: &Group,
        editor_name: Option<&str>,
        requested: &Vec<String>,
        include_voter: bool,
    ) -> BTreeMap<String, BTreeMap<String, VoteCount>> {
        let mut map = BTreeMap::new();
        for requested_name in requested {
            if let Some((_, nicknames)) = group.profiles.get(requested_name) {
                map.insert(
                    requested_name.clone(),
                    Self::make_nickname_map(nicknames, editor_name, include_voter),
                );
            }
        }
        map
    }

    fn group_to_response(
        group: &Group,
        editor_name: &str,
        password: &str,
        is_admin: bool,
    ) -> PersonProfileResponse {
        let allowed_to_modify = is_admin
            || group
                .profiles
                .get(editor_name)
                .map_or(false, |(p, _)| p == password);

        PersonProfileResponse {
            should_overwrite: true,
            permissions: Permissions::perm(allowed_to_modify, is_admin),
            profiles: Self::convert_group(
                group,
                allowed_to_modify.then_some(editor_name),
                is_admin,
            ),
        }
    }

    fn group_to_response_custom(
        group: &Group,
        editor_name: &str,
        password: &str,
        requested: &Vec<String>,
        is_admin: bool,
    ) -> PersonProfileResponse {
        let allowed_to_modify = is_admin
            || group
                .profiles
                .get(editor_name)
                .map_or(false, |(p, _)| p == password);

        PersonProfileResponse {
            should_overwrite: false,
            permissions: Permissions::perm(allowed_to_modify, is_admin),
            profiles: Self::convert_group_custom(
                group,
                allowed_to_modify.then_some(editor_name),
                requested,
                is_admin,
            ),
        }
    }

    fn person_profiles(&self, asked: &AskForPersonProfile) -> PersonProfileResponse {
        info!("person_profiles: {:?}", asked);
        let is_admin = self.is_admin(&asked.editor, &asked.password);

        match (self.classes.get(&asked.class), &asked.kind) {
            (Some(class), RequestKind::All) => Self::group_to_response(
                &class.participants,
                &asked.editor,
                &asked.password,
                is_admin,
            ),
            (Some(class), RequestKind::Custom(requested)) => Self::group_to_response_custom(
                &class.participants,
                &asked.editor,
                &asked.password,
                &requested,
                is_admin,
            ),
            (None, _) => PersonProfileResponse {
                should_overwrite: false,
                permissions: Permissions::NONE,
                profiles: BTreeMap::new(),
            },
        }
    }

    fn add_nickname(&mut self, add: &AddNickname) -> PersonProfileResponse {
        let AddNickname {
            class,
            editor,
            password,
            name,
            nickname,
        } = add;
        info!(
            "add_nickname: {} to {} by {} in class {}",
            nickname, name, editor, class
        );

        let is_admin = self.is_admin(editor, password);

        match self.classes.get_mut(class) {
            None => PersonProfileResponse::default(),
            Some(class) => {
                let allowed_to_modify = is_admin || class.is_allowed_to_modify(editor, password);
                if allowed_to_modify {
                    class
                        .add_nickname(name, nickname)
                        .unwrap_or_else(|e| warn!("error while trying to add nickname: {}", e));
                    Self::group_to_response_custom(
                        &class.participants,
                        editor,
                        password,
                        &vec![name.clone()],
                        is_admin,
                    )
                } else {
                    PersonProfileResponse::default()
                }
            }
        }
    }

    fn vote_nickname(&mut self, vote: &VoteNickname) -> PersonProfileResponse {
        let VoteNickname {
            class,
            name,
            nickname,
            voter,
            password,
        } = vote;
        info!(
            "vote_nickname: name: {}, nickname: {}, voter: {}",
            name, nickname, voter
        );

        let is_admin = self.is_admin(voter, password);

        match self.classes.get_mut(class) {
            None => PersonProfileResponse::default(),
            Some(class) => {
                let allowed_to_modify = is_admin || class.is_allowed_to_modify(voter, password);
                if allowed_to_modify {
                    class
                        .vote_nickname(name, voter, nickname)
                        .unwrap_or_else(|e| {
                            warn!("error while trying to vote for nickname: {}", e)
                        });
                    Self::group_to_response_custom(
                        &class.participants,
                        voter,
                        password,
                        &vec![name.clone()],
                        is_admin,
                    )
                } else {
                    PersonProfileResponse::default()
                }
            }
        }
    }

    fn delete_nickname(&mut self, delete: &DeleteNickname) -> PersonProfileResponse {
        let DeleteNickname {
            class,
            editor,
            name,
            password,
            nickname,
        } = delete;

        info!(
            "delete_nickname: name: {}, nickname: {}, by {}",
            editor, nickname, name
        );

        let is_admin = self.is_admin(editor, password);

        match self.classes.get_mut(class) {
            None => PersonProfileResponse::default(),
            Some(class) => {
                let allowed_to_modify = is_admin
                    || class
                        .participants
                        .profiles
                        .get(editor)
                        .map_or(false, |(p, _)| p == password);
                if allowed_to_modify {
                    class.delete_nickname(name, nickname).unwrap_or_else(|e| {
                        warn!("error while trying to delete a nickname: {}", e)
                    });
                    Self::group_to_response_custom(
                        &class.participants,
                        editor,
                        password,
                        &vec![name.clone()],
                        is_admin,
                    )
                } else {
                    PersonProfileResponse::default()
                }
            }
        }
    }
}

#[actix_web::get("/class_list")]
async fn list_class(state: web::Data<State>) -> impl Responder {
    web::Json(state.lock().unwrap().list_classes())
}

#[actix_web::post("/person_profile")]
async fn person_profiles(
    asked: web::Json<AskForPersonProfile>,
    state: web::Data<State>,
) -> impl Responder {
    web::Json(state.lock().unwrap().person_profiles(&asked))
}

#[actix_web::post("/add_nickname")]
async fn add_nickname(
    add_nickname: web::Json<AddNickname>,
    state: web::Data<State>,
) -> impl Responder {
    web::Json(state.lock().unwrap().add_nickname(&add_nickname))
}

#[actix_web::post("/vote_nickname")]
async fn vote_nickname(
    vote_nickname: web::Json<VoteNickname>,
    state: web::Data<State>,
) -> impl Responder {
    web::Json(state.lock().unwrap().vote_nickname(&vote_nickname))
}

#[actix_web::post("/delete_nickname")]
async fn delete_nickname(
    delete_nickname: web::Json<DeleteNickname>,
    state: web::Data<State>,
) -> impl Responder {
    web::Json(state.lock().unwrap().delete_nickname(&delete_nickname))
}

async fn save_loop(state: Arc<Mutex<AppState>>) {
    let mut interval = actix_web::rt::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let mut state = state.lock().unwrap();
        for (_, class) in state.classes.iter_mut() {
            class.save_if_dirty();
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // install global subscriber configured based on RUST_LOG envvar.
    tracing_subscriber::fmt().init();

    info!("Starting server");

    let state = Arc::new(AppState::new());

    let future = save_loop(state.clone());

    actix_web::rt::spawn(future);
    let cloned = state.clone();

    let e = HttpServer::new(move || {
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
    .await;

    info!("server stopping");
    let mut state = cloned.lock().unwrap();
    for (_, class) in state.classes.iter_mut() {
        class.save_if_dirty();
    }
    info!("content saved");
    e
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(list_class);
    cfg.service(person_profiles);
    cfg.service(add_nickname);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
}
