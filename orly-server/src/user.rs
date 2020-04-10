use serde::{Serialize, Deserialize};

pub type UserId = uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub uuid: UserId,
    pub name: String,
}
