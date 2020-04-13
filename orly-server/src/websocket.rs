use warp::ws::{Message, WebSocket};

use serde::{Serialize, Deserialize};
use serde_json::json;

use futures::{FutureExt, StreamExt};
use tokio::sync::{mpsc, Mutex};

use crate::{UserId, User};

#[derive(Serialize, Deserialize, Debug)]
pub struct UserConnectionRequest {
    name: String
}

pub struct OnlineUser {
    pub user: User,
    pub wstx: mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>
}

impl OnlineUser {
    pub fn send_text(&self, msg: String) -> Result<(), tokio::sync::mpsc::error::SendError<Result<warp::filters::ws::Message, warp::Error>>>{
        let msg = Message::text(msg);
        let msg = Ok(msg);
        self.wstx.send(msg)
    }
}

pub type Users = std::sync::Arc<Mutex<std::collections::HashMap<UserId, OnlineUser>>>;

pub async fn user_connected(ws: WebSocket, ucr: UserConnectionRequest, users: Users) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = uuid::Uuid::new_v4();
    
    eprintln!("new chat user: {}", my_id);
    
    // Split the socket into a sender and receive of messages.
    let (user_ws_tx, mut user_ws_rx) = ws.split();
    
    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));
    
    let user = User {
        uuid: my_id,
        name: ucr.name.clone(),
    };
    
    let online_user = OnlineUser {
        user: user,
        wstx: tx
    };
    
    let user_self_msg = json!({
        "type": "user-info.self",
        "user": online_user.user
    }).to_string();
    
    if let Err(_disconnected) = online_user.send_text(user_self_msg) {
        return;
    }
    
    let user_connect_msg = json!({
        "type": "user.join",
        "user": {
            "uuid": online_user.user.uuid,
            "name": online_user.user.name
        }
    }).to_string();
    
    for (&uid, online_user) in users.lock().await.iter_mut() {
        if my_id != uid {
            if let Err(_disconnected) = online_user.send_text(user_connect_msg.clone()) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
    
    let user_list: Vec<User> = users.lock().await.iter().map(|(_, user)| user.user.clone()).collect();
    let user_list_msg = json!({
        "type": "user-info.list",
        "users": user_list
    }).to_string();
    
    if let Err(_disconnected) = online_user.send_text(user_list_msg) {
        return;
    }
    
    // Save the sender in our list of connected users.
    users.lock().await.insert(my_id, online_user);
    
    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();
    
    // Process messages coming from the user...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        
        if let Ok(msg) = msg.to_str() {
            let json = match serde_json::from_str::<serde_json::Value>(msg) {
                Ok(json) => json,
                Err(err) => {
                    eprintln!("User packet json could not parse: {}", err);
                    break;
                }
            };
            
            let obj = match json {
                serde_json::Value::Object(obj) => obj,
                _ => {
                    eprintln!("User packet json not object.");
                    break;
                }
            };
            
            let msg_type = match obj.get("type").map(|v| v.as_str()).flatten() {
                Some(v) => v,
                None => {
                    eprintln!("User packet is invalid: No type.");
                    break;
                },
            };
            
            println!("User packet: {:?}", obj);
            
            match msg_type {
                "user.message" => {
                    if let Some(msg) = obj.get("message").map(|v| v.as_str()).flatten() {
                        user_message(my_id, msg, &users).await;
                    } else {
                        eprintln!("User message packet is invalid: No message given.");
                        break;
                    }
                },
                _ => {
                    eprintln!("User packet is invalid: Unknown type -> {}", msg_type);
                    break;
                }
            }
        }
    }
    
    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users2).await;
}

pub async fn user_message(my_id: UserId, msg: &str, users: &Users) {
    
    if msg.len() == 0 {
        println!("User #{} sent empty message.", my_id);
        if let Some(online_user) = users.lock().await.get(&my_id) {
            let err = json!({
                "type": "user.message.error",
                "error": "message too empty"
            }).to_string();
            if let Err(_disconnected) = online_user.send_text(err) {
                // Nothing to do here.
            }
        }
        return;
    }
    
    if msg.len() > 1024 {
        println!("User #{} sent message that is too long.", my_id);
        if let Some(online_user) = users.lock().await.get(&my_id) {
            let err = json!({
                "type": "user.message.error",
                "error": "message too long"
            }).to_string();
            if let Err(_disconnected) = online_user.send_text(err) {
                // Nothing to do here.
            }
        }
        return;
    }
    
    println!("User #{} sent message.", my_id);
    
    use comrak::{markdown_to_html, ComrakOptions};
    let msg = markdown_to_html(&msg, &ComrakOptions::default());
    
    let new_msg = json!({
        "type": "user.message",
        "screen_id": "default",
        "user": my_id,
        "message": msg
    }).to_string();
    
    // New message from this user, send it to everyone else (except same uid)...
    //
    // We use `retain` instead of a for loop so that we can reap any user that
    // appears to have disconnected.
    for (&uid, online_user) in users.lock().await.iter_mut() {
        if let Err(_disconnected) = online_user.send_text(new_msg.clone()) {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
        }
    }
}

pub async fn user_disconnected(my_id: UserId, users: &Users) {
    eprintln!("good bye user: {}", my_id);
    
    let user_disconnect_msg = json!({
        "type": "user.leave",
        "user": my_id
    }).to_string();
    
    for (&uid, online_user) in users.lock().await.iter_mut() {
        if my_id != uid {
            if let Err(_disconnected) = online_user.send_text(user_disconnect_msg.clone()) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }

    // Stream closed up, so remove from the user list
    users.lock().await.remove(&my_id);
}
