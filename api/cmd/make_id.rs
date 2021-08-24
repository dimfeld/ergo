use std::fmt::Display;

use ergo_database::object_id::*;
use structopt::StructOpt;

use crate::error::Error;

#[derive(Debug, StructOpt)]
pub struct ToUuidArgs {
    value: String,
}

#[derive(Debug, StructOpt)]
pub enum ObjectIdType {
    Task,
    Org,
    Role,
    User,
    Input,
    Action,
    InputCategory,
    ActionCategory,
    Account,
    TaskTrigger,
    TaskTemplate,
}

#[derive(Debug, StructOpt)]
pub enum Args {
    #[structopt(about = "Create a new object ID")]
    New(ObjectIdType),
    #[structopt(about = "Decode an ID to its UUID form")]
    ToUuid(ToUuidArgs),
}

pub async fn main(args: Args) -> Result<(), Error> {
    match args {
        Args::New(t) => {
            let id: Box<dyn Display> = match t {
                ObjectIdType::Task => Box::new(TaskId::new()),
                ObjectIdType::Org => Box::new(OrgId::new()),
                ObjectIdType::Role => Box::new(RoleId::new()),
                ObjectIdType::User => Box::new(UserId::new()),
                ObjectIdType::Input => Box::new(InputId::new()),
                ObjectIdType::Action => Box::new(ActionId::new()),
                ObjectIdType::InputCategory => Box::new(InputCategoryId::new()),
                ObjectIdType::ActionCategory => Box::new(ActionCategoryId::new()),
                ObjectIdType::Account => Box::new(AccountId::new()),
                ObjectIdType::TaskTrigger => Box::new(TaskTriggerId::new()),
                ObjectIdType::TaskTemplate => Box::new(TaskTemplateId::new()),
            };
            println!("{}", id);
        }
        Args::ToUuid(ToUuidArgs { value }) => {
            let id = decode_suffix(&value[value.len() - 22..])
                .map_err(|e| Error::StringError(e.to_string()))?;
            println!("{}", id);
        }
    };
    Ok(())
}
