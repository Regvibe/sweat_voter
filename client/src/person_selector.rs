use std::collections::BTreeMap;

use egui::RichText;
use common::packets::c2s::{AddNickname, DeleteNickname, VoteNickname};
use common::packets::s2c::{PersonProfileResponse, VoteCount};

pub struct PersonSelector {
    pub persons: BTreeMap<String, BTreeMap<String, VoteCount>>,
    pub selected: String,
    pub new_nickname: String,
    pub allow_to_modify: bool,
}


pub enum Action {
    Propose(AddNickname),
    Vote(VoteNickname),
    Delete(DeleteNickname),
    None,
}

impl PersonSelector {
    pub fn new() -> Self {
        Self {
            persons: BTreeMap::new(),
            selected: String::new(),
            new_nickname: String::new(),
            allow_to_modify: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.persons.is_empty()
    }

    pub fn set_persons(&mut self, person_profile_response: PersonProfileResponse) {
        match person_profile_response {
            PersonProfileResponse { allowed_to_modify, profiles, partial_response: true,  } => { //the server only updated some participants
                self.persons.extend(profiles);
                self.allow_to_modify = allowed_to_modify;
            }
            PersonProfileResponse { allowed_to_modify, profiles, .. } => { // the server sent the whole list in one go
                self.persons = profiles; // we replace the whole list, and **do not** keep the old values
                self.allow_to_modify = allowed_to_modify;
            }
        }

    }

    pub fn display_name_selector(&mut self, ui: &mut egui::Ui) -> Vec<String> {

        let mut profile_requested = Vec::new();
        egui::SidePanel::left("left_panel").resizable(true).show_inside(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Participants");
                ui.label("choisissez un participant pour voir les surnoms");
                for name in self.persons.keys() {
                    if ui.selectable_value(&mut self.selected, name.clone(), name.as_str()).changed() { //really consider switching all theses for cow
                        profile_requested.push(name.clone());
                    }
                }
            });
        });
        profile_requested
    }

    pub fn update_nickname_selector(&mut self, ui: &mut egui::Ui, class: Option<&str>, editor_name: &str, password: &str) -> Action {
        let mut action = Action::None;
        if let (Some(class), Some(nicknames)) = (class, self.persons.get(&self.selected)) {

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("nicknames").striped(true).show(ui, |ui| {
                    ui.heading("Surnoms");
                    ui.heading("Votes");
                    ui.end_row();

                    for (nickname, vote) in nicknames.iter() {
                        ui.label(nickname);

                        let color = if vote.contain_you {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 100, 255)
                        };

                        ui.label(RichText::new(vote.count.to_string())
                            .color(color));

                        if self.allow_to_modify
                            && self.persons.contains_key(editor_name)
                            && ui.button("Voter").clicked() { //lazy evaluation hide the button if your not in the list
                            action = Action::Vote(VoteNickname {
                                class: class.to_string(),
                                name: self.selected.clone(),
                                nickname: nickname.clone(),
                                voter: editor_name.to_string(),
                                password: password.to_string(),
                            });
                        }

                        if self.allow_to_modify && editor_name == self.selected && ui.button("Supprimer").clicked() {
                            action = Action::Delete(DeleteNickname {
                                class: class.to_string(),
                                editor: editor_name.to_string(),
                                nickname: nickname.clone(),
                                password: password.to_string(),
                            });
                        }
                        ui.end_row();
                    }
                });

                if self.allow_to_modify {
                    ui.add(egui::TextEdit::singleline(&mut self.new_nickname).hint_text(format!("nouveau surnom pour {}", self.selected)).char_limit(30));
                    if ui.button("Proposer").clicked() {
                        action = Action::Propose(AddNickname {
                            class: class.to_string(),
                            editor: editor_name.to_string(),
                            password: password.to_string(),
                            name: self.selected.clone(),
                            nickname: self.new_nickname.clone(),
                        });
                        self.new_nickname.clear();
                    }
                }
            });
        }
        action
    }
}