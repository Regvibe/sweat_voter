
pub mod packets;

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Nickname {
    pub nickname: String,
    pub votes: Vec<String>,
}

impl Default for Nickname {
    fn default() -> Self {
        Self {
            nickname: "template nickname".to_string(),
            votes: Vec::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Group {
    pub profiles: BTreeMap<String, (String, Vec<Nickname>)>,
}

/*impl Default for Group {
    fn default() -> Self {
        let mut names = BTreeMap::new();
        names.insert("template 1".to_string(), vec![Nickname::default()]);
        names.insert("template 2".to_string(), vec![Nickname::default()]);
        Self { names }
    }
}*/



