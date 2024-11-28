
pub mod c2s {
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AddNickname {
        pub class: String,
        pub editor: String,
        pub password: String,
        pub name: String,
        pub nickname: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DeleteNickname {
        pub class: String,
        pub editor: String,
        pub password: String,
        pub nickname: String,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteNickname {
        pub class: String,
        pub name: String,
        pub nickname: String,
        pub voter: String,
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
    use std::collections::BTreeMap;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct ClassList {
        pub names: Vec<String>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteCount {
        pub count: usize,
        pub contain_you: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct PersonProfileResponse {
        pub partial_response: bool,
        pub allowed_to_modify: bool,
        pub profiles: BTreeMap<String, BTreeMap<String, VoteCount>>,
    }

    impl Default for PersonProfileResponse {
        fn default() -> Self {
            Self {
                partial_response: false,
                allowed_to_modify: false,
                profiles: BTreeMap::new(),
            }
        }
    }
}