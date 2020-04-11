use std::collections::HashMap;
use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use tokio::sync::{mpsc, Mutex};

use serde::{Serialize, Deserialize};
use serde_json::json;

//use rusqlite::{params, Connection, Result};
use warp::ws::{Message, WebSocket};
use warp::{Filter, http::Response, http::response::Builder};

mod user;
use user::*;

struct OnlineUser {
    user: User,
    wstx: mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>
}

impl OnlineUser {
    fn send_text(&self, msg: String) -> Result<(), tokio::sync::mpsc::error::SendError<Result<warp::filters::ws::Message, warp::Error>>>{
        let msg = Message::text(msg);
        let msg = Ok(msg);
        self.wstx.send(msg)
    }
}

type Users = Arc<Mutex<HashMap<UserId, OnlineUser>>>;

#[derive(Serialize, Deserialize, Debug)]
struct UserConnectionRequest {
    name: String
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    println!("Hello, world!");
    
    let current_exe = std::env::current_exe().expect("Executable Location");
    let working_dir = current_exe.parent().expect("Working Directory");
    println!("Working Directory: {:?}", working_dir);
    
    let _conn = rusqlite::Connection::open(working_dir.join("db.sqlite")).expect("Failed to start SQLite!");
    
    let users = Arc::new(Mutex::new(HashMap::new()));
    let users = warp::any().map(move || users.clone());
    
    let websocket = warp::path("websocket")
        .and(warp::path::end())
        .and(warp::query::<UserConnectionRequest>())
        .and(warp::ws())
        .and(users)
        .map(|ucr: UserConnectionRequest, ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |socket| user_connected(socket, ucr, users))
    });
    
    fn static_reply(content_type: &str, body: &'static str) -> Result<warp::http::Response<&'static str>, warp::http::Error> {
        Response::builder()
            .header("Content-type", content_type)
            .body(body)
    }
    
    let index_html = warp::path::end().map(|| static_reply("text/html", include_str!("www/index.html")));
    let index_css = warp::path!("index.css").map(|| static_reply("text/css", include_str!("www/index.css")));
    
    let js_require = warp::path!("js" / "require.min.js").map(|| static_reply("application/javascript", include_str!("www/js/require.min.js")));
    let js_showdown = warp::path!("js" / "showdown.min.js").map(|| static_reply("application/javascript", include_str!("www/js/showdown.min.js")));
    let js_index = warp::path!("js" / "index.js").map(|| static_reply("application/javascript", include_str!("www/js/index.js")));
    let js_index_map = warp::path!("js" / "index.js.map").map(|| static_reply("application/javascript", include_str!("www/js/index.js.map")));
    
    let routes = index_html
        .or(index_css)
        .or(js_require)
        .or(js_showdown)
        .or(js_index)
        .or(js_index_map)
        .or(websocket)
        //.or(warp::fs::dir(working_dir.join("www")))
    ;
    
    warp::serve(routes)
        .run(([0, 0, 0, 0], 6991))
        .await;
}

async fn user_connected(ws: WebSocket, ucr: UserConnectionRequest, users: Users) {
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
        "type": "user-list",
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
                    if let Some(msg) = obj.get("type").map(|v| v.as_str()).flatten() {
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

async fn user_message(my_id: UserId, msg: &str, users: &Users) {
    
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
    let msg = markdown_to_html(msg, &ComrakOptions::default());
    let msg = msg.trim_start_matches("<p>");
    let msg = msg.trim_end_matches("\\n");
    let msg = msg.trim_end_matches("</p>");
    
    let new_msg = json!({
        "type": "user.message",
        "user": my_id,
        "message": msg
    }).to_string();
    
    // New message from this user, send it to everyone else (except same uid)...
    //
    // We use `retain` instead of a for loop so that we can reap any user that
    // appears to have disconnected.
    for (&uid, online_user) in users.lock().await.iter_mut() {
        if my_id != uid {
            if let Err(_disconnected) = online_user.send_text(new_msg.clone()) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
}

async fn user_disconnected(my_id: UserId, users: &Users) {
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
