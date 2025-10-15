pub mod packets;

use serde::{Deserialize, Serialize};

/// A shortcut for a profil, this can be used publicly,
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct ProfilID(pub u32);

/// A shortcut for a profil, this can be used publicly,
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct ClassID(pub u32);

#[derive(Deserialize, Serialize, Debug, Clone, Default, Hash, Eq, PartialEq)]
/// Used to log in
pub struct Identity {
    pub name: String,
    pub password: String,
}
