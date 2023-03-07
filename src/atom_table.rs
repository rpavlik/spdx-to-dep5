// Copyright 2022-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: BSL-1.0
//
// Author: Rylie Pavlik <rylie.pavlik@collabora.com>

use std::{collections::HashMap, fmt::Debug, hash::Hash};
use typed_index_collections::TiVec;

/// A data structure that lets you use strongly typed indices/keys instead
/// of bulky values, performing a lookup in both directions
#[derive(Debug)]
pub(crate) struct AtomTable<T, I>
where
    I: From<usize> + Copy,
{
    vec: TiVec<I, T>,
    map: HashMap<T, I>,
}

impl<T, I> Default for AtomTable<T, I>
where
    I: From<usize> + Copy,
{
    fn default() -> Self {
        Self {
            vec: Default::default(),
            map: Default::default(),
        }
    }
}

impl<T, I> AtomTable<T, I>
where
    T: Hash + Eq,
    I: From<usize> + Copy,
    usize: From<I>,
{
    pub(crate) fn get_or_create_id_for_owned_value(&mut self, value: T) -> I
    where
        T: Clone,
    {
        if let Some(id) = self.map.get(&value) {
            return *id;
        }
        let id = self.vec.push_and_get_key(value.clone());
        self.map.insert(value, id);
        id
    }

    pub(crate) fn get_or_create_id(&mut self, value: &T) -> I
    where
        T: Clone,
    {
        if let Some(id) = self.map.get(value) {
            return *id;
        }
        let id = self.vec.push_and_get_key(value.clone());
        self.map.insert(value.clone(), id);
        id
    }

    pub(crate) fn get_id(&self, value: &T) -> Option<I> {
        self.map.get(value).copied()
    }

    pub(crate) fn get_value(&self, id: I) -> Option<&T> {
        self.vec.get(id)
    }

    /// Apply a function to all values
    pub(crate) fn transform<U: Hash + Eq + Clone>(
        &self,
        mut f: impl FnMut(&T) -> U,
    ) -> AtomTable<U, I>
    where
        I: Eq + Debug,
    {
        self.try_transform(move |val| -> Result<U, ()> { Ok(f(val)) })
            .expect("Nowhere to introduce an error")
    }
    pub(crate) fn try_transform<U: Hash + Eq + Clone, E>(
        &self,
        mut f: impl FnMut(&T) -> Result<U, E>,
    ) -> Result<AtomTable<U, I>, E>
    where
        I: Eq + Debug,
    {
        let mut vec: TiVec<I, U> = Default::default();

        let mut map: HashMap<U, I> = Default::default();
        vec.reserve(self.vec.len());
        for (id, val) in self.vec.iter_enumerated() {
            let new_val = f(val)?;
            let new_id = vec.push_and_get_key(new_val.clone());
            assert_eq!(new_id, id);
            let old_id_for_this_val = map.insert(new_val, new_id);
            // Must not map more than one input to a single output
            assert!(old_id_for_this_val.is_none());
        }
        Ok(AtomTable { vec, map })
    }
}
