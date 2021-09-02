use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub type UserId = Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
}
