use crate::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, Deref)]
pub struct ClusterNodes(SharedRwLock<HashMap<String, NodeEntities>>);

#[derive(Debug, Clone, Default, Deref)]
pub struct NodeEntities(SharedRwLock<HashMap<String, EntityDataContainer>>);

#[derive(Debug, Clone, Default, Deref)]
pub struct EntityDataContainer(SharedRwLock<EntityData>);

#[derive(Debug, Clone, Default)]
pub struct EntityData {
    pub stat: String,
    pub attr: Document,
}

macro_rules! get_or_insert {
    ($item:ident, $name:ident, $type:ty) => {{
        let item = {
            let lck = $item.read().await;
            (&*lck).get($name).cloned()
        };
        if item.is_some() {
            item.unwrap()
        } else {
            let mut lck = $item.write().await;
            let i: $type = Default::default();
            lck.insert($name.into(), i.clone());
            i
        }
    }};
}

impl ClusterNodes {
    pub async fn update_stat(&self, node: &str, entity: &str, stat: String) {
        let e = get_or_insert!(self, node, NodeEntities);
        let dat = get_or_insert!(e, entity, EntityDataContainer);
        let mut lck = dat.write().await;
        lck.stat = stat;
    }

    pub async fn update_attr(&self, node: &str, entity: &str, attr: Document) {
        let e = get_or_insert!(self, node, NodeEntities);
        let dat = get_or_insert!(e, entity, EntityDataContainer);
        let mut lck = dat.write().await;
        lck.attr = attr;
    }

    pub async fn get_nodes(&self) -> Vec<String> {
        let lck = self.read().await;
        lck.keys().map(|k| k.to_string()).collect()
    }

    pub async fn get_entities(&self) -> HashSet<String> {
        let lck = self.read().await;
        let mut result: HashSet<String> = Default::default();
        for (_, v) in lck.iter() {
            let lck = v.read().await;
            for val in lck.keys().map(|k| k.to_string()).into_iter() {
                result.insert(val);
            }
        }
        result
    }

    pub async fn get_dev_id_prefix(&self, dev_id: &str) -> Vec<(String, String, EntityData)> {
        let lck = self.read().await;
        let mut result: Vec<(String, String, EntityData)> = Default::default();
        for (k, v) in lck.iter() {
            let lck = v.read().await;
            for (k2, v) in lck.iter() {
                if k2.starts_with(dev_id) {
                    let v = v.read().await;
                    result.insert(0, (k.to_string(), k2.to_string(), v.clone()));
                }
            }
        }
        result
    }
}
