use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use super::identity::{HashIndex, hash_admitted};
use super::{ConstructedValue, key_equal};
use crate::{Budget, Location, QueryError};

type Entry = (ConstructedValue, ConstructedValue);

/// Insertion-ordered constructed dictionary entries with a lazily built key index.
/// Public construction is checked by the ordinary budgeted value entry points.
/// Mutable access invalidates the index; iteration and Rust equality retain order.
/// Convert owned entry pairs with `entries.into()`; budgeted copying remains on
/// [`super::copy_constructed_value`] rather than a deep `Clone` implementation.
pub struct Dictionary {
    entries: Vec<Entry>,
    index: OnceLock<Box<HashIndex>>,
}

impl Dictionary {
    pub(crate) fn position(
        &self,
        key: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<usize>, QueryError> {
        let hash = hash_admitted(key, budget, location)?;
        let index = self.key_index(budget, location)?;
        budget.charge(1, location.clone())?;
        for &position in index.candidates(hash) {
            if key_equal(&self.entries[position].0, key, budget, location)? {
                return Ok(Some(position));
            }
        }
        Ok(None)
    }

    fn key_index(
        &self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<&HashIndex, QueryError> {
        if let Some(index) = self.index.get() {
            return Ok(index);
        }
        let mut index = HashIndex::default();
        for (position, (key, _)) in self.entries.iter().enumerate() {
            let hash = hash_admitted(key, budget, location)?;
            index.insert(hash, position, budget, location)?;
        }
        // Concurrent readers may have installed the same immutable index first.
        let _ = self.index.set(Box::new(index));
        Ok(self.index.get().expect("a complete index was installed"))
    }
}

impl From<Vec<Entry>> for Dictionary {
    fn from(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            index: OnceLock::new(),
        }
    }
}

impl Deref for Dictionary {
    type Target = [Entry];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

impl DerefMut for Dictionary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.index.take();
        &mut self.entries
    }
}

impl IntoIterator for Dictionary {
    type Item = Entry;
    type IntoIter = std::vec::IntoIter<Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Dictionary {
    type Item = &'a Entry;
    type IntoIter = std::slice::Iter<'a, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

impl<'a> IntoIterator for &'a mut Dictionary {
    type Item = &'a mut Entry;
    type IntoIter = std::slice::IterMut<'a, Entry>;

    fn into_iter(self) -> Self::IntoIter {
        self.index.take();
        self.entries.iter_mut()
    }
}

impl fmt::Debug for Dictionary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.entries.fmt(formatter)
    }
}

impl PartialEq for Dictionary {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}
