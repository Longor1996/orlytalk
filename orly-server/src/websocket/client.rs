use super::*;

#[derive(Serialize, Debug)]
pub struct OnlineClient {
    pub id: ClientId,
    pub user: Option<UserId>,
    
    #[serde(skip)]
    pub wstx: mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>
}

impl OnlineClient {
    pub fn send(&self, msg: &OrlyMessage) {
        msg.send(self);
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct OnlineClientInfo {
    pub id: ClientId,
    pub user: Option<User>,
}

pub type Clients = std::sync::Arc<DashMap<ClientId, OnlineClient>>;
