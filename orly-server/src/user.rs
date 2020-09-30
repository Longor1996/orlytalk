use serde::{Serialize, Deserialize};

pub type UserId = u64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
}
