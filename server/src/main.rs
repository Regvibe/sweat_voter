mod commands;
mod data_server;

use crate::commands::{AddClass, AddLonelyToClass, AddProfil, AddToClass, ChangeName, ChangePassword, ChangePermission, DeleteClass, DeleteProfil, PermissionKind, RemoveFromClass, ViewPassword};
use crate::data_server::{compat, serialization, DataServer, NickNameProposition, ServerError};
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
use common::packets::c2s::{AskForPersonProfil, CommandInput, DeleteNickname, Login, UpdateNicknameProtection, VoteNickname};
use common::ProfilID;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::stdin;
use std::sync::Mutex;
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tokio::task::spawn_blocking;
use tracing::info;
use common::packets::c2s;
use common::packets::s2c::CommandResponse;
use crate::data_server::permissions::Permissions;

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
        if let Some((repartition, id_map)) = self.data_server.try_to_save_profils() {
            let file = File::create("classes.json").unwrap();
            serde_json::to_writer_pretty(file, &repartition).unwrap();
            let file = File::create("id_map.json").unwrap();
            serde_json::to_writer_pretty(file, &id_map).unwrap();
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

    fn execute_command(&mut self, command: Commands) -> Result<Option<String>, ServerError> {
        let server = &mut self.data_server;
        match command {
            Commands::Exit => Ok(Some("You can't shutdown the server from here".to_string())),
            Commands::AddProfil(AddProfil { name, password }) => {
                server.add_profile(name, password).map(|_| None)
            }
            Commands::DeleteProfil(DeleteProfil { name }) => {
                server.delete_profil(name).map(|_| None)
            }
            Commands::AddClass(AddClass { name }) => server.add_class(name).map(|_| None),
            Commands::DeleteClass(DeleteClass { name }) => server.delete_class(name).map(|_| None),
            Commands::ViewLonelyPeople => {
                use std::fmt::Write;

                let peoples = server.find_people_out_of_any_class();
                let mut output = String::new();
                if peoples.is_empty() {
                    writeln!(&mut output, "No people found!").unwrap();
                } else {
                }
                for people in peoples {
                    writeln!(&mut output, "{}", people).unwrap();
                }
                Ok(Some(output))
            }
            Commands::AddLonelyPeopleToClass(AddLonelyToClass{ class }) => {
                let people = server.find_id_out_of_any_class();
                for id in people {
                    server.add_to_class(id, &class)?;
                }
                Ok(None)
            }
            Commands::ViewPassword(ViewPassword { name }) => {
                let id = server.get_profil_id(&name)?;
                let password = server.get_password(id)?;
                Ok(Some(format!("{} password is {}", name, password)))
            }
            Commands::ChangePassword(ChangePassword { name, new_password }) => {
                let id = server.get_profil_id(&name)?;
                server.change_password(id, new_password)?;
                Ok(None)
            }
            Commands::ChangeName(ChangeName { name, new_name }) => {
                server.change_name(name, new_name).map(|_| None)
            }
            Commands::AddToClass(AddToClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.add_to_class(id, &class_name)?;
                Ok(None)
            }
            Commands::RemoveFromClass(RemoveFromClass {
                profil_name,
                class_name,
            }) => {
                let id = server.get_profil_id(&profil_name)?;
                server.remove_from_class(id, class_name)?;
                Ok(None)
            }
            Commands::ChangePerm(ChangePermission { name, kind }) => {
                let id = server.get_profil_id(&name)?;
                let perm = server.get_permissions_mut(id)?;
                match kind {
                    PermissionKind::Vote { permission } => perm.vote = permission,
                    PermissionKind::Delete { permission } => perm.delete = permission,
                    PermissionKind::Protect { permission } => perm.protect_nickname = permission,
                    PermissionKind::UseCmd { permission } => perm.allowed_to_use_cmd = permission,
                }
                Ok(None)
            }
        }
    }
}

fn get_id(data_server: &DataServer, user: Option<actix_identity::Identity>) -> Option<ProfilID> {
    let name = user?.id().ok()?;
    data_server.get_profil_id(&name).ok()
}

#[actix_web::post("/login")]
async fn login(
    login: web::Json<Login>,
    req: HttpRequest,
    state: web::Data<State>,
) -> impl Responder {
    let server = &state.lock().unwrap().data_server;
    let id = server.log(&login.identity);
    if id.is_some() {
        actix_identity::Identity::login(&req.extensions(), login.identity.name.clone()).unwrap();
    };
    web::Json(server.class_list(id))
}

#[actix_web::post("/change_password")]
async fn change_password(new_password: web::Json<c2s::ChangePassword>, state: web::Data<State>, user: Option<actix_identity::Identity>) -> impl Responder {

    let server = &mut state.lock().unwrap().data_server;
    let Some(id) = get_id(&server, user) else {
        return HttpResponse::Unauthorized();
    };
    if server.change_password(id, new_password.0.new_password).is_ok() {
        HttpResponse::Ok()
    } else {
        HttpResponse::BadRequest()
    }
}

#[actix_web::post("/logout")]
async fn logout(state: web::Data<State>, user: Option<actix_identity::Identity>) -> impl Responder {
    if let Some(user) = user {
        user.logout();
    }
    let server = &state.lock().unwrap().data_server;
    web::Json(server.class_list(None))
}

#[actix_web::get("/class_list")]
async fn list_class(
    state: web::Data<State>,
    user: Option<actix_identity::Identity>,
) -> impl Responder {
    let server = &state.lock().unwrap().data_server;
    let id = get_id(&server, user);
    web::Json(server.class_list(id))
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

#[actix_web::post("/cmd_input")]
async fn cmd_input(
    cmd: web::Json<CommandInput>,
    state: web::Data<State>,
    user: Option<actix_identity::Identity>
) -> impl Responder {
    let app = &mut state.lock().unwrap();
    let Some(id) = get_id(&app.data_server, user) else { return Either::Right(HttpResponse::Unauthorized()); };

    if let Some(Permissions{ allowed_to_use_cmd: false, .. }) = app.data_server.get_permission(id) { return Either::Right(HttpResponse::Unauthorized()); };

    let Some(inputs) = shlex::split(&cmd.text) else {
        return Either::Left(web::Json(CommandResponse{
            text: "this command could not be parsed, check your quotes".to_string(),
        }));
    };

    let clap = Commands::clap().setting(AppSettings::NoBinaryName);
    let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
    let command = match command {
        Ok(command) => Commands::from_clap(&command),
        Err(e) => {
            return Either::Left(web::Json(CommandResponse{
                text: e.to_string(),
            }));
        }
    };

    let result = app.execute_command(command);
    let text = match result {
        Ok(None) => "action performed successfully!".to_string(),
        Ok(Some(result)) => result.trim().to_string(),
        Err(e) => e.to_string(),
    };

    Either::Left(web::Json(CommandResponse{
        text
    }))
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
    DeleteProfil(DeleteProfil),
    AddClass(AddClass),
    DeleteClass(DeleteClass),
    ViewLonelyPeople,
    AddLonelyPeopleToClass(AddLonelyToClass),
    ViewPassword(ViewPassword),
    ChangePassword(ChangePassword),
    ChangeName(ChangeName),
    AddToClass(AddToClass),
    RemoveFromClass(RemoveFromClass),
    ChangePerm(ChangePermission),
}

fn wait_for_cmd_input(server: web::Data<Mutex<AppState>>) {
    let mut command = String::new();
    loop {
        // read stdin
        command.clear();
        if let Err(e) = stdin().read_line(&mut command) {
            println!("{}", e);
            continue;
        }

        // parse the command
        let Some(inputs) = shlex::split(&command) else {
            println!("this command could not be parsed, check your quotes");
            continue;
        };

        let clap = Commands::clap().setting(AppSettings::NoBinaryName);
        let command = clap.get_matches_from_safe(inputs.iter().map(|input| input.trim()));
        let command = match command {
            Ok(command) => Commands::from_clap(&command),
            Err(e) => {
                println!("{}", e);
                continue;
            }
        };

        if let Commands::Exit = command {
            return;
        }

        let result = server.lock().unwrap().execute_command(command);
        match result {
            Ok(None) => println!("action performed successfully!"),
            Ok(Some(result)) => println!("{}", result.trim()),
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
    let cloned2 = state.clone();
    tokio::spawn(save_loop(state.clone()));

    let signal = async || {
        spawn_blocking(move || wait_for_cmd_input(cloned))
            .await
            .unwrap();
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
    cloned2.lock().unwrap().save();
    info!("content saved");
    e
}

fn routes(cfg: &mut ServiceConfig) {
    cfg.service(login);
    cfg.service(logout);
    cfg.service(change_password);
    cfg.service(list_class);
    cfg.service(person_profile);
    cfg.service(delete_nickname);
    cfg.service(vote_nickname);
    cfg.service(update_protection_nickname);
    cfg.service(cmd_input);
}
