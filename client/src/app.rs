use eframe::App;
use oneshot::{Receiver, TryRecvError};

enum WaitingMessage {
    None,
    Names(Receiver<Vec<String>>),
}

pub struct HttpApp {
    names: Option<Vec<String>>,
    incoming_message: WaitingMessage,
}

impl HttpApp {

    fn request_name(&mut self) {
        let request = ehttp::Request::get("https://localhost:8080/list");
        let (tx, rx) = oneshot::channel();
        let incoming_message = WaitingMessage::Names(rx);

        ehttp::fetch(request, move |response| {
            println!("response: {:?}", response);
            let names = response.map(|result| String::from_utf8(result.bytes));
            if let Ok(Ok(names)) = names {
                let names: common::NameList = serde_json::from_str(&names).unwrap();
                tx.send(names.names).unwrap();
            }
        });
        self.incoming_message = incoming_message;
    }

    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let mut this = Self {
            names: None,
            incoming_message: WaitingMessage::None,
        };
        this.request_name();
        this
    }
}

impl App for HttpApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Names");
            if let WaitingMessage::Names(rx) = &self.incoming_message {
                match rx.try_recv() {
                    Ok(names) => self.names = Some(names),
                    Err(TryRecvError::Disconnected) => self.incoming_message = WaitingMessage::None,
                    _ => (),
                }
            }

            match &self.names {
                Some(names) => {
                    for name in names {
                        ui.label(name);
                    }
                }
                None => {
                    if let WaitingMessage::Names(_) = &self.incoming_message {
                        ui.label("Loading names...");
                    } else {
                        ui.label("working on it...");
                        self.request_name();
                    };
                }
            }
        });
    }
}

