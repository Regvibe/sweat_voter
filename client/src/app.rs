use crate::class_selector::ClassSelector;
use crate::editor_selector::EditorSelector;
use crate::person_selector::{Action, PersonSelector};
use common::packets::c2s::{AskForPersonProfil, DeleteNickname, VoteNickname};
use common::packets::s2c::{ClassList, Profile};
use eframe::App;
use egui::TextBuffer;
use log::warn;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

enum IncomingPacket {
    ClassList(ClassList),
    PersonProfileResponse(Profile),
}

pub struct HttpApp {
    incoming_message: Receiver<IncomingPacket>,
    sender: Sender<IncomingPacket>,
    editor_selector: EditorSelector,
    class_selector: ClassSelector,
    person_selector: PersonSelector,
    ctx: egui::Context,
}

impl HttpApp {
    #[cfg(not(target_arch = "wasm32"))]
    const ROOT: &'static str = "https://sweat.corneille-rouen.xyz/";
    #[cfg(target_arch = "wasm32")]
    const ROOT: &'static str = "";

    fn fetch<T>(&self, request: ehttp::Request, deserializer: T)
    where
        T: Send + 'static + FnOnce(String) -> Option<IncomingPacket>,
    {
        let new_sender = self.sender.clone();
        let ctx = self.ctx.clone();

        ehttp::fetch(request, move |response| {
            let response = response.map(|result| String::from_utf8(result.bytes));
            match response {
                Err(e) => {
                    warn!("Failed to fetch: {}", e);
                    return;
                }
                Ok(Err(e)) => {
                    warn!("Failed to fetch: {}", e);
                    return;
                }
                Ok(Ok(response)) => {
                    let packet = deserializer(response);
                    if let Some(packet) = packet {
                        let _ = new_sender.send(packet).expect("Failed to send packet");
                        ctx.request_repaint();
                    }
                }
            }
        });
    }

    fn request_class_list(&mut self) {
        let request = ehttp::Request::get(format!("{}class_list", Self::ROOT));
        self.fetch(request, |response| {
            let class_list: ClassList =
                serde_json::from_str(&response).expect("Failed to parse class list");
            Some(IncomingPacket::ClassList(class_list))
        });
    }

    const PROFILE_RESPONSE_HANDLER: fn(String) -> Option<IncomingPacket> = |response| {
        let person_profile_response: Profile =
            serde_json::from_str(&response).expect("Failed to parse person profile response");
        Some(IncomingPacket::PersonProfileResponse(
            person_profile_response,
        ))
    };

    fn request_person_profile(&mut self, ask_for_person_profile: AskForPersonProfil) {
        let request = ehttp::Request::json(
            format!("{}person_profile", Self::ROOT),
            &ask_for_person_profile,
        )
        .expect("Failed to create request");
        self.fetch(request, Self::PROFILE_RESPONSE_HANDLER);
    }

    fn delete_nickname(&mut self, delete_nickname: DeleteNickname) {
        let request =
            ehttp::Request::json(format!("{}delete_nickname", Self::ROOT), &delete_nickname)
                .expect("Failed to create request");
        self.fetch(request, Self::PROFILE_RESPONSE_HANDLER);
    }

    fn vote_nickname(&mut self, vote_nickname: VoteNickname) {
        let request = ehttp::Request::json(format!("{}vote_nickname", Self::ROOT), &vote_nickname)
            .expect("Failed to create request");
        self.fetch(request, Self::PROFILE_RESPONSE_HANDLER);
    }

    fn check_incoming(&mut self) {
        for message in self.incoming_message.try_iter() {
            match message {
                IncomingPacket::ClassList(class_list) => {
                    let ClassList { mut classes } = class_list;

                    self.class_selector.set_classes(
                        classes
                            .iter_mut()
                            .map(|(id, class)| (*id, class.name.take()))
                            .collect(),
                    );
                    self.person_selector.set_classes(
                        classes
                            .into_iter()
                            .map(|(class_id, class)| (class_id, class.profiles)),
                    )
                }
                IncomingPacket::PersonProfileResponse(person_profile_response) => {
                    self.person_selector.set_profil(person_profile_response)
                }
            }
        }
    }

    pub fn new(ctx: &eframe::CreationContext) -> Self {
        let editor_selector = EditorSelector::new(ctx.storage);
        let ctx = ctx.egui_ctx.clone();

        let (sender, incoming_message) = mpsc::channel();
        let mut this = Self {
            incoming_message,
            sender,
            editor_selector,
            class_selector: ClassSelector::new(),
            person_selector: PersonSelector::new(),
            ctx,
        };
        this.request_class_list();
        this
    }
}

impl App for HttpApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.check_incoming();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::TopBottomPanel::top("header").show_inside(ui, |ui| {
                #[cfg(target_arch = "wasm32")]
                ui.add_space(200.0); // Jasmine I'm going to kill you
                                     // ugliest way to leave free space to vote

                let _class_updated = self.class_selector.update(ui);
                let editor_updated = self.editor_selector.update(ui);

                if let (Some(storage), true) = (frame.storage_mut(), editor_updated) {
                    self.editor_selector.save(storage);
                }

                /*if class_updated || editor_updated {
                    if let Some(selected) = self.class_selector.get_selected() {
                        self.request_person_profile(AskForPersonProfile {
                            class: selected.to_string(),
                            editor: self.editor_selector.get_name().to_string(),
                            password: self.editor_selector.get_password().to_string(),
                            kind: RequestKind::All,
                        })
                    }
                }*/
            });

            let Some(selected_class) = self.class_selector.get_selected() else {
                return;
            };

            // when has chosen a profil to view, we need to fetch it from the server
            let requested_profiles = self
                .person_selector
                .display_name_selector(ui, selected_class);
            if let Some(profil) = requested_profiles {
                self.request_person_profile(AskForPersonProfil {
                    identity: self.editor_selector.get_identity(),
                    profil,
                })
            }

            let action = self
                .person_selector
                .update_nickname_selector(ui, self.editor_selector.get_identity());
            match action {
                Action::Delete(delete_nickname) => self.delete_nickname(delete_nickname),
                Action::Vote(vote_nickname) => self.vote_nickname(vote_nickname),
                _ => {}
            }
        });
    }
}
