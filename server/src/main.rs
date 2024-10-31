use std::io::ErrorKind;
use std::ops::Deref;
use std::sync::RwLock;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, web::ServiceConfig, App, HttpServer, Responder};
use actix_web::http::{KeepAlive};
use actix_web::middleware::Logger;
use common::{AddNickname, DeleteNickname, Nickname, Participants, VoteNickname};

use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

extern crate tracing;

#[derive(Clone)]
struct AppState {
    participants: Participants,
}

impl AppState {
    fn create() -> Self {
        match std::fs::File::open("participants.json") {
            Ok(file) => {
                let participants: Participants = serde_json::from_reader(file).expect("Failed to read participants.json");
                Self {
                    participants,
                }
            }
            Err(error) => {
                if error.kind() == ErrorKind::NotFound {
                    let participants = Participants::default();
                    let file = std::fs::File::create("participants.json").expect("Failed to create participants.json");
                    serde_json::to_writer_pretty(file, &participants).expect("Failed to write participants.json");
                }
                Self {
                    participants: Participants::default(),
                }
            }
        }
    }

    fn new() -> RwLock<Self> {
        RwLock::new(Self::create())
    }

    fn save(&self) {
        let file = std::fs::File::create("participants.json").expect("Failed to create participants.json");
        serde_json::to_writer_pretty(file, &self.participants).expect("Failed to write participants.json");
    }
}

#[actix_web::get("/list")]
async fn list(data: web::Data<RwLock<AppState>>) -> impl Responder {
    let lock = data.read().expect("Failed to lock data");
    let participants = lock.participants.clone();
    web::Json(participants)
}

#[actix_web::post("/add_nickname")]
async fn add_nickname(add_nickname: web::Json<AddNickname>, data: web::Data<RwLock<AppState>>) -> impl Responder {
    let AddNickname {
        name,
        nickname,
    } = add_nickname.deref();
    let mut lock = data.write().expect("Failed to lock data");
    let nicknames = lock.participants.names.get_mut(name).expect("Failed to find name");
    if let None = nicknames.iter().find(|n| n.nickname == nickname.trim()) { //add only if not already present
        let trim = nickname.trim();
        if trim.is_empty() {
            return web::Json(lock.participants.clone());
        }
        nicknames.push(Nickname {
            nickname: nickname.trim().to_string(),
            votes: Vec::new(),
        });
    }
    lock.save();
    web::Json(lock.participants.clone())
}

#[actix_web::post("/vote_nickname")]
async fn vote_nickname(vote_nickname: web::Json<VoteNickname>, data: web::Data<RwLock<AppState>>) -> impl Responder {
    let VoteNickname {
        name,
        nickname,
        voter,
    } = vote_nickname.deref();
    let mut lock = data.write().expect("Failed to lock data");

    let nicknames = lock.participants.names.get_mut(name).expect("Failed to find name");

    //remove from all other nicknames
    for nickname in nicknames.iter_mut() {
        nickname.votes.retain(|v| *v != *voter);
    }

    if let Some(nickname) = nicknames.iter_mut().find(|n| n.nickname == *nickname) {
        nickname.votes.push(voter.clone());
    }
    lock.save();
    web::Json(lock.participants.clone())
}

#[actix_web::post("/delete_nickname")]
async fn delete_nickname(delete_nickname: web::Json<DeleteNickname>, data: web::Data<RwLock<AppState>>) -> impl Responder {
    let DeleteNickname {
        name,
        nickname,
    } = delete_nickname.deref();
    let mut lock = data.write().expect("Failed to lock data");

    let nicknames = lock.participants.names.get_mut(name).expect("Failed to find name");
    nicknames.retain(|n| n.nickname != *nickname);
    lock.save();
    web::Json(lock.participants.clone())
}



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // install global subscriber configured based on RUST_LOG envvar.
    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .init();

    HttpServer::new(|| {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::new(AppState::new()))
            .wrap(Logger::default())
            .wrap(cors)
            .configure(routes)
            .service(Files::new("assets", "client/dist/assets").show_files_listing())
            .service(Files::new("", "client/dist/").index_file("index.html"))

    })
        .keep_alive(KeepAlive::Os)
        .bind(("0.0.0.0", 16384))?
        .run()
        .await
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(list);
    cfg.service(add_nickname);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
}