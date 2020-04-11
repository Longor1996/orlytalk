use std::collections::HashMap;
use std::sync::Arc;

use futures::{FutureExt, StreamExt};
use tokio::sync::{mpsc, Mutex};

use serde::{Serialize, Deserialize};
use serde_json::json;

//use rusqlite::{params, Connection, Result};
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod user;
use user::*;

struct OnlineUser {
    user: User,
    wsrx: mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>
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
    
    //let conn = Connection::open("db.sqlite").expect("Failed to start SQLite!");
    
    let users = Arc::new(Mutex::new(HashMap::new()));
    let users = warp::any().map(move || users.clone());
    
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String)
        .map(|name| format!("Hello, {}!", name));
    
    let websocket = warp::path("websocket")
        .and(warp::path::end())
        .and(warp::query::<UserConnectionRequest>())
        .and(warp::ws())
        .and(users)
        .map(|ucr: UserConnectionRequest, ws: warp::ws::Ws, users| {
            ws.on_upgrade(move |socket| user_connected(socket, ucr, users))
    });
    
    let index = warp::path::end().map(|| warp::reply::html(include_str!("www/index.html")));
    
    let showdown = warp::path!("showdown.min.js")
        .map(|| warp::reply::html(include_str!("www/showdown.min.js")));
    
    let routes = index
        .or(showdown)
        .or(websocket)
        .or(hello)
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
        wsrx: tx
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
            if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(user_connect_msg.clone()))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
    
    let user_list: Vec<User> = users.lock().await.iter().map(|(uuid, user)| user.user.clone()).collect();
    let user_list_msg = json!({
        "type": "user-list",
        "users": user_list
    }).to_string();
    
    if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(user_list_msg))) {
        // The tx is disconnected, our `user_disconnected` code
        // should be happening in another task, nothing more to
        // do here.
    }
    
    // Save the sender in our list of connected users.
    users.lock().await.insert(my_id, online_user);
    
    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.
    
    // Make an extra clone to give to our disconnection handler...
    let users2 = users.clone();
    
    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error(uid={}): {}", my_id, e);
                break;
            }
        };
        user_message(my_id, msg, &users).await;
    }
    
    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    user_disconnected(my_id, &users2).await;
}

async fn user_message(my_id: UserId, msg: Message, users: &Users) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s.trim()
    } else {
        return;
    };
    
    if msg.len() == 0 {
        println!("User #{} sent empty message.", my_id);
        if let Some(online_user) = users.lock().await.get(&my_id) {
            let err = json!({
                "type": "user.message.error",
                "error": "message too empty"
            }).to_string();
            if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(err))) {
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
            if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(err))) {
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
            if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(new_msg.clone()))) {
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
            if let Err(_disconnected) = online_user.wsrx.send(Ok(Message::text(user_disconnect_msg.clone()))) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }

    // Stream closed up, so remove from the user list
    users.lock().await.remove(&my_id);
}
