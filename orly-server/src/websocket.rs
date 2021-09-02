use std::sync::Arc;

use warp::ws::WebSocket;

use serde::{Serialize, Deserialize};

use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use dashmap::DashMap;

use crate::{RuntimeState, User, UserId};
pub type ClientId = u64;

pub mod messages;
pub use messages::*;

pub mod client;
pub use client::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserConnectionRequest {}

pub async fn client_connected(ws: WebSocket, _ucr: UserConnectionRequest, state: Arc<RuntimeState>) {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;
    lazy_static! {
        static ref CLIENT_ID_AUTO_INCREMENT: AtomicU64 = {
            AtomicU64::new(0)
        };
    };
    
    let clients = state.clients.clone();
    
    // Use a counter to assign a new unique ID for this user.
    let client_id = CLIENT_ID_AUTO_INCREMENT.fetch_add(1, Ordering::Relaxed);
    
    let user_id = uuid::Uuid::new_v4();
    
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
        user: Some(user_id),
        wstx: tx
    };
    
    client.send(&OrlyMessage::ClientInfoSelf {client: &client});
    
    client_channel_broadcast(&OrlyMessage::ClientJoin {client: &client}, &clients).await;
    
    // This is horrible.
    client.send(&OrlyMessage::ClientInfoList {
        clients: clients.iter()
            .map(| m | OnlineClientInfo {
                id: m.id,
                user: m.user.map(|id| state.users
                        .get(&id)
                        .map(|u| u.value().clone())
                    )
                    .flatten()
            })
            .collect()
    });
    
    {
        let posts = state.messages.read().await;
        for post in posts.iter() {
            let msg = post.into();
            client.send(&msg);
        }
    }
    
    // Save the sender in our list of connected clients.
    clients.insert(client_id, client);
    
    // Make an extra clone to give to our disconnection handler...
    let clients_cpy = clients.clone();
    
    eprintln!("[Client {}] Now processing messages...", client_id);
    
    // Process messages coming from the client...
    while let Some(result) = client_recv.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("[Client {}] Websocket Error: {}", client_id, e);
                break;
            }
        };
        
        if msg.is_close() {
            eprintln!("[Client {}] Websocket Closing...", client_id);
            break;
        }
        
        let msg = match OrlyMessage::from_message(&msg) {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("[Client {}] Unable to parse message: {}", client_id, err);
                break;
            }
        };
        
        match msg {
            OrlyMessage::ClientChannelBroadcastData { message } => {
                client_channel_broadcast(&OrlyMessage::ChannelBroadcastData {
                    message,
                    view: "default".to_owned(),
                    user: user_id
                }, &clients).await;
            }
            
            OrlyMessage::ClientChannelBroadcastFormatted { message } => {
                match format_message(&message) {
                    Ok(message) => {
                        client_channel_broadcast(&OrlyMessage::ChannelBroadcastFormatted {
                            message,
                            view: "default".to_owned(),
                            user: user_id
                        }, &clients).await;
                    },
                    Err(err) => {
                        eprintln!("[Client {}] User message error: {}", client_id, err);
                    },
                }
            },
            
            OrlyMessage::ClientChannelBroadcast { message } => {
                client_channel_broadcast(&OrlyMessage::ChannelBroadcast {
                    message,
                    view: "default".to_owned(),
                    user: user_id
                }, &clients).await;
            },
            
            _ => {
                eprintln!("[Client {}] User packet not handled: {:?}", client_id, msg);
                break;
            }
        }
    }
    
    client_disconnected(client_id, &clients_cpy).await;
}

pub fn format_message(msg: &str) -> Result<String, &'static str> {
    
    if msg.is_empty() {
        return Err("message is empty");
    }
    
    if msg.len() > 1024 {
        return Err("message too large");
    }
    
    use comrak::{markdown_to_html, ComrakOptions};
    let opts = ComrakOptions::default();
    let msg = markdown_to_html(msg, &opts);
    
    Ok(msg)
}

pub async fn client_channel_broadcast(msg: &OrlyMessage<'_>, clients: &Clients) {
    for multiref in clients.iter_mut() {
        multiref.value().send(msg);
    }
}

pub async fn client_disconnected(client_id: ClientId, clients: &Clients) {
    eprintln!("[Client {}] Disconnected!", client_id);
    
    if let Some((_, client)) = clients.remove(&client_id) {
        let msg = OrlyMessage::ClientLeave {
            client: client.id,
            user: client.user,
        };
        
        client_channel_broadcast(&msg, clients).await;
    }
}
