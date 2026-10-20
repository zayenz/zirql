use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

use super::identity::{HashIndex, hash_admitted};
use super::{ConstructedValue, key_equal};
use crate::{Budget, Location, QueryError};

/// Insertion-ordered set storage with a lazily built structural membership index.
/// Public construction is checked by the ordinary constructed value entry points.
/// Mutable access invalidates the index. Rust equality compares stored order;
/// language equality compares membership. Copy through `copy_constructed_value`.
pub struct SetElements {
    values: Vec<ConstructedValue>,
    index: OnceLock<Box<HashIndex>>,
}

impl SetElements {
    /// Find a structurally equal member in admitted set storage.
    ///
    /// # Errors
    /// Hashing, comparison and index allocation use the supplied finite budget.
    pub(crate) fn position(
        &self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<Option<usize>, QueryError> {
        let hash = hash_admitted(value, budget, location)?;
        let index = self.membership_index(budget, location)?;
        budget.charge(1, location.clone())?;
        for &position in index.candidates(hash) {
            if key_equal(&self.values[position], value, budget, location)? {
                return Ok(Some(position));
            }
        }
        Ok(None)
    }

    /// Test structural membership in admitted set storage.
    ///
    /// # Errors
    /// Returns the same located budget or comparison failures as `position`.
    pub(crate) fn contains(
        &self,
        value: &ConstructedValue,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<bool, QueryError> {
        self.position(value, budget, location)
            .map(|position| position.is_some())
    }

    /// Recover the owned values without copying them.
    pub fn into_vec(self) -> Vec<ConstructedValue> {
        self.values
    }

    fn membership_index(
        &self,
        budget: &mut Budget,
        location: &Location,
    ) -> Result<&HashIndex, QueryError> {
        if let Some(index) = self.index.get() {
            return Ok(index);
        }
        let mut index = HashIndex::default();
        for (position, value) in self.values.iter().enumerate() {
            let hash = hash_admitted(value, budget, location)?;
            index.insert(hash, position, budget, location)?;
        }
        let _ = self.index.set(Box::new(index));
        Ok(self.index.get().expect("a complete index was installed"))
    }
}
impl From<Vec<ConstructedValue>> for SetElements {
    fn from(values: Vec<ConstructedValue>) -> Self {
        Self {
            values,
            index: OnceLock::new(),
        }
    }
}
impl FromIterator<ConstructedValue> for SetElements {
    fn from_iter<T: IntoIterator<Item = ConstructedValue>>(values: T) -> Self {
        values.into_iter().collect::<Vec<_>>().into()
    }
}
impl Deref for SetElements {
    type Target = [ConstructedValue];
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}
impl DerefMut for SetElements {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.index.take();
        &mut self.values
    }
}
impl IntoIterator for SetElements {
    type Item = ConstructedValue;
    type IntoIter = std::vec::IntoIter<ConstructedValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}
impl<'a> IntoIterator for &'a SetElements {
    type Item = &'a ConstructedValue;
    type IntoIter = std::slice::Iter<'a, ConstructedValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}
impl<'a> IntoIterator for &'a mut SetElements {
    type Item = &'a mut ConstructedValue;
    type IntoIter = std::slice::IterMut<'a, ConstructedValue>;
    fn into_iter(self) -> Self::IntoIter {
        self.index.take();
        self.values.iter_mut()
    }
}
impl fmt::Debug for SetElements {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.values.fmt(formatter)
    }
}
impl PartialEq for SetElements {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}
