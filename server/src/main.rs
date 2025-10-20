mod commands;
mod data_server;

use crate::commands::AddProfil;
use crate::data_server::{compat, serialization, DataServer, NickNameProposition};
use actix_cors::Cors;
use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::http::KeepAlive;
use actix_web::{
    web, web::ServiceConfig, App, Either, HttpMessage, HttpRequest, HttpResponse, HttpServer,
    Responder,
};
use common::packets::c2s::{
    AskForPersonProfil, DeleteNickname, Login, UpdateNicknameProtection, VoteNickname,
};
use common::ProfilID;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::stdin;
use std::sync::Mutex;
use structopt::clap::{AppSettings, Error};
use structopt::StructOpt;
use tokio::task::spawn_blocking;
use tracing::info;

extern crate tracing;

type State = Mutex<AppState>;

struct AppState {
    data_server: DataServer,
}

impl AppState {
    fn save(&mut self) {
        if let Some(nicknames) = self.data_server.try_to_save_nickname() {
            let file = File::create("nicknames.json").unwrap();
            serde_json::to_writer_pretty(file, &nicknames).unwrap()
        }
    }

    fn load_nicknames() -> Option<HashMap<ProfilID, Vec<NickNameProposition>>> {
        let file = File::open("nicknames.json").ok()?;
        serde_json::from_reader(file).ok()
    }

    fn people_repartition() -> serialization::PeopleRepartition {
        let file = File::open("classes.json");

        match file {
            Ok(file) => serde_json::from_reader(file).unwrap(),
            Err(_) => {
                let template = serialization::PeopleRepartition::template();
                let file = File::create("classes.json").expect("Failed to create a template file");
                serde_json::to_writer_pretty(file, &template).unwrap();
                template
            }
        }
    }

    fn load_old_nicknames(data_server: &mut DataServer) {
        let files = fs::read_dir("./classes").expect("Failed to read dir");
        for file in files.flatten() {
            let path = file.path();
            if path.is_file() && path.extension() == Some("json".as_ref()) {
                let name = path
                    .file_stem()
                    .get_or_insert("unknown".as_ref())
                    .to_string_lossy()
                    .to_string();

                info!("found class: {} at {:?}", name, path);
                let json = File::open(&path).unwrap();
                let participants: compat::Group = serde_json::from_reader(json).unwrap();
                data_server.import_old_nickname(participants);
                let _ = fs::remove_file(&path);
            }
        }
    }

    fn id_map() -> serialization::IdMap {
        let Ok(file) = File::open("id_map.json") else {
            return Default::default();
        };
        let Ok(map) = serde_json::from_reader(file) else {
            return Default::default();
        };
        map
    }

    fn new() -> Mutex<Self> {
        let people_repartition = Self::people_repartition();
        let id_map = Self::id_map();
        let mut data_server = DataServer::new(people_repartition, id_map);

        if let Some(nicknames) = Self::load_nicknames() {
            data_server.load_proposition(nicknames);
        }

        Self::load_old_nicknames(&mut data_server);

        let generated_id_map = data_server.build_id_map();

        let file = File::create("id_map.json").expect("Failed to create a id_map file");
        serde_json::to_writer_pretty(file, &generated_id_map).unwrap();

        Mutex::new(AppState { data_server })
    }
}

fn get_id(data_server: &DataServer, user: Option<actix_identity::Identity>) -> Option<ProfilID> {
    let name = user?.id().ok()?;
    data_server.get_profil_id(&name)
}

#[actix_web::post("/login")]
async fn login(
    login: web::Json<Login>,
    req: HttpRequest,
    state: web::Data<State>,
) -> impl Responder {
    if state.lock().unwrap().data_server.log(&login.identity) {
        actix_identity::Identity::login(&req.extensions(), login.identity.name.clone()).unwrap();
        HttpResponse::Ok()
    } else {
        HttpResponse::Unauthorized()
    }
}

#[actix_web::post("/logout")]
async fn logout(user: Option<actix_identity::Identity>) -> impl Responder {
    if let Some(user) = user {
        user.logout();
    }
    HttpResponse::Ok()
}

#[actix_web::get("/class_list")]
async fn list_class(state: web::Data<State>) -> impl Responder {
    web::Json(state.lock().unwrap().data_server.class_list())
}

#[actix_web::post("/person_profile")]
async fn person_profile(
    asked: web::Json<AskForPersonProfil>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let AskForPersonProfil { profil } = asked.0;
    let server = &state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    web::Json(server.personne_profil(id, profil))
}

#[actix_web::post("/vote_nickname")]
async fn vote_nickname(
    vote_nickname: web::Json<VoteNickname>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let VoteNickname { target, nickname } = vote_nickname.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    if let Some(id) = id {
        server.vote(id, target, nickname);
        Either::Left(web::Json(server.personne_profil(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

#[actix_web::post("/delete_nickname")]
async fn delete_nickname(
    delete_nickname: web::Json<DeleteNickname>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let DeleteNickname { target, nickname } = delete_nickname.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);

    if let Some(id) = id {
        server.delete(id, target, nickname);
        Either::Left(web::Json(server.personne_profil(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

#[actix_web::post("/update_nickname_protection")]
async fn update_protection_nickname(
    nickname_protection_update: web::Json<UpdateNicknameProtection>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let UpdateNicknameProtection {
        target,
        nickname,
        protection_statut,
    } = nickname_protection_update.0;
    let server = &mut state.lock().unwrap().data_server;
    let id = get_id(&server, user);

    if let Some(id) = id {
        server.update_nickname_protection(id, target, nickname, protection_statut);
        Either::Left(web::Json(server.personne_profil(Some(id), target)))
    } else {
        Either::Right(HttpResponse::Unauthorized())
    }
}

async fn save_loop(state: web::Data<Mutex<AppState>>) {
    let mut interval = actix_web::rt::time::interval(std::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        let mut state = state.lock().unwrap();
        state.save()
    }
}

#[derive(StructOpt)]
enum Commands {
    Exit,
    AddProfil(AddProfil),
}

fn wait_for_cmd_input() {
    let mut command = String::new();
    loop {
        command.clear();
        match stdin().read_line(&mut command) {
            Ok(_) => {
                let iter = command.trim().split_ascii_whitespace();

                let clap = Commands::clap().setting(AppSettings::NoBinaryName);
                let command = clap.get_matches_from_safe(iter);
                let command = match command {
                    Ok(command) => command,
                    Err(e) => {
                        println!("{}", e);
                        continue;
                    }
                };

                match Commands::from_clap(&command) {
                    Commands::Exit => return,
                    Commands::AddProfil(add) => {
                        println!("{:?}", add)
                    }
                }
            }
            Err(e) => println!("{}", e),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // install global subscriber configured based on RUST_LOG envvar.
    tracing_subscriber::fmt().init();
    let secret_key = Key::generate();

    info!("Starting server");

    let state = web::Data::new(AppState::new());

    let cloned = state.clone();
    tokio::spawn(save_loop(state.clone()));

    let signal = async || {
        spawn_blocking(wait_for_cmd_input).await.unwrap();
    };

    let e = HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::clone(&state))
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            //.wrap(Logger::default())
            .wrap(cors)
            .configure(routes)
            .service(Files::new("assets", "client/dist/assets").show_files_listing())
            .service(Files::new("", "client/dist/").index_file("index.html"))
    })
    .shutdown_signal(signal())
    .keep_alive(KeepAlive::Os)
    .bind(("0.0.0.0", 8080))?
    .run()
    .await;

    info!("server stopping");
    cloned.lock().unwrap().save();
    info!("content saved");
    e
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(login);
    cfg.service(logout);
    cfg.service(list_class);
    cfg.service(person_profile);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
    cfg.service(update_protection_nickname);
}
