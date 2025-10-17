use crate::class_selector::ClassSelector;
use crate::login_selector::{EditorSelector, LoginAction};
use crate::person_selector::{PersonSelector, ProfilAction};
use common::packets::c2s::{AskForPersonProfil, DeleteNickname, Login, VoteNickname};
use common::packets::s2c::{ClassList, Profile};
use common::Identity;
use eframe::App;
use egui::TextBuffer;
use log::warn;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

enum IncomingPacket {
    ClassList(ClassList),
    PersonProfileResponse(Profile),
    LoginUpdate { logged: bool },
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
    const UNAUTHORIZED: u16 = 401;

    fn fetch<T>(&self, request: ehttp::Request, deserializer: T)
    where
        T: Send + 'static + FnOnce(ehttp::Response) -> Option<IncomingPacket>,
    {
        let new_sender = self.sender.clone();
        let ctx = self.ctx.clone();

        ehttp::fetch(request, move |response| {
            let response = match response {
                Ok(response) => response,
                Err(e) => {
                    warn!("Failed to fetch: {}", e);
                    return;
                }
            };

            if response.status == Self::UNAUTHORIZED {
                let _ = new_sender
                    .send(IncomingPacket::LoginUpdate { logged: false })
                    .expect("Failed to channel packet");
                ctx.request_repaint();
                return;
            }

            if let Some(packet) = deserializer(response) {
                let _ = new_sender.send(packet).expect("Failed to channel packet");
                ctx.request_repaint();
            }
        });
    }

    fn request_class_list(&mut self) {
        let request = ehttp::Request::get(format!("{}class_list", Self::ROOT));
        self.fetch(request, |response| {
            Some(IncomingPacket::ClassList(response.json().ok()?))
        });
    }

    const PROFILE_RESPONSE_HANDLER: fn(ehttp::Response) -> Option<IncomingPacket> =
        |response| Some(IncomingPacket::PersonProfileResponse(response.json().ok()?));

    fn login(&mut self, identity: Identity) {
        let request = ehttp::Request::json(format!("{}login", Self::ROOT), &Login { identity })
            .expect("Failed to create request");
        self.fetch(request, |response| {
            Some(IncomingPacket::LoginUpdate {
                logged: response.ok,
            })
        });
    }

    fn logout(&mut self) {
        let request = ehttp::Request::post(format!("{}logout", Self::ROOT), vec![]);
        self.fetch(request, |response| {
            response
                .ok
                .then_some(IncomingPacket::LoginUpdate { logged: false })
        })
    }

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
        let mut should_update_viewed_profil = false;
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
                IncomingPacket::LoginUpdate { logged } => {
                    should_update_viewed_profil = true;
                    self.editor_selector.set_logged(logged)
                }
            }
        }
        if let Some(profil) = self
            .person_selector
            .get_selected_profil()
            .filter(|_| should_update_viewed_profil)
        {
            self.request_person_profile(AskForPersonProfil { profil })
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
        if !this.editor_selector.is_empty() {
            this.login(this.editor_selector.get_identity())
        }
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

                let action = self.editor_selector.update(ui);

                match action {
                    LoginAction::Login => {
                        self.login(self.editor_selector.get_identity());
                        if let Some(storage) = frame.storage_mut() {
                            self.editor_selector.save(storage)
                        }
                    }
                    LoginAction::Logout => self.logout(),
                    _ => (),
                }

                self.class_selector.update(ui);
            });

            let Some(selected_class) = self.class_selector.get_selected() else {
                return;
            };

            // when has chosen a profil to view, we need to fetch it from the server
            let requested_profiles = self
                .person_selector
                .display_name_selector(ui, selected_class);
            if let Some(profil) = requested_profiles {
                self.request_person_profile(AskForPersonProfil { profil })
            }

            let action = self.person_selector.update_nickname_selector(ui);
            match action {
                ProfilAction::Delete(delete_nickname) => self.delete_nickname(delete_nickname),
                ProfilAction::Vote(vote_nickname) => self.vote_nickname(vote_nickname),
                _ => {}
            }
        });
    }
}
