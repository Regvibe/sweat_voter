use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "add_profil", about = "Add a profil")]
pub struct AddProfil {
    #[structopt(short, long)]
    profil: String,
}
