use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// THIS FILE WILL BE DEPRECATED, IT ONLY HERE FOR RETRO COMPATIBILITY

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Nickname {
    pub nickname: String,
    pub votes: Vec<String>,
}

/// structure that represent a "class" of people, with their nicknames
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Group {
    // name, (password, nicknames)
    pub profiles: BTreeMap<String, (String, Vec<Nickname>)>,
}
