use crate::data_server::permissions::InteractionPermission;
use structopt::clap::arg_enum;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Add a profil")]
pub struct AddProfil {
    pub name: String,
    pub password: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Remove a profil")]
pub struct DeleteProfil {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "create a new class")]
pub struct AddClass {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "remove a class class")]
pub struct DeleteClass {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "view someone password")]
pub struct ViewPassword {
    pub name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "change someone password")]
pub struct ChangePassword {
    pub name: String,
    pub new_password: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "change someone name")]
pub struct ChangeName {
    pub name: String,
    pub new_name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "add someone to class")]
pub struct AddToClass {
    pub profil_name: String,
    pub class_name: String,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "add remove to class")]
pub struct RemoveFromClass {
    pub profil_name: String,
    pub class_name: String,
}

arg_enum! {
    #[derive(Debug)]
    pub enum PermissionKind {
        Vote,
        Delete,
        Protect,
    }
}

#[derive(Debug, StructOpt)]
#[structopt(about = "change permission to class")]
pub struct ChangePermission {
    pub name: String,
    pub kind: PermissionKind,
    pub permission: InteractionPermission,
}
