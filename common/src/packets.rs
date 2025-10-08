pub mod c2s {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AddNickname {
        pub class: String,  // In what class the action is performed
        pub editor: String, // who is editing
        pub password: String,
        pub name: String,     // who will receive a new nickname
        pub nickname: String, // the proposition
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DeleteNickname {
        pub class: String,  // In what class the action is performed
        pub editor: String, // who is editing
        pub password: String,
        pub name: String,     // the owner of the nickname
        pub nickname: String, // which nickname to delete
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteNickname {
        pub class: String,    // In what class the action is performed
        pub name: String,     // the owner of the nickname
        pub nickname: String, // which nickname to vote for
        pub voter: String,    // who is voting
        pub password: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub enum RequestKind {
        All,
        Custom(Vec<String>),
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AskForPersonProfile {
        pub class: String,
        pub editor: String,
        pub password: String,
        pub kind: RequestKind,
    }
}
pub mod s2c {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct ClassList {
        pub names: Vec<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteCount {
        pub count: usize,
        pub contain_you: bool,
        pub voters: Vec<String>, //used be admin to see who voted
    }

    #[derive(Deserialize, Serialize, Debug, Copy, Clone)]
    pub struct Permissions {
        pub vote: bool,
        pub suggest: bool,
        pub delete_own: bool,
        pub delete_other: bool,
    }

    impl Permissions {
        pub const NONE: Self = Self {
            vote: false,
            suggest: false,
            delete_own: false,
            delete_other: false,
        };
        pub const ADMIN: Self = Self {
            vote: true,
            suggest: true,
            delete_own: true,
            delete_other: true,
        };
        pub const STANDARD: Self = Self {
            vote: true,
            suggest: true,
            delete_own: true,
            delete_other: false,
        };

        pub fn perm(edition: bool, admin: bool) -> Self {
            match (edition, admin) {
                (_, true) => Self::ADMIN,
                (true, false) => Self::STANDARD,
                (false, _) => Self::NONE,
            }
        }
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct PersonProfileResponse {
        pub should_overwrite: bool,
        pub permissions: Permissions,
        pub profiles: BTreeMap<String, BTreeMap<String, VoteCount>>,
    }

    impl Default for PersonProfileResponse {
        fn default() -> Self {
            Self {
                should_overwrite: true,
                permissions: Permissions::NONE,
                profiles: BTreeMap::new(),
            }
        }
    }
}
