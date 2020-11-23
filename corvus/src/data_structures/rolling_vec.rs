use crate::prelude::*;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct RollingVecEntry<T> {
    timestamp: Instant,
    entry:     Arc<T>,
}

impl<T> RollingVecEntry<T> {
    fn new(entry: T) -> Self {
        RollingVecEntry {
            timestamp: Instant::now(),
            entry:     Arc::new(entry),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RollingVec<T> {
    entries:  SharedMutex<Vec<RollingVecEntry<T>>>,
    duration: Duration,
}

impl<T> Default for RollingVec<T> {
    fn default() -> Self {
        RollingVec {
            entries:  Arc::new(Mutex::new(vec![])),
            duration: Duration::from_secs(60),
        }
    }
}

impl<T> RollingVec<T> {
    pub fn new(duration: Duration) -> Self {
        RollingVec {
            entries: Arc::new(Mutex::new(vec![])),
            duration,
        }
    }

    pub async fn flush(&self) {
        let mut lck = (&*self.entries).lock().await;
        lck.retain(|i| i.timestamp.elapsed() < self.duration);
    }

    pub async fn add(&self, element: T) {
        let mut lck = (&*self.entries).lock().await;
        lck.push(RollingVecEntry::new(element));
    }

    pub async fn get_latest(&self) -> Option<Arc<T>> {
        self.flush().await;
        let lck = (&*self.entries).lock().await;
        match lck.last() {
            Some(i) => Some(i.entry.clone()),
            None => None,
        }
    }

    pub async fn get_all(&self) -> Vec<Arc<T>> {
        self.flush().await;
        let lck = (&*self.entries).lock().await;
        lck.iter().map(|i| i.entry.clone()).collect()
    }
}
