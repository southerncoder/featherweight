//! Identity-keyed, insertion-ordered set of lines: the Rust stand-in for the
//! TypeScript `Set<PhysicalTextLine>` / `ReadonlySet<PhysicalTextLine>` used
//! across the layout and recognition stages. Rust-only support module (it has
//! no TypeScript counterpart file).
//!
//! Membership is by object identity (`std::ptr::eq`), exactly like a JS `Set`
//! of objects, and iteration follows first-insertion order.
use std::collections::HashSet;

use crate::core::physical_lines::PhysicalTextLine;

#[derive(Debug, Clone, Default)]
pub struct LineSet<'a> {
    members: HashSet<*const PhysicalTextLine>,
    order: Vec<&'a PhysicalTextLine>,
}

impl<'a> LineSet<'a> {
    /// `new Set()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// `Set.prototype.add`; returns `true` when the line was not yet present.
    pub fn insert(&mut self, line: &'a PhysicalTextLine) -> bool {
        let inserted = self.members.insert(std::ptr::from_ref(line));
        if inserted {
            self.order.push(line);
        }
        inserted
    }

    /// `Set.prototype.has` (identity comparison).
    pub fn contains(&self, line: &PhysicalTextLine) -> bool {
        self.members.contains(&std::ptr::from_ref(line))
    }

    /// `Set.prototype.size`.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// `set.size === 0`.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Iterates in insertion order, like `[...set]`.
    pub fn iter(&self) -> impl Iterator<Item = &'a PhysicalTextLine> + '_ {
        self.order.iter().copied()
    }

    /// `[...set]`.
    pub fn to_vec(&self) -> Vec<&'a PhysicalTextLine> {
        self.order.clone()
    }

    /// Consumes the set, returning its lines in insertion order.
    pub fn into_vec(self) -> Vec<&'a PhysicalTextLine> {
        self.order
    }
}

impl<'a> FromIterator<&'a PhysicalTextLine> for LineSet<'a> {
    fn from_iter<I: IntoIterator<Item = &'a PhysicalTextLine>>(iter: I) -> Self {
        let mut set = Self::new();
        set.extend(iter);
        set
    }
}

impl<'a> Extend<&'a PhysicalTextLine> for LineSet<'a> {
    fn extend<I: IntoIterator<Item = &'a PhysicalTextLine>>(&mut self, iter: I) {
        for line in iter {
            self.insert(line);
        }
    }
}
