use common::packets::c2s::{DeleteNickname, UpdateNicknameProtection, VoteNickname};
use common::packets::s2c;
use common::packets::s2c::NicknameStatut;
use common::{ClassID, ProfilID};
use egui::{RichText, TextBuffer};
use std::collections::HashMap;

struct Profile {
    allowed_to_vote: bool,
    allowed_to_protect: bool,
    nicknames: Vec<NicknameStatut>,
}

pub struct PersonSelector {
    /// contain the profil
    profiles: HashMap<ProfilID, Profile>,
    /// who is in which class
    classes: HashMap<ClassID, Vec<(ProfilID, String)>>,
    /// current profil viewed
    selected_profil: Option<ProfilID>,
    /// edition field for a nickname proposition
    new_nickname: String,
}

pub enum ProfilAction {
    Vote(VoteNickname),
    Delete(DeleteNickname),
    UpdateProtection(UpdateNicknameProtection),
    None,
}

impl PersonSelector {
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
            classes: HashMap::new(),
            selected_profil: None,
            new_nickname: String::new(),
        }
    }

    pub fn get_selected_profil(&self) -> Option<ProfilID> {
        self.selected_profil
    }

    pub fn set_classes<T: Iterator<Item = (ClassID, Vec<(ProfilID, String)>)>>(&mut self, iter: T) {
        let iter = iter.map(|(id, mut profiles)| {
            profiles.sort_unstable_by(|(_, a), (_, b)| a.cmp(b));
            (id, profiles)
        });
        self.classes = HashMap::from_iter(iter)
    }

    /// used to cache a profil received by the server
    pub fn set_profil(&mut self, profil: s2c::Profile) {
        let s2c::Profile {
            profil_id,
            mut nicknames,
            allowed_to_vote,
            allowed_to_protect,
        } = profil;

        //always sort by the most voted !
        nicknames.sort_by(|a, b| {
            usize::cmp(&b.count, &a.count).then(String::cmp(&a.proposition, &b.proposition))
        });

        self.profiles.insert(
            profil_id,
            Profile {
                allowed_to_vote,
                allowed_to_protect,
                nicknames,
            },
        );
    }

    /// Profil selector, take which class to display and return which profil is requested
    pub fn display_name_selector(
        &mut self,
        ui: &mut egui::Ui,
        class_id: ClassID,
    ) -> Option<ProfilID> {
        let Some(profils) = self.classes.get(&class_id) else {
            return None;
        };
        let mut requested_profil = None;

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("Participants");
                    ui.label("choisissez un participant pour voir les surnoms");
                    for (id, name) in profils {
                        if ui
                            .selectable_value(&mut self.selected_profil, Some(*id), name.as_str())
                            .changed()
                        {
                            requested_profil = Some(*id);
                        }
                    }
                });
            });
        requested_profil
    }

    pub fn update_nickname_selector(&mut self, ui: &mut egui::Ui) -> ProfilAction {
        let mut action = ProfilAction::None;
        let Some(id) = self.selected_profil else {
            return action;
        };
        let Some(profil) = self.profiles.get(&id) else {
            return action;
        };

        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("nicknames").striped(true).show(ui, |ui| {
                ui.heading("Surnoms");
                ui.heading("Votes");
                ui.end_row();

                for NicknameStatut {
                    proposition,
                    count,
                    contain_you,
                    allowed_to_be_delete,
                    protected,
                } in profil.nicknames.iter()
                {
                    ui.label(proposition);

                    let color = if *contain_you {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else {
                        egui::Color32::from_rgb(100, 100, 255)
                    };

                    ui.label(RichText::new(count.to_string()).color(color));

                    if profil.allowed_to_vote && ui.button("Voter").clicked() {
                        //lazy evaluation hide the button if your not in the list
                        action = ProfilAction::Vote(VoteNickname {
                            nickname: proposition.clone(),
                            target: id,
                        });
                    }

                    if *allowed_to_be_delete && ui.button("Supprimer").clicked() {
                        action = ProfilAction::Delete(DeleteNickname {
                            nickname: proposition.clone(),
                            target: id,
                        });
                    }

                    if profil.allowed_to_protect {
                        let result = if *protected {
                            ui.button("déverrouiller")
                        } else {
                            ui.button("verrouiller")
                        };
                        if result.clicked() {
                            action = ProfilAction::UpdateProtection(UpdateNicknameProtection {
                                target: id,
                                nickname: proposition.clone(),
                                protection_statut: !protected,
                            })
                        }
                    }

                    ui.end_row();
                }
            });
        });

        if profil.allowed_to_vote {
            ui.horizontal(|ui| {
                let pressed_enter = ui
                    .add(
                        egui::TextEdit::singleline(&mut self.new_nickname)
                            .hint_text("nouveau surnom")
                            .char_limit(30),
                    )
                    .lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Proposer").clicked() || pressed_enter {
                    action = ProfilAction::Vote(VoteNickname {
                        nickname: self.new_nickname.take(),
                        target: id,
                    });
                }
            });
        }
        action
    }
}
