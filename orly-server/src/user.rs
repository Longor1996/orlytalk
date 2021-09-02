use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub type UserId = Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub name: String,
}

impl User {
    
    pub fn load_users(database: &rusqlite::Connection) -> Result<DashMap<UserId, User>, rusqlite::Error> {
        
        let mut stmt = database.prepare("
            select * from users
        ")?;
        
        let users = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(1)?,
                name: row.get(2)?,
            })
        })?;
        
        let out = DashMap::default();
        
        for user in users {
            let user = user.unwrap();
            out.insert(user.id, user);
        }
        
        println!("Loaded {} users...", out.len());
        
        Ok(out)
    }
    
}