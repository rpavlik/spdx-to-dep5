// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::HashMap, hash::Hash, iter::FromIterator};

use derive_more::{From, Into};
use indextree::{Arena, Node, NodeEdge, NodeId, Traverse};
use itertools::Itertools;
use spdx_rs::models::{self, SimpleExpression};
use typed_index_collections::TiVec;

use crate::{
    cleanup::{cleanup_copyright_text, StrExt},
    deb822::dep5::FilesParagraph,
};

/// Identifier per `Metadata`
#[derive(From, Into, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MetadataId(usize);

/// Combination of copyright text and license. We try to unify over these.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub copyright_text: String,
    pub license: Vec<SimpleExpression>,
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
fn get_or_insert_child<P, F>(
    arena: &mut Arena<Element>,
    parent_node_id: NodeId,
    pred: P,
    factory: F,
) -> NodeId
where
    P: Fn(&Node<Element>) -> bool,
    F: FnOnce() -> Element,
{
    parent_node_id
        .children(arena)
        .find(|&id| arena.get(id).map_or(false, &pred))
        .unwrap_or_else(|| {
            let new_id = arena.new_node(factory());
            parent_node_id.append(new_id, arena);
            new_id
        })
}

/// Find or create a node in the `arena` (rooted at `root`), corresponding to the provided path, split on '/' into path segments.
/// Returns the node ID.
fn find_or_create_node(arena: &mut Arena<Element>, root: NodeId, path: &str) -> NodeId {
    path.split('/').fold(root, |parent_id, path_segment| {
        get_or_insert_child(
            arena,
            parent_id,
            |node| node.get().path_segment == path_segment,
            || Element::new(path_segment),
        )
    })
}

/// Keep advancing a traversal until it returns the "End" of the given `id` or runs out of elements.
fn skip_until_end_of_id(traversal: &mut Traverse<Element>, id: NodeId) {
    while let Some(edge) = traversal.next() {
        if let NodeEdge::End(end_id) = edge {
            if end_id == id {
                return;
            }
        }
    }
}

/// Stores license and copyright metadata and an associated tree data structure corresponding to the file system tree.
pub struct CopyrightDataTree {
    tree_arena: Arena<Element>,
    root: NodeId,
    metadata: TiVec<MetadataId, Metadata>,
    metadata_map: HashMap<Metadata, MetadataId>,
}

impl Extend<models::FileInformation> for CopyrightDataTree {
    fn extend<T: IntoIterator<Item = models::FileInformation>>(&mut self, iter: T) {
        for item in iter {
            self.accumulate(&item)
        }
    }
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

    /// Add a single element of SPDX FileInformation to the tree, after cleanup and processing.
    fn accumulate(&mut self, item: &models::FileInformation) {
        let license = item.license_information_in_file.clone();
        let copyright_text = cleanup_copyright_text(&item.copyright_text).join("\n");
        let metadata_id = self.find_or_insert_metadata(Metadata {
            copyright_text,
            license,
        });
        let filename = item.file_name.strip_prefix_if_present("./");
        let id = find_or_create_node(&mut self.tree_arena, self.root, filename);
        let node = self.tree_arena.get_mut(id).unwrap();
        node.get_mut().metadata = Some(metadata_id);
    }

    /// Search for the provided metadata, returning its ID if it is already known.
    /// If it is not known, add it to our collection, assign an ID, and return that ID.
    fn find_or_insert_metadata(&mut self, metadata: Metadata) -> MetadataId {
        self.metadata_map
            .get(&metadata)
            .copied()
            .unwrap_or_else(|| {
                let id = self.metadata.push_and_get_key(metadata.clone());
                self.metadata_map.insert(metadata, id);
                id
            })
    }
    fn set_metadata_id_for_node(&mut self, id: NodeId, metadata_id: MetadataId) {
        if let Some(node) = self.tree_arena.get_mut(id) {
            node.get_mut().metadata = Some(metadata_id);
        }
    }
    /// If all the direct children of `id` share the same Some(metadata_id), return it
    fn get_common_child_metadata_id_if_any(&self, id: NodeId) -> Option<MetadataId> {
        let all_child_metadata = id.children(&self.tree_arena).map(|child_id| {
            self.tree_arena
                .get(child_id)
                .and_then(|node| node.get().metadata)
        });
        let mut unique_metadata = all_child_metadata.unique();
        let first = unique_metadata.next();
        if let Some(Some(metadata_id)) = first {
            // OK we got one valid one.
            if unique_metadata.count() == 0 {
                // and none left after that: which means we have one unique one.
                return Some(metadata_id);
            }
        }
        None
    }

    fn is_directory(&self, id: NodeId) -> bool {
        id.children(&self.tree_arena).count() > 0
    }

    fn get_path(&self, id: NodeId) -> Option<String> {
        let mut ancestors = id.ancestors(&self.tree_arena).collect_vec();
        ancestors.reverse();
        let ancestor_nodes: Option<Vec<_>> = ancestors
            .into_iter()
            .map(|id| self.tree_arena.get(id))
            .collect();
        if let Some(ancestor_nodes) = ancestor_nodes {
            let path = ancestor_nodes
                .into_iter()
                .map(|node| node.get().path_segment.as_str())
                .join("/");
            return Some(path);
        }
        None
    }

    fn get_pattern(&self, id: NodeId) -> Option<String> {
        self.get_path(id).map(|path| {
            if self.is_directory(id) {
                path + "/*"
            } else {
                path
            }
        })
    }
    fn get_metadata_id(&self, id: NodeId) -> Option<MetadataId> {
        self.tree_arena.get(id).and_then(|node| node.get().metadata)
    }

    /// Propagate metadata IDs upward when all children have the same metadata ID
    pub fn propagate_metadata(&mut self) {
        // Record the visit order so we can be done with the iterator and modify the tree
        let mut visit_order = vec![];
        for edge in self.root.traverse(&self.tree_arena) {
            if let NodeEdge::End(id) = edge {
                visit_order.push(id);
            }
        }
        for id in visit_order {
            if let Some(child_metadata_id) = self.get_common_child_metadata_id_if_any(id) {
                self.set_metadata_id_for_node(id, child_metadata_id);
            }
        }
    }
}

impl FromIterator<models::FileInformation> for CopyrightDataTree {
    fn from_iter<T: IntoIterator<Item = models::FileInformation>>(iter: T) -> Self {
        let mut ret = Self::new();
        ret.extend(iter);
        ret
    }
}

struct NodeIdsWithMetadata<'a> {
    cdt: &'a CopyrightDataTree,
    traversal: Traverse<'a, Element>,
}
impl<'a> NodeIdsWithMetadata<'a> {
    fn new(cdt: &'a CopyrightDataTree) -> NodeIdsWithMetadata<'a> {
        NodeIdsWithMetadata {
            cdt,
            traversal: cdt.root.traverse(&cdt.tree_arena),
        }
    }
}
impl Iterator for NodeIdsWithMetadata<'_> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(edge) = self.traversal.next() {
            // Only starts are interesting
            if let NodeEdge::Start(id) = edge {
                // If we have our own metadata ID then we are the path
                if self.cdt.get_metadata_id(id).is_some() {
                    // skip all our descendants
                    skip_until_end_of_id(&mut self.traversal, id);
                    return Some(id);
                }
            }
        }
        None
    }
}

pub fn make_paragraphs(cdt: CopyrightDataTree) -> impl Iterator<Item = FilesParagraph> {
    let mut paras = vec![];
    let grouped = NodeIdsWithMetadata::new(&cdt).group_by(|&id| cdt.get_metadata_id(id));
    for (key, grouped_ids) in &grouped {
        let metadata_id = key.unwrap();
        if let Some(metadata) = cdt.metadata.get(metadata_id) {
            let files = grouped_ids
                .filter_map(|id| cdt.get_pattern(id))
                .sorted_unstable()
                .collect_vec()
                .join("\n");
            let license_string = metadata
                .license
                .iter()
                .map(|expr| expr.to_string())
                .join(" OR ");
            paras.push(FilesParagraph {
                files: files.into(),
                copyright: metadata.copyright_text.clone().into(),
                license: license_string.into(),
                comment: None,
            })
        }
    }
    paras.into_iter()
}
