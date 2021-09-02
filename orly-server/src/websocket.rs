use warp::ws::{Message, WebSocket};

use serde::{Serialize, Deserialize};
use serde_json::json;

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use dashmap::DashMap;

use crate::User;
pub type ClientId = u64;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserConnectionRequest {}

#[derive(Serialize, Debug)]
pub struct OnlineClient {
    pub id: ClientId,
    pub user: Option<User>,
    
    #[serde(skip)]
    pub wstx: mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>
}

impl OnlineClient {
    pub fn send_text<S: Into<String>>(&self, msg: S) -> Result<(), tokio::sync::mpsc::error::SendError<Result<warp::filters::ws::Message, warp::Error>>>{
        let msg = Message::text(msg);
        let msg = Ok(msg);
        self.wstx.send(msg)
    }
    
    pub fn send_binary<S: Into<Vec<u8>>>(&self, payload: S) -> Result<(), tokio::sync::mpsc::error::SendError<Result<warp::filters::ws::Message, warp::Error>>>{
        let msg = Message::binary(payload);
        let msg = Ok(msg);
        self.wstx.send(msg)
    }
}

pub type Clients = std::sync::Arc<DashMap<ClientId, OnlineClient>>;

pub async fn client_connected(ws: WebSocket, _ucr: UserConnectionRequest, clients: Clients) {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    lazy_static! {
        static ref CLIENT_ID_AUTO_INCREMENT: AtomicU64 = {
            AtomicU64::new(0)
        };
    };
    
    // Use a counter to assign a new unique ID for this user.
    let client_id = CLIENT_ID_AUTO_INCREMENT.fetch_add(1, Ordering::Relaxed);
    
    eprintln!("[Client {}] Connected!", &client_id);
    
    // Split the socket into a sender and receive of messages.
    let (client_send, mut client_recv) = ws.split();
    
    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let forward_id = client_id;
    let (tx, rx) = mpsc::unbounded_channel();
    
    tokio::task::spawn(async move {
        let mut rx = rx;
        let mut cs = client_send;
        
        while let Some(rx) = rx.recv().await {
            match rx {
                Ok(msg) => {
                    println!("[Client {}] WebSocket Send: {:?}", forward_id, &msg);
                    match cs.send(msg).await {
                        Ok(_) => continue,
                        Err(err) => eprintln!("[Client {}] WebSocket Send Error: {}", forward_id, err),
                    }
                },
                Err(err) => eprintln!("[Client {}] WebSocket Send Receiver Error: {}", forward_id, err)
            };
        }
    });
    
    let client = OnlineClient {
        id: client_id,
        user: None,
        wstx: tx
    };
    
    let login_acknowledgement_msg = json!({
        "type": "client-info.self",
        "client": client,
        //"user": client.user // TODO: Use cookies so we can load and send this immediately?
    }).to_string();
    
    if let Err(_disconnected) = client.send_text(login_acknowledgement_msg) {
        client_disconnected(client_id, &clients).await;
        return;
    }
    
    client_channel_broadcast_text(&json!({
        "type": "client.join",
        "client": client
    }).to_string(), &clients).await;
    
    let client_list: Vec<User> = clients.iter().filter_map(|multiref| multiref.value().user.clone()).collect();
    let client_list_msg = json!({
        "type": "client-info.list",
        "clients": client_list
    }).to_string();
    
    if let Err(_disconnected) = client.send_text(client_list_msg) {
        client_disconnected(client_id, &clients).await;
        return;
    }
    
    // Save the sender in our list of connected clients.
    clients.insert(client_id, client);
    
    // Make an extra clone to give to our disconnection handler...
    let clients_cpy = clients.clone();
    
    // Process messages coming from the client...
    while let Some(result) = client_recv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[Client {}] Websocket Error: {}", client_id, e);
                break;
            }
        };
        
        if msg.is_binary() {
            let msg = msg.as_bytes();
            
            let index: usize = match msg.iter()
                .enumerate()
                .find_map(|(i,b)| if *b == b':' {Some(i)} else {None}) {
                    Some(i) => i,
                    None => {
                        eprintln!("[Client {}] Message invalid; could not find end of preload.", client_id);
                        break;
                    }
                };
            
            let (preload, payload) = msg.split_at(index);
            
            let preload = match std::str::from_utf8(preload) {
                Ok(str) => str,
                Err(e) => {
                    eprintln!("[Client {}] Message invalid; preload is not valid UTF-8: {}", client_id, e);
                    break;
                }
            };
            
            let at_index = preload.find('@');
            
            if let Some(at) = at_index {
                let (msg_type, msg_target) = preload.split_at(at);
                eprintln!("[Client {}] Received Binary Message; type is '{}', target is '{}'.", client_id, msg_type, msg_target);
                
                if msg_type == "channel.broadcast" {
                    client_channel_broadcast_binary(payload, &clients).await;
                }
            } else {
                let msg_type = preload;
                eprintln!("[Client {}] Received Binary Message; type is '{}'.", client_id, msg_type);
            }
            
            continue;
        }
        
        if let Ok(msg) = msg.to_str() {
            let json = match serde_json::from_str::<serde_json::Value>(msg) {
                Ok(json) => json,
                Err(err) => {
                    eprintln!("[Client {}] Message invalid; failed to parse JSON: {}", client_id, err);
                    break;
                }
            };
            
            let obj = match json {
                serde_json::Value::Object(obj) => obj,
                _ => {
                    eprintln!("[Client {}] Message invalid; JSON root is not an object.", client_id);
                    break;
                }
            };
            
            let msg_type = match obj.get("type").map(|v| v.as_str()).flatten() {
                Some(v) => v,
                None => {
                    eprintln!("[Client {}] Message invalid; no message type given.", client_id);
                    break;
                },
            };
            
            println!("[Client {}] Received JSON Message: {:?}", client_id, obj);
            
            match msg_type {
                "channel.broadcast.formatted" => {
                    if let Some(msg) = obj.get("message").map(|v| v.as_str()).flatten() {
                        client_channel_broadcast_formatted(client_id, msg, &clients).await;
                        continue;
                    } else {
                        eprintln!("[Client {}] User message packet is invalid: No message given.", client_id);
                        break;
                    }
                },
                "channel.broadcast" => {
                    client_channel_broadcast_text(msg, &clients).await;
                },
                _ => {
                    eprintln!("[Client {}] User packet is invalid: Unknown type -> {}", client_id, msg_type);
                    break;
                }
            }
        }
    }
    
    // client_recv stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    client_disconnected(client_id, &clients_cpy).await;
}

pub async fn client_channel_broadcast_formatted(client_id: ClientId, msg: &str, clients: &Clients) {
    
    if msg.is_empty() {
        println!("[Client {}] Received empty message: Discarding!", client_id);
        if let Some(online_user) = clients.get(&client_id) {
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
        println!("[Client {}] Received large message: Discarding!", client_id);
        if let Some(online_user) = clients.get(&client_id) {
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
    
    println!("[Client {}] Received message: Broadcasting...", client_id);
    
    use comrak::{markdown_to_html, ComrakOptions};
    let msg = markdown_to_html(&msg, &ComrakOptions::default());
    
    let new_msg = json!({
        "type": "channel.broadcast.formatted",
        "view": "default",
        "client": client_id,
        "message": msg
    }).to_string();
    
    client_channel_broadcast_text(&new_msg, &clients).await;
}

pub async fn client_channel_broadcast_text(msg: &str, clients: &Clients) {
    for multiref in clients.iter_mut() {
        if let Err(_disconnected) = multiref.value().send_text(msg) {
            // Nothing to do here.
        }
    }
}

pub async fn client_channel_broadcast_binary(payload: &[u8], clients: &Clients) {
    for multiref in clients.iter_mut() {
        if let Err(_disconnected) = multiref.value().send_binary(payload) {
            // Nothing to do here.
        }
    }
}

pub async fn client_disconnected(my_id: ClientId, clients: &Clients) {
    eprintln!("[Client {}] Disconnected!", my_id);
    
    let client_disconnect_msg = json!({
        "type": "client.leave",
        "user": my_id
    }).to_string();
    
    for multiref in clients.iter_mut() {
        if &my_id != multiref.key() {
            if let Err(_disconnected) = multiref.value().send_text(client_disconnect_msg.clone()) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }

    // Stream closed up, so remove from the user list
    clients.remove(&my_id);
}
