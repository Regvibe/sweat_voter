
pub mod packets;

use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AdminList {
    pub admins: HashSet<String>,
}

