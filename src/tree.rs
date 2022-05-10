// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::HashMap, hash::Hash};

use derive_more::{From, Into};
use indextree::{Arena, Node, NodeId};
use spdx_rs::models;
use typed_index_collections::TiVec;

use crate::cleanup::{cleanup_copyright_text, StrExt};

/// Identifier per `Metadata`
#[derive(From, Into, Debug, Clone, Copy)]
struct MetadataId(usize);

/// Combination of copyright text and license. We try to unify over these.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub copyright_text: String,
    pub license: String,
}

/// A part of a path, which might have a Metadata (copyright + license) associated with it, by ID.
struct Element {
    path_segment: String,
    metadata: Option<MetadataId>,
}
impl Element {
    fn new(path_segment: &str) -> Self {
        Self {
            path_segment: path_segment.to_string(),
            metadata: None,
        }
    }
}

/// Find an element that is a child of `parent_node_id` that satisfies `pred`, and return its ID.
/// If none exists, create a new element by calling `factory()` and appending it.
/// Return the child node ID in either case.
fn get_or_insert_child<'a, P, F>(
    arena: &mut Arena<Element>,
    parent_node_id: NodeId,
    pred: P,
    factory: F,
) -> NodeId
where
    P: Fn(&Node<Element>) -> bool,
    F: FnOnce() -> Element,
{
    let maybe_node_id = parent_node_id.children(&arena).find(|id| {
        // let pred = &pred;
        arena.get(*id).map_or(false, |node| pred(node))
    });
    match maybe_node_id {
        Some(found_node_id) => found_node_id,
        None => {
            let new_id = arena.new_node(factory());
            parent_node_id.append(new_id, arena);
            new_id
        }
    }
}

/// Find or create a node in the `arena` (rooted at `root`), corresponding to the provided path, split on '/' into path segments.
/// Returns the node ID.
fn find_or_create_node(arena: &mut Arena<Element>, root: NodeId, path: &str) -> NodeId {
    // let mut node_id = root;
    path.split('/').fold(root, |parent_id, path_segment| {
        get_or_insert_child(
            arena,
            parent_id,
            |node| node.get().path_segment == path_segment,
            || Element::new(&path_segment),
        )
    })
    // for path_segment in path.split('/') {
    //     let new_node_id = get_or_insert_child(
    //         arena,
    //         node_id,
    //         |node| node.get().path_segment == path_segment,
    //         || Element::new(&path_segment),
    //     );
    //     node_id = new_node_id;
    // }
    // root
}

/// Stores license and copyright metadata and an associated tree data structure corresponding to the file system tree.
pub struct CopyrightDataTree {
    tree_arena: Arena<Element>,
    root: NodeId,
    metadata: TiVec<MetadataId, Metadata>,
    metadata_map: HashMap<Metadata, MetadataId>,
}

impl CopyrightDataTree {
    fn new() -> Self {
        let mut arena = Arena::new();
        let root = arena.new_node(Element::new("."));
        Self {
            tree_arena: arena,
            root,
            metadata: TiVec::default(),
            metadata_map: HashMap::default(),
        }
    }
    /// Make a new instance from an iterator of SPDX file information.
    pub fn from_iter(iter: impl Iterator<Item = models::FileInformation>) -> Self {
        let mut ret = Self::new();
        ret.accumulate_from_iter(iter);
        ret
    }

    fn accumulate_from_iter(&mut self, iter: impl Iterator<Item = models::FileInformation>) {
        for item in iter {
            self.accumulate(&item);
        }
    }
    /// Add a single element of SPDX FileInformation to the tree, after cleanup and processing.
    fn accumulate(&mut self, item: &models::FileInformation) {
        let license = item.license_information_in_file.join(" OR ");
        let copyright_text = cleanup_copyright_text(&item.copyright_text).join("\n");
        let filename = item.file_name.strip_prefix_if_present("./");
        let metadata = Metadata {
            copyright_text,
            license,
        };
        let metadata_id = self.find_or_insert_metadata(metadata);
        let id = find_or_create_node(&mut self.tree_arena, self.root, &filename);
        let node = self.tree_arena.get_mut(id).unwrap();
        node.get_mut().metadata = Some(metadata_id);
    }

    /// Search for the provided metadata, returning its ID if it is already known.
    /// If it is not known, add it to our collection, assign an ID, and return that ID.
    fn find_or_insert_metadata(&mut self, metadata: Metadata) -> MetadataId {
        match self.metadata_map.get(&metadata) {
            Some(id) => *id,
            None => {
                let id = self.metadata.push_and_get_key(metadata.clone());
                self.metadata_map.insert(metadata, id);
                id
            }
        }
    }
}
