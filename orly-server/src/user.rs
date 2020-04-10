use serde::{Serialize, Deserialize};

pub type UserId = uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub uuid: UserId,
    pub name: String,
}
