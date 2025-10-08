use common::packets::s2c::ClassList;
use egui::Spinner;

pub struct ClassSelector {
    classes: Vec<String>,
    selected: usize,
}

impl ClassSelector {
    pub fn new() -> Self {
        Self {
            classes: Vec::new(),
            selected: 0,
        }
    }

    pub fn set_classes(&mut self, list: ClassList) {
        self.classes = list.names;
    }

    pub fn update(&mut self, ui: &mut egui::Ui) -> bool {
        if self.classes.is_empty() {
            ui.add(Spinner::new());
            ui.label("Aucune classe disponible");
            return false;
        }

        let mut changed = false;
        ui.label("choisissez une classe");
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                for (i, name) in self.classes.iter().enumerate() {
                    changed |= ui.selectable_value(&mut self.selected, i, name).changed();
                }
            });
        });

        changed
    }

    pub fn get_selected(&self) -> Option<&str> {
        self.classes.get(self.selected).map(|s| s.as_str())
    }
}
