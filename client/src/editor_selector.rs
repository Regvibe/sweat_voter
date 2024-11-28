pub struct EditorSelector {
    name: String,
    password: String,
}

impl EditorSelector {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            password: String::new(),
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label("Login");
        let name_response = ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Nom Prénom").char_limit(30)).lost_focus();
        let password_response = ui.add(egui::TextEdit::singleline(&mut self.password).hint_text("Mot de passe").char_limit(30)).lost_focus();
        return (name_response || password_response) && !self.name.is_empty() && !self.password.is_empty()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_password(&self) -> &str {
        &self.password
    }
}