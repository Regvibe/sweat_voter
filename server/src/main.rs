use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web, App, HttpServer, Responder, Result, get};
use actix_web::http::{KeepAlive};
use actix_web::middleware::Logger;
use common::NameList;


#[derive(Clone)]
struct AppState {
    names: NameList,
}

async fn list(data: web::Data<AppState>) -> Result<impl Responder> {
    let names = data.names.clone();
    Ok(web::Json(names))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {

    HttpServer::new(|| {
            let cors = Cors::permissive();

            App::new()
                .app_data(AppState {
                    names: NameList{ names: vec!["Samy".to_owned(), "Nicolas".to_owned(), "Madeleine".to_owned(), "Hugo".to_owned()] },
                })
                .wrap(cors)
                .service(Files::new("assets", "client/dist/assets").show_files_listing())
                .service(Files::new("", "client/dist/").index_file("index.html")).wrap(Logger::default())
                .route("list", web::get().to(list))
                //.route("/list", web::get().method(Method::GET).to(list))

        })
        .keep_alive(KeepAlive::Os)
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}