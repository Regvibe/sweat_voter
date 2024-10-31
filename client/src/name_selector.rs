use egui::RichText;
use common::{AddNickname, DeleteNickname, Nickname, Participants, VoteNickname};

pub struct NamesSelector {
    pub participants: Participants,
    pub selected: String,
    pub new_nickname: String,
}

pub enum Action {
    Propose(AddNickname),
    Vote(VoteNickname),
    Delete(DeleteNickname),
    None,
}

impl NamesSelector {
    pub fn display_name_selector(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("left_panel").resizable(true).show_inside(ui, |ui| {

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Participants");
                ui.label("choisissez un participant pour voir les surnoms");
                for name in self.participants.names.keys() {
                    ui.selectable_value(&mut self.selected, name.clone(), name.as_str()); //ouch clone here
                }
            });
        });
    }

    pub fn display_nickname_selector(&mut self, ui: &mut egui::Ui, editor_name: &str) -> Action {
        let mut action = Action::None;
        if let Some(nicknames) = self.participants.names.get(&self.selected) {

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new("nicknames").striped(true).show(ui, |ui| {
                    ui.heading("Surnoms");
                    ui.heading("Votes");
                    ui.end_row();

                    for Nickname { nickname, votes } in nicknames {
                        ui.label(nickname);

                        let color = if votes.contains(&editor_name.to_string()) {
                            egui::Color32::from_rgb(255, 100, 100)
                        } else {
                            egui::Color32::from_rgb(100, 100, 255)
                        };

                        ui.label(RichText::new(votes.len().to_string())
                            .color(color));
                        if ui.button("Voter").clicked() {
                            action = Action::Vote(VoteNickname {
                                name: self.selected.clone(),
                                nickname: nickname.clone(),
                                voter: editor_name.to_string(),
                            });
                        }
                        if editor_name == self.selected && ui.button("Supprimer").clicked() {
                            action = Action::Delete(DeleteNickname {
                                name: self.selected.clone(),
                                nickname: nickname.clone(),
                            });
                        }
                        ui.end_row();
                    }
                });

                ui.add(egui::TextEdit::singleline(&mut self.new_nickname).hint_text(format!("nouveau surnom pour {}", self.selected)));
                if ui.button("Proposer").clicked() {
                    action = Action::Propose(AddNickname {
                        name: self.selected.clone(),
                        nickname: self.new_nickname.clone(),
                    });
                    self.new_nickname.clear();
                }
            });
        }
        action
    }
}