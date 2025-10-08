pub struct EditorSelector {
    name: String,
    password: String,
}

impl EditorSelector {
    const NAME_KEY: &'static str = "name";
    const PASSWORD_KEY: &'static str = "password";

    pub fn new(storage: Option<&dyn eframe::Storage>) -> Self {
        Self {
            name: storage
                .map(|s| s.get_string(Self::NAME_KEY))
                .flatten()
                .unwrap_or(String::new()),
            password: storage
                .map(|s| s.get_string(Self::PASSWORD_KEY))
                .flatten()
                .unwrap_or(String::new()),
        }
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> bool {
        ui.label("Login");
        let name_response = ui
            .add(
                egui::TextEdit::singleline(&mut self.name)
                    .hint_text("Prénom")
                    .char_limit(30),
            )
            .lost_focus();
        let password_response = ui
            .add(
                egui::TextEdit::singleline(&mut self.password)
                    .hint_text("Mot de passe")
                    .char_limit(30),
            )
            .lost_focus();
        (name_response || password_response) && !self.name.is_empty() && !self.password.is_empty()
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        storage.set_string(Self::NAME_KEY, self.name.clone());
        storage.set_string(Self::PASSWORD_KEY, self.password.clone());
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_password(&self) -> &str {
        &self.password
    }
}
