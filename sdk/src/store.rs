//! This module provides wrappers around various data structures from the `ic_stable_structures` crate.
//!
//! These wrappers automatically impl the `__CustomStorageAccess` trait, you should use `T::with` and `T::with_mut` methods to access the inner structure.
//!
//! # Example
//! ```
//! #[exchange]
//! pub mod exchange {
//!     use ree_exchange_sdk::store::*;
//!
//!     // ... other code ...
//!
//!     // Define a storage type using the StableBTreeMap wrapper with `MemoryId = 0`
//!     #[storage(memory = 0)]
//!     pub type DummyStorage = StableBTreeMap<String, String>;
//!
//!     // Now you can use `DummyStorage` in your exchange module
//!     #[ic_cdk::update]
//!     pub fn new_pool() {
//!         DummyStorage::with_mut(|map| map.insert("hello".to_string(), "world".to_string()));
//!     }
//! }
//!
//! // if you want to use the storage outside the exchange module, you can do it like this:
//! use self::exchange::*;
//!
//! #[ic_cdk::update]
//! pub fn use_storage_outside() {
//!    DummyStorage::with_mut(|map| map.insert("foo".to_string(), "bar".to_string()));
//! }
//! ```

use ic_stable_structures::{BTreeMap, BTreeSet, Cell, MinHeap, Storable, Vec};

#[doc(hidden)]
pub trait StorageType {
    type Type;

    fn init(memory: crate::Memory) -> Self::Type;
}

/// Wrapper around `ic_stable_structures::BTreeMap`.
/// reference: <https://docs.rs/ic-stable-structures/latest/ic_stable_structures/btreemap/struct.BTreeMap.html>
pub struct StableBTreeMap<K: Storable + Ord + Clone, V: Storable> {
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K, V> StorageType for StableBTreeMap<K, V>
where
    K: Storable + Ord + Clone,
    V: Storable,
{
    type Type = BTreeMap<K, V, crate::Memory>;

    fn init(memory: crate::Memory) -> BTreeMap<K, V, crate::Memory> {
        BTreeMap::init(memory)
    }
}

/// Wrapper around `ic_stable_structures::Cell`.
/// reference: <https://docs.rs/ic-stable-structures/latest/ic_stable_structures/cell/struct.Cell.html>
pub struct StableCell<T: Storable> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageType for StableCell<T>
where
    T: Storable,
{
    type Type = Cell<Option<T>, crate::Memory>;

    fn init(memory: crate::Memory) -> Cell<Option<T>, crate::Memory> {
        Cell::init(memory, None)
    }
}

/// Wrapper around `ic_stable_structures::BTreeSet`.
/// reference: <https://docs.rs/ic-stable-structures/latest/ic_stable_structures/btreeset/struct.BTreeSet.html>
pub struct StableBTreeSet<T: Storable + Ord + Clone> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageType for StableBTreeSet<T>
where
    T: Storable + Ord + Clone,
{
    type Type = BTreeSet<T, crate::Memory>;

    fn init(memory: crate::Memory) -> BTreeSet<T, crate::Memory> {
        BTreeSet::init(memory)
    }
}

/// Wrapper around `ic_stable_structures::Vec`.
/// reference: <https://docs.rs/ic-stable-structures/latest/ic_stable_structures/vec/struct.Vec.html>
pub struct StableVec<T: Storable> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageType for StableVec<T>
where
    T: Storable,
{
    type Type = Vec<T, crate::Memory>;

    fn init(memory: crate::Memory) -> Vec<T, crate::Memory> {
        Vec::init(memory)
    }
}

/// Wrapper around `ic_stable_structures::MinHeap`.
/// reference: <https://docs.rs/ic-stable-structures/latest/ic_stable_structures/min_heap/struct.MinHeap.html>
pub struct StableMinHeap<T: Storable + Ord + Clone> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> StorageType for StableMinHeap<T>
where
    T: Storable + Ord + Clone,
{
    type Type = MinHeap<T, crate::Memory>;

    fn init(memory: crate::Memory) -> MinHeap<T, crate::Memory> {
        MinHeap::init(memory)
    }
}
