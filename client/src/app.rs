use eframe::App;
use egui::Spinner;
use oneshot::{Receiver, TryRecvError};
use common::{AddNickname, DeleteNickname, Participants, VoteNickname};
use crate::name_selector::{Action, NamesSelector};

pub struct HttpApp {
    names: Option<NamesSelector>,
    incoming_message: Option<Receiver<Participants>>,
    pub editor_name: String,
    can_try_edit: bool,
}

impl HttpApp {

    fn fetch(&mut self, request: ehttp::Request) {
        let (tx, rx) = oneshot::channel();
        self.incoming_message = Some(rx);

        ehttp::fetch(request, move |response| {
            let names = response.map(|result| String::from_utf8(result.bytes));
            if let Ok(Ok(names)) = names {
                let participants: Participants = serde_json::from_str(&names).expect(&names);
                tx.send(participants).expect("Failed to send names");
            }
        });
    }

    fn request_name(&mut self) {
        let request = ehttp::Request::get("list");
        self.fetch(request);
    }

    fn propose_nickname(&mut self, add_nickname: AddNickname) {
        let request = ehttp::Request::json("add_nickname", &add_nickname).expect("Failed to create request");
        self.fetch(request);
    }

    fn delete_nickname(&mut self, delete_nickname: DeleteNickname) {
        let request = ehttp::Request::json("delete_nickname", &delete_nickname).expect("Failed to create request");
        self.fetch(request);
    }

    fn vote_nickname(&mut self, vote_nickname: VoteNickname) {
        let request = ehttp::Request::json("vote_nickname", &vote_nickname).expect("Failed to create request");
        self.fetch(request);
    }

    fn check_incoming(&mut self) {
        if let Some(rx) = &self.incoming_message {
            match rx.try_recv() {
                Ok(participants) => {
                    self.names = Some(match self.names.take() {
                        None => NamesSelector {
                                participants,
                                selected: "".to_string(),
                                new_nickname: "".to_string(),
                            },
                        Some(old) => NamesSelector {
                                participants,
                                ..old
                            }
                        })
                },
                Err(TryRecvError::Disconnected) => {
                    self.incoming_message = None;
                },
                _ => {}
            }
        }
    }

    fn update_try_edit(&mut self) {
        self.can_try_edit = self.names.as_ref().is_some_and(|names| {
            names.participants.names.contains_key(&self.editor_name)
        });
    }

    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let mut this = Self {
            names: None,
            editor_name: "".to_string(),
            incoming_message: None,
            can_try_edit: false,
        };
        this.request_name();
        this
    }
}


impl App for HttpApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_incoming();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::top("header").show_inside(ui, |ui| {
                ui.heading("Super Selecteur de Sweat 3000");
                ui.label("Pour suivre les petites traditions du lycée Corneille, on doit proposer des surnoms aux autres pour les sweats");
                ui.label("chaque participant dispose d'une voie par personne pour choisir un surnom");
                ui.label("vous etres libre de proposer de nouveaux surnoms pour les autres");
                ui.label("vous pouvez changer vos votes à tout moment");
                ui.label("si un surnom ne vous plait pas, vous pouvez le supprimer");

                if ui.button("Rafraichir").clicked() {
                    self.request_name();
                }

                ui.label("entrez votre Nom Prénom, seul le premier mot du nom et du prénom, accent compris");
                let reponse = ui.add(egui::TextEdit::singleline(&mut self.editor_name).hint_text("Nom Prénom"));
                if reponse.changed() {
                    self.update_try_edit();
                }
            });

            if let Some(names) = &mut self.names {
                if self.can_try_edit {
                    names.display_name_selector(ui);
                    let action = names.display_nickname_selector(ui, &self.editor_name);
                    match action {
                        Action::Propose(add_nickname) => self.propose_nickname(add_nickname),
                        Action::Delete(delete_nickname) => self.delete_nickname(delete_nickname),
                        Action::Vote(vote_nickname) => self.vote_nickname(vote_nickname),
                        _ => (),
                    }
                }
            } else {
                ui.add(Spinner::new());
                ui.label("Loading...");
            }
        });
    }
}

