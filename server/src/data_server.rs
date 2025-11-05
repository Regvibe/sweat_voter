use crate::data_server::mutation_tracker::MutationTracker;
use crate::data_server::permissions::{InteractionPermission, Permissions};
use crate::data_server::ServerError::{
    ClassAlreadyExist, ClassDoesntExist, PersonAlreadyExist, PersonDoesntExist,
};
use common::packets::s2c;
use common::{ClassID, Identity, ProfilID};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::hash::RandomState;

pub mod compat;
pub mod mutation_tracker;
pub mod permissions;
pub mod serialization;

pub struct Profil {
    identity: Identity,
    permissions: Permissions,
}

#[derive(Clone, Debug)]
pub enum ServerError {
    PersonDoesntExist,
    ClassDoesntExist,
    PersonAlreadyExist,
    ClassAlreadyExist,
}

impl Display for ServerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PersonDoesntExist => f.write_str("This person does not exist"),
            ClassDoesntExist => f.write_str("This class does not exist"),
            PersonAlreadyExist => f.write_str("This person already exists"),
            ClassAlreadyExist => f.write_str("This class already exists"),
        }
    }
}

impl Error for ServerError {}

#[derive(Debug)]
pub struct Class {
    name: String,
    profiles: HashSet<ProfilID>,
}

/// A single Nickname proposition
#[derive(Serialize, Deserialize, Clone)]
pub struct NickNameProposition {
    author: ProfilID,
    proposition: String,
    votes: Vec<ProfilID>,
    protected: bool,
}

/// Global storage of most of the server content
pub struct DataServer {
    id_to_profil: MutationTracker<HashMap<ProfilID, Profil>>,
    free_profil_id_beginning: u32,
    name_to_id: MutationTracker<HashMap<String, ProfilID>>,
    classes: MutationTracker<HashMap<ClassID, Class>>,
    free_class_id_beginning: u32,
    nick_name_proposition: MutationTracker<HashMap<ProfilID, Vec<NickNameProposition>>>,
}

impl DataServer {
    /// Since data storage on disk and in ram are really different, this function is really long, most of the code is wrapping things together
    pub fn new(
        repartition: serialization::PeopleRepartition,
        id_map: serialization::IdMap,
    ) -> Self {
        let serialization::IdMap {
            profil_mapping,
            class_mapping,
        } = id_map;

        // process profil loading,
        let mut last_profil_id_used = profil_mapping
            .iter()
            .fold(0, |acc, (ProfilID(x), _)| u32::max(*x, acc));
        let mut raw_name_to_id_map: HashMap<_, _, RandomState> =
            HashMap::from_iter(profil_mapping.into_iter().map(|(id, name)| (name, id)));
        // this is a bit tricky to use since I want to reuse the same function for class building
        let mut get_profil_id = |name| {
            *raw_name_to_id_map.entry(name).or_insert_with(|| {
                last_profil_id_used += 1;
                ProfilID(last_profil_id_used)
            })
        };

        let profil_iter = repartition.profiles.into_iter().map(
            |serialization::Profil {
                 identity,
                 permissions,
             }| {
                (
                    get_profil_id(identity.name.clone()),
                    Profil {
                        identity,
                        permissions,
                    },
                )
            },
        );

        let id_to_profil = HashMap::from_iter(profil_iter);
        let name_to_id = HashMap::from_iter(
            id_to_profil
                .iter()
                .map(|(id, profil)| (profil.identity.name.clone(), *id)),
        );
        let _ = get_profil_id;

        //class loading
        let mut last_class_id_used = class_mapping
            .iter()
            .fold(0, |acc, (ClassID(x), _)| u32::max(*x, acc));
        let raw_class_name_to_id_map: HashMap<_, _, RandomState> =
            HashMap::from_iter(class_mapping.into_iter().map(|(id, name)| (name, id)));
        let mut get_class_id = |name: &String| {
            raw_class_name_to_id_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| {
                    last_class_id_used += 1;
                    ClassID(last_class_id_used)
                })
        };

        let class_iter =
            repartition
                .classes
                .into_iter()
                .map(|serialization::Class { name, people }| {
                    (
                        get_class_id(&name),
                        Class {
                            name,
                            profiles: HashSet::from_iter(people.iter().flat_map(|person_name| {
                                raw_name_to_id_map.get(person_name).cloned()
                            })),
                        },
                    )
                });

        let classes = HashMap::from_iter(class_iter);

        Self {
            id_to_profil: MutationTracker::new(id_to_profil),
            free_profil_id_beginning: last_profil_id_used,
            name_to_id: MutationTracker::new(name_to_id),
            classes: MutationTracker::new(classes),
            free_class_id_beginning: last_class_id_used,
            nick_name_proposition: Default::default(),
        }
    }

    // It kinda hurt to look at, but it's really straightforward: a bunch of map to correctly cast data
    pub fn build_id_map(&self) -> serialization::IdMap {
        let profil_mapping = self
            .id_to_profil
            .iter()
            .map(|(id, profil)| (*id, profil.identity.name.clone()))
            .collect();
        let class_mapping = self
            .classes
            .iter()
            .map(|(id, class)| (*id, class.name.clone()))
            .collect();
        serialization::IdMap {
            profil_mapping,
            class_mapping,
        }
    }

    pub fn build_people_repartition(&self) -> serialization::PeopleRepartition {
        let mut profiles: Vec<_> = self
            .id_to_profil
            .values()
            .map(|profil| serialization::Profil {
                identity: profil.identity.clone(),
                permissions: profil.permissions,
            })
            .collect();

        profiles.sort_by(|a, b| a.identity.name.cmp(&b.identity.name));

        let mut classes: Vec<_> = self
            .classes
            .values()
            .map(|class| serialization::Class {
                name: class.name.clone(),
                people: class
                    .profiles
                    .iter()
                    .flat_map(|id| {
                        self.id_to_profil
                            .get(id)
                            .map(|profil| profil.identity.name.clone())
                    })
                    .collect(),
            })
            .collect();

        classes.sort_by(|a, b| a.name.cmp(&b.name));

        serialization::PeopleRepartition { profiles, classes }
    }

    pub fn load_proposition(
        &mut self,
        nick_name_proposition: HashMap<ProfilID, Vec<NickNameProposition>>,
    ) {
        self.nick_name_proposition = MutationTracker::new(nick_name_proposition)
    }

    pub fn import_old_nickname(&mut self, group: compat::Group) {
        for (name, (_, old_nicknames)) in group.profiles {
            if old_nicknames.is_empty() {
                continue;
            }
            let Some(id) = self.name_to_id.get(&name).cloned() else {
                continue;
            };
            let nicknames = self.nick_name_proposition.entry(id).or_insert(vec![]);

            nicknames.extend(old_nicknames.into_iter().flat_map(
                |compat::Nickname {
                     nickname: proposition,
                     votes,
                 }| {
                    let votes: Vec<_> = votes
                        .into_iter()
                        .flat_map(|voter| self.name_to_id.get(&voter).cloned())
                        .collect();
                    // take the first voter as the owner, else the person that will receive the nickname
                    let author = votes.first().cloned().unwrap_or(id);
                    Some(NickNameProposition {
                        author,
                        proposition,
                        votes,
                        protected: false,
                    })
                },
            ))
        }
    }

    pub fn try_to_save_nickname(&mut self) -> Option<HashMap<ProfilID, Vec<NickNameProposition>>> {
        if self.nick_name_proposition.clear_dirty() {
            Some(self.nick_name_proposition.clone())
        } else {
            None
        }
    }

    pub fn try_to_save_profils(
        &mut self,
    ) -> Option<(serialization::PeopleRepartition, serialization::IdMap)> {
        if self.id_to_profil.clear_dirty()
            || self.name_to_id.clear_dirty()
            || self.classes.clear_dirty()
        {
            let repartition = self.build_people_repartition();
            let id_map = self.build_id_map();
            Some((repartition, id_map))
        } else {
            None
        }
    }

    pub fn add_profile(&mut self, name: String, password: String) -> Result<(), ServerError> {
        let entry = self.name_to_id.entry(name.clone());
        if let Occupied(_) = entry {
            return Err(PersonAlreadyExist);
        }

        self.free_profil_id_beginning += 1;
        let id = ProfilID(self.free_profil_id_beginning);
        entry.insert_entry(id);
        let _ = entry;
        self.id_to_profil.insert(
            id,
            Profil {
                identity: Identity { name, password },
                permissions: Default::default(),
            },
        );
        Ok(())
    }

    pub fn delete_profil(&mut self, profil: String) -> Result<(), ServerError> {
        let removed = self.name_to_id.remove(&profil).ok_or(PersonDoesntExist)?;

        self.id_to_profil.remove(&removed);

        // small optimisation to reduced unused id overhead
        if removed.0 == self.free_profil_id_beginning {
            self.free_profil_id_beginning -= 1;
        }

        self.nick_name_proposition.remove(&removed);
        for propositions in self.nick_name_proposition.values_mut() {
            for proposition in propositions {
                proposition.votes.retain(|voter| voter != &removed);
            }
        }

        Ok(())
    }

    pub fn add_class(&mut self, name: String) -> Result<(), ServerError> {
        let entry = self.classes.values().find(|class| class.name == name);

        if entry.is_some() {
            return Err(ClassAlreadyExist);
        }

        self.free_class_id_beginning += 1;
        let id = ClassID(self.free_profil_id_beginning);
        self.classes.insert(
            id,
            Class {
                name,
                profiles: HashSet::new(),
            },
        );
        Ok(())
    }

    pub fn delete_class(&mut self, name: String) -> Result<(), ServerError> {
        let id = self
            .classes
            .iter()
            .find_map(|(id, class)| (*class.name == name).then_some(*id))
            .ok_or(ClassDoesntExist)?;
        self.classes.remove(&id);

        if id.0 == self.free_class_id_beginning {
            self.free_class_id_beginning -= 1;
        }
        Ok(())
    }

    pub fn find_people_out_of_any_class(&self) -> Vec<String> {
        let mut people = vec![];
        'outer: for (id, profil) in self.id_to_profil.iter() {
            for class in self.classes.values() {
                if class.profiles.contains(id) {
                    continue 'outer;
                }
            }
            people.push(profil.identity.name.clone());
        }
        people
    }

    pub fn get_password(&self, id: ProfilID) -> Result<String, ServerError> {
        let profil = self.id_to_profil.get(&id).ok_or(PersonDoesntExist)?;
        Ok(profil.identity.password.clone())
    }

    pub fn change_password(
        &mut self,
        id: ProfilID,
        new_password: String,
    ) -> Result<(), ServerError> {
        let profil = self.id_to_profil.get_mut(&id).ok_or(PersonDoesntExist)?;
        profil.identity.password = new_password;
        Ok(())
    }

    pub fn get_permissions_mut(&mut self, id: ProfilID) -> Result<&mut Permissions, ServerError> {
        self.id_to_profil
            .get_mut(&id)
            .map(|v| &mut v.permissions)
            .ok_or(PersonDoesntExist)
    }

    pub fn change_name(&mut self, old_name: String, new_name: String) -> Result<(), ServerError> {
        let id = self.name_to_id.remove(&old_name).ok_or(PersonDoesntExist)?;
        self.name_to_id.insert(new_name.clone(), id);
        let profil = self.id_to_profil.get_mut(&id).ok_or(PersonDoesntExist)?;
        profil.identity.name = new_name;
        Ok(())
    }

    pub fn add_to_class(
        &mut self,
        profil_id: ProfilID,
        class_name: String,
    ) -> Result<(), ServerError> {
        let (_, class) = self
            .classes
            .iter_mut()
            .find(|(_, class)| *class.name == class_name)
            .ok_or(ClassDoesntExist)?;
        if class.profiles.insert(profil_id) {
            Ok(())
        } else {
            Err(PersonAlreadyExist)
        }
    }

    pub fn remove_from_class(
        &mut self,
        profil_id: ProfilID,
        class_name: String,
    ) -> Result<(), ServerError> {
        let (_, class) = self
            .classes
            .iter_mut()
            .find(|(_, class)| *class.name == class_name)
            .ok_or(ClassDoesntExist)?;
        if class.profiles.remove(&profil_id) {
            Ok(())
        } else {
            Err(PersonDoesntExist)
        }
    }

    /// check if two profils share the same class
    pub fn are_in_same_class(&self, a: ProfilID, b: ProfilID) -> bool {
        for (_, class) in self.classes.iter() {
            if class.profiles.contains(&a) && class.profiles.contains(&b) {
                return true;
            }
        }
        false
    }

    pub fn is_action_allowed_between(
        &self,
        interaction_permission: InteractionPermission,
        editor: ProfilID,
        target: ProfilID,
    ) -> bool {
        match interaction_permission {
            InteractionPermission::Forbidden => false,
            InteractionPermission::YourSelf => editor == target,
            InteractionPermission::SameClass => self.are_in_same_class(editor, target),
            InteractionPermission::AnyBody => true,
        }
    }

    pub fn get_permission(&self, profil_id: ProfilID) -> Option<Permissions> {
        self.id_to_profil
            .get(&profil_id)
            .map(|profil| profil.permissions)
    }

    /// voting and adding a nickname is the same operation, if the voter or target doesn't exist, it simply does nothing
    pub fn vote(&mut self, voter: ProfilID, target: ProfilID, proposition: String) {
        let Some(permissions) = self.get_permission(voter) else {
            return;
        };
        if !self.is_action_allowed_between(permissions.vote, voter, target) {
            return;
        };

        let proposition = proposition.trim().to_string();
        if proposition.is_empty() {
            return;
        };
        let nicknames = match self.nick_name_proposition.entry(target) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) if self.id_to_profil.contains_key(&target) => entry.insert(vec![]),
            _ => return,
        };

        let mut found = false;
        for nickname in nicknames.iter_mut() {
            nickname.votes.retain(|p| *p != voter);
            if nickname.proposition == proposition {
                found = true;
                nickname.votes.push(voter)
            }
        }
        if !found {
            nicknames.push(NickNameProposition {
                author: voter,
                proposition,
                votes: vec![voter],
                protected: false,
            })
        }
    }

    /// Attempt to perform a delete operation
    pub fn delete(&mut self, deleter: ProfilID, target: ProfilID, nickname: String) {
        let Some(permissions) = self.get_permission(deleter) else {
            return;
        };
        let is_allowed_to_delete =
            self.is_action_allowed_between(permissions.delete, deleter, target);
        let can_by_pass_protect =
            self.is_action_allowed_between(permissions.protect_nickname, deleter, target);

        let Some(nicknames) = self.nick_name_proposition.get_mut(&target) else {
            return;
        };
        let Some(i) = nicknames.iter().position(|n| *n.proposition == nickname) else {
            return;
        };

        if (is_allowed_to_delete || nicknames[i].author == deleter)
            && (!nicknames[i].protected || can_by_pass_protect)
        {
            nicknames.swap_remove(i);
        }
    }

    /// Attempt to protect a nickname proposition
    pub fn update_nickname_protection(
        &mut self,
        admin: ProfilID,
        target: ProfilID,
        nickname: String,
        new_statut: bool,
    ) {
        let Some(permissions) = self.get_permission(admin) else {
            return;
        };

        if !self.is_action_allowed_between(permissions.protect_nickname, admin, target) {
            return;
        }

        let Some(nicknames) = self.nick_name_proposition.get_mut(&target) else {
            return;
        };
        let Some(i) = nicknames.iter().position(|n| *n.proposition == nickname) else {
            return;
        };
        nicknames[i].protected = new_statut;
    }

    /// Return if a user can log
    pub fn log(&self, identity: &Identity) -> bool {
        let Identity { name, password } = identity;
        self.name_to_id.get(name).is_some_and(|id| {
            self.id_to_profil
                .get(id)
                .is_some_and(|profil| profil.identity.password == *password)
        })
    }

    pub fn get_profil_id(&self, name: &String) -> Result<ProfilID, ServerError> {
        self.name_to_id.get(name).cloned().ok_or(PersonDoesntExist)
    }

    //------------ Network related functions ------------

    /// build the list of classes
    pub fn class_list(&self) -> s2c::ClassList {
        let vec: Vec<_> = self
            .classes
            .iter()
            .map(|(id, class)| {
                (
                    *id,
                    s2c::Class {
                        name: class.name.clone(),
                        profiles: class
                            .profiles
                            .iter()
                            .map(|profil| {
                                (
                                    *profil,
                                    self.id_to_profil.get(profil).unwrap().identity.name.clone(),
                                )
                            })
                            .collect(),
                    },
                )
            })
            .collect();
        s2c::ClassList { classes: vec }
    }

    /// return if a person can vote, delete and bypass protection, and can delete your proposition on which you are the author
    pub fn get_permission_on_profil(
        &self,
        requester: ProfilID,
        asked_profil: ProfilID,
    ) -> (bool, bool, bool) {
        let Some(permission) = self.get_permission(requester) else {
            return (false, false, false);
        };
        (
            self.is_action_allowed_between(permission.vote, requester, asked_profil),
            self.is_action_allowed_between(permission.delete, requester, asked_profil),
            self.is_action_allowed_between(permission.protect_nickname, requester, asked_profil),
        )
    }

    /// build a packet for a given identity
    pub fn personne_profil(
        &self,
        requester: Option<ProfilID>,
        asked_profil: ProfilID,
    ) -> s2c::Profile {
        let (allowed_to_vote, allowed_to_delete, allowed_to_protect) = requester
            .map(|r| self.get_permission_on_profil(r, asked_profil))
            .unwrap_or((false, false, false));

        let nicknames = self.nick_name_proposition.get(&asked_profil);
        let nicknames = match nicknames {
            None => vec![],
            Some(propositions) => propositions
                .iter()
                .map(|proposition| s2c::NicknameStatut {
                    proposition: proposition.proposition.clone(),
                    count: proposition.votes.len(),
                    contain_you: requester
                        .is_some_and(|requester| proposition.votes.contains(&requester)),
                    allowed_to_be_delete: (allowed_to_delete
                        || requester.is_some_and(|r| r == proposition.author))
                        && (!proposition.protected || allowed_to_protect),
                    protected: proposition.protected,
                })
                .collect(),
        };

        s2c::Profile {
            profil_id: asked_profil,
            nicknames,
            allowed_to_vote,
            allowed_to_protect,
        }
    }
}
