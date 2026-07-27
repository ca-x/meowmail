use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use uuid::Uuid;

type MailboxKey = (Uuid, Uuid);
type MailboxLock = AsyncMutex<()>;
type MailboxLockMap = HashMap<MailboxKey, Weak<MailboxLock>>;

#[derive(Clone, Default)]
pub struct MailboxLocks {
    inner: Arc<Mutex<MailboxLockMap>>,
}

impl MailboxLocks {
    pub async fn lock(&self, user_id: Uuid, account_id: Uuid) -> OwnedMutexGuard<()> {
        self.get(user_id, account_id).lock_owned().await
    }

    pub fn try_lock(&self, user_id: Uuid, account_id: Uuid) -> Option<OwnedMutexGuard<()>> {
        self.get(user_id, account_id).try_lock_owned().ok()
    }

    fn get(&self, user_id: Uuid, account_id: Uuid) -> Arc<MailboxLock> {
        let mut locks = self.inner.lock().expect("mailbox lock map poisoned");
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(&(user_id, account_id)).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert((user_id, account_id), Arc::downgrade(&lock));
        lock
    }
}

#[cfg(test)]
mod tests {
    use super::MailboxLocks;

    #[tokio::test]
    async fn serializes_operations_for_the_same_user_and_account() {
        let locks = MailboxLocks::default();
        let user_id = uuid::Uuid::new_v4();
        let account_id = uuid::Uuid::new_v4();
        let first = locks.lock(user_id, account_id).await;
        let same_lock = locks
            .inner
            .lock()
            .unwrap()
            .get(&(user_id, account_id))
            .unwrap()
            .upgrade()
            .unwrap();

        assert!(same_lock.clone().try_lock_owned().is_err());
        drop(first);
        assert!(same_lock.try_lock_owned().is_ok());
    }
}
