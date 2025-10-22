use crate::data_server::permissions::{InteractionPermission, Permissions};
use common::{ClassID, Identity, ProfilID};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Profil {
    #[serde(flatten)]
    pub identity: Identity,
    #[serde(default)]
    #[serde(skip_serializing_if = "Permissions::is_default")]
    pub permissions: Permissions,
}

#[derive(Serialize, Deserialize)]
pub struct PeopleRepartition {
    pub profiles: Vec<Profil>,
    pub classes: Vec<Class>,
}

impl PeopleRepartition {
    pub fn template() -> Self {
        let admin = Profil {
            identity: Identity {
                name: "Admin".to_string(),
                password: "mdp".to_string(),
            },
            permissions: Permissions {
                vote: InteractionPermission::AnyBody,
                delete: InteractionPermission::SameClass,
                protect_nickname: InteractionPermission::SameClass,
            },
        };
        let someone = Profil {
            identity: Identity {
                name: "Someone".to_string(),
                password: "mdp".to_string(),
            },
            permissions: Default::default(),
        };
        let class = Class {
            name: "TemplateClass".to_string(),
            people: vec!["Admin".to_string(), "Someone".to_string()],
        };

        PeopleRepartition {
            profiles: vec![admin, someone],
            classes: vec![class],
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct IdMap {
    pub profil_mapping: Vec<(ProfilID, String)>, //profil name <-> Id
    pub class_mapping: Vec<(ClassID, String)>,   //class name <-> Id
}

#[derive(Serialize, Deserialize)]
pub struct Class {
    pub name: String,
    pub people: Vec<String>,
}
