use crate::prelude::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::time::SystemTime;

#[derive(Deref, Debug, Clone)]
pub struct ClusterState(SharedRwLock<ClusterStateData>);

#[derive(Debug)]
pub struct ClusterStateData {
    node_name:      String,
    sid:            String,
    current_leader: Option<String>,
    last_timestamp: SystemTime,
}

impl ClusterState {
    pub fn new(node_name: String) -> Self {
        ClusterState(Arc::new(RwLock::new(ClusterStateData {
            node_name,
            current_leader: None,
            last_timestamp: SystemTime::now(),
            sid: String::from_utf8(
                thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(30)
                    .collect::<Vec<u8>>(),
            )
            .unwrap(),
        })))
    }

    pub async fn is_leader(&self) -> bool {
        let s = self.read().await;
        matches!(&s.current_leader, Some(cl) if cl == &s.sid)
    }

    pub async fn set_leader(&self, leader: String) {
        let mut s = self.write().await;
        if !matches!(&s.current_leader, Some(cl) if cl == &leader) {
            debug!("Current cluster leader is '{}'", leader);
            (*s).current_leader = Some(leader);
        }
        (*s).last_timestamp = SystemTime::now();
    }

    pub async fn leader_needed(&self) -> Result<bool> {
        let s = self.read().await;
        Ok(s.current_leader.is_none() || (s.last_timestamp.elapsed()?.as_secs() > 60))
    }

    pub async fn get_leader(&self) -> Result<Option<String>> {
        let s = self.read().await;
        Ok(s.current_leader.clone())
    }

    pub async fn get_sid(&self) -> Result<String> {
        let s = self.read().await;
        Ok(s.sid.clone())
    }
}
