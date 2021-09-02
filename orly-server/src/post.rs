use uuid::Uuid;

use crate::websocket::{OrlyMessage, format_message};

pub type PostTime = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone)]
pub struct Post {
    pub id: u64,
    pub time: PostTime,
    pub view: String,
    pub user_id: u64,
    pub user_uuid: Uuid,
    pub content: String,
}

impl Post {
    
    pub fn load_posts(database: &rusqlite::Connection) -> Result<slice_deque::SliceDeque<Post>, rusqlite::Error> {
        
        let mut stmt = database.prepare("
            select *
            from (
                select *
                from messages
                order by id DESC
                limit 100
            )
            order by id ASC
        ")?;
        
        let posts = stmt.query_map([], |row| {
            Ok(Post {
                id: row.get(0)?,
                time: row.get(1)?,
                view: row.get(2)?,
                user_id: row.get(3)?,
                user_uuid: row.get(4)?,
                content: row.get(5)?,
            })
        })?;
        
        let mut out = slice_deque::SliceDeque::with_capacity(4096);
        
        for post in posts {
            let post = post.unwrap();
            out.push_front(post);
        }
        
        println!("Loaded {} posts...", out.len());
        
        Ok(out)
    }
    
}

impl From<&Post> for OrlyMessage<'_> {
    fn from(post: &Post) -> Self {
        let message = format_message(&post.content).expect("formatting error");
        
        Self::ChannelBroadcastFormatted {
            message,
            view: post.view.clone(),
            user: post.user_uuid,
        }
    }
}
