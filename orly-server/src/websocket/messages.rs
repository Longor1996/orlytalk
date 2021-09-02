//! Types of messages.

use serde::{Serialize, Deserialize};
use crate::user::UserId;
use super::{ClientId, OnlineClient, OnlineClientInfo};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum OrlyMessage<'m> {
    #[serde(rename = "empty")]
    Empty,
    
    #[serde(rename = "client.join", skip_deserializing)]
    ClientJoin { client: &'m OnlineClient },
    
    #[serde(rename = "client.leave", skip_deserializing)]
    ClientLeave { client: ClientId, user: Option<UserId> },
    
    #[serde(rename = "client-info.self", skip_deserializing)]
    ClientInfoSelf { client: &'m OnlineClient },
    
    #[serde(rename = "client-info.list", skip_deserializing)]
    ClientInfoList { clients: Vec<OnlineClientInfo> },
    
    #[serde(rename = "client.error", skip_deserializing)]
    ClientError { error: String },
    
    #[serde(rename = "channel.broadcast.text", skip_serializing)]
    ClientChannelBroadcast { message: String },
    
    #[serde(rename = "channel.broadcast.data", skip_serializing)]
    ClientChannelBroadcastData { message: &'m [u8] },
    
    #[serde(rename = "channel.broadcast.text.formatted", skip_serializing)]
    ClientChannelBroadcastFormatted { message: String },
    
    #[serde(rename = "channel.broadcast.text", skip_deserializing)]
    ChannelBroadcast { message: String, view: String, user: UserId },
    
    #[serde(rename = "channel.broadcast.data", skip_deserializing)]
    ChannelBroadcastData { message: &'m [u8], view: String, user: UserId },
    
    #[serde(rename = "channel.broadcast.text.formatted", skip_deserializing)]
    ChannelBroadcastFormatted { message: String, view: String, user: UserId },
}

#[derive(Serialize, Clone, Debug)]
pub struct OrlyMessageTx<'m> {
    #[serde(flatten)]
    message: OrlyMessage<'m>,
}

impl OrlyMessage<'_> {
    
    pub fn from_message<'m>(msg: &'m warp::ws::Message) -> Result<OrlyMessage<'m>, &'static str> {
        
        if msg.is_text() {
            let msg = msg.to_str().map_err(|_| "message is not a string")?;
            let msg = serde_json::from_str::<OrlyMessage>(msg).map_err(|_err| "invalid json")?;
            return Ok(msg);
        }
        
        if msg.is_binary() {
            let msg = msg.as_bytes();
            
            let index: usize = match msg.iter()
                .enumerate()
                .find_map(|(i,b)| if *b == b':' {Some(i)} else {None}) {
                    Some(i) => i,
                    None => {
                        return Err("could not find end of preload");
                    }
                };
            
            let (preload, payload) = msg.split_at(index);
            
            let preload = match std::str::from_utf8(preload) {
                Ok(str) => str,
                Err(_e) => {
                    return Err("preload is not valid UTF-8");
                }
            };
            
            let (msg_type, _msg_target) = match preload.split_once('@') {
                Some((mtype, mtarget)) => {
                    (mtype, Some(mtarget))
                },
                None => (preload, None),
            };
            
            if msg_type == "channel.broadcast.data" {
                return Ok(OrlyMessage::ClientChannelBroadcastData {
                    message: payload,
                })
            }
        }
        
        if msg.is_close() || msg.is_ping() || msg.is_pong() {
            return Ok(OrlyMessage::Empty)
        }
        
        Err("unknown message type")
    }
    
    pub fn send(&self, client: &OnlineClient) {
        
        // NOTE: This is thoroughly unsafe and stupid.
        if let OrlyMessage::ChannelBroadcastData {
            message,
            ..
        } = self {
            if let Err(err) = client.wstx.send(Ok(warp::ws::Message::binary(message.to_vec()))) {
                eprintln!("Failed to send message: {}", err);
            }
        }
        
        // ...all other messages are sent as text.
        match serde_json::to_string(&self) {
            Ok(str) => if let Err(err) = client.wstx.send(Ok(warp::ws::Message::text(str))) {
                eprintln!("Failed to send message: {}", err)
            },
            Err(err) => eprintln!("Failed to serialize message: {}", err),
        };
    }
    
}
