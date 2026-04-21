pub mod errors;
pub mod test_utils;

use derive_more::{Deref, DerefMut};
use native_db::*;
use parking_lot::{ArcRwLockReadGuard, ArcRwLockWriteGuard, RawRwLock, RwLock};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[macro_export]
macro_rules! iter_type_to_iter_variant {
    ($v:expr, $variant:path) => {
        $v.into_iter().map(|item| $variant(item))
    };
}

#[macro_export]
macro_rules! iter_variant_to_iter_type {
    ($v:expr, $variant:path) => {
        $v.into_iter()
            .filter_map(|item| {
                if let $variant(inner) = item {
                    Some(inner)
                } else {
                    None
                }
            })
            .collect()
    };
}

/// type alias for a [ArcRwLockReadGuard];
pub type PtrRGaurd<T> = ArcRwLockReadGuard<RawRwLock, T>;
pub type PtrWGaurd<T> = ArcRwLockWriteGuard<RawRwLock, T>;

/// Simple abstraction over [parking_lot::RwLock]
#[derive(Deref, DerefMut)]
pub struct Ptr<T>(Arc<RwLock<T>>);

impl<T: ToKey> ToKey for Ptr<T> {
    fn to_key(&self) -> Key {
        // get exclusive read & write access before writing to the database
        let ptr = &*self.clone().write_arc();
        ptr.to_key()
    }
    fn key_names() -> Vec<String> {
        vec!["Ptr".into(), "YomichanPtr".into()]
    }
}

impl<T> Ptr<T> {
    pub fn new(val: T) -> Self {
        Ptr(Arc::new(RwLock::new(val)))
    }

    /// Executes a closure with an immutable reference to the inner data.
    /// Used for quick reads to the inner `&T`
    ///
    /// # Example
    /// ```
    /// let name = my_ptr.with(|data| data.name.clone());
    /// ```
    pub fn with_ptr<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.0.read();
        f(&*guard)
    }

    /// Acquires a write lock, runs the closure, and releases the lock.
    /// Used for quick writes to the inner `&mut T`
    ///
    /// # Example
    ///
    /// ```
    /// my_ptr.with_ptr_mut(|data| {
    ///     data.counter += 1;
    /// });
    /// ```
    pub fn with_ptr_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.0.write();
        f(&mut *guard)
    }
}

impl<T> From<T> for Ptr<T> {
    fn from(value: T) -> Self {
        Self(Arc::new(parking_lot::RwLock::new(value)))
    }
}

impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T: PartialEq> PartialEq for Ptr<T> {
    fn eq(&self, other: &Self) -> bool {
        // Lock both for reading and then compare the values.
        // The locks are short-lived and dropped at the end of the statement.
        let self_guard = self.0.read();
        let other_guard = other.0.read();
        *self_guard == *other_guard
    }
}

impl<T: Eq> Eq for Ptr<T> {}

impl<T: PartialOrd> PartialOrd for Ptr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.read().partial_cmp(&*other.0.read())
    }
}

impl<T: Ord> Ord for Ptr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.read().cmp(&*other.0.read())
    }
}

impl<T: Hash> Hash for Ptr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.read().hash(state);
    }
}

impl<T: fmt::Debug> fmt::Debug for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ptr").field(&*self.0.read()).finish()
    }
}

impl<T: Default> Default for Ptr<T> {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(T::default())))
    }
}

impl<'de, T> Deserialize<'de> for Ptr<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value: T = T::deserialize(deserializer)?;
        Ok(Ptr::from(value))
    }
}

impl<T> Serialize for Ptr<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let guard = self.0.read();
        T::serialize(&*guard, serializer)
    }
}
