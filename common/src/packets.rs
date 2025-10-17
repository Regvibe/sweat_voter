pub mod c2s {
    use crate::{Identity, ProfilID};
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Login {
        pub identity: Identity
    }

    /// Voting and adding a nickname is the same operation
    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct VoteNickname {
        pub target: ProfilID,
        pub nickname: String, // which nickname to vote for
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct DeleteNickname {
        pub target: ProfilID, // the owner of the nickname
        pub nickname: String, // which nickname to delete
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct AskForPersonProfil {
        pub profil: ProfilID,
    }
}
pub mod s2c {
    use crate::{ClassID, ProfilID};
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Class {
        pub name: String,
        pub profiles: Vec<(ProfilID, String)>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct ClassList {
        pub classes: Vec<(ClassID, Class)>,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct NicknameStatut {
        pub proposition: String,
        pub count: usize,
        pub contain_you: bool,
        pub allowed_to_be_delete: bool,
    }

    #[derive(Deserialize, Serialize, Debug, Clone)]
    pub struct Profile {
        pub profil_id: ProfilID,
        pub nicknames: Vec<NicknameStatut>,
        pub allowed_to_vote: bool,
    }
}
