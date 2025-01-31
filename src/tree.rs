// Copyright 2021-2025, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    iter::{self, FromIterator},
};

use crate::{
    cleanup::{cleanup_copyright_text, StrExt},
    deb822::dep5::FilesParagraph,
};
use atom_table::AtomTable;
use copyright_statements::{
    Copyright, CopyrightDecompositionError, DecomposedCopyright, YearRangeCollection,
    YearRangeNormalizationOptions, YearSpec,
};
use derive_more::{From, Into};
use indextree::{Arena, Node, NodeEdge, NodeId, Traverse};
use itertools::Itertools;
use spdx_rs::models::{self, SpdxExpression};

/// Identifier per `Metadata`
#[derive(From, Into, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MetadataId(usize);

/// Combination of copyright text and license. We try to unify over these.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Metadata {
    pub copyright_text: String,
    pub license: Vec<SpdxExpression>,
}

trait MetadataStore {
    type CopyrightType;

    fn get_license_for_id(&self, id: MetadataId) -> Option<&Vec<SpdxExpression>>;

    fn get_copyright_text_for_id(&self, id: MetadataId) -> Option<&Self::CopyrightType>;
}

/// A part of a path, which might have a Metadata (copyright + license) associated with it, by ID.
#[derive(Debug)]
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
        .find(|&id| arena.get(id).is_some_and(&pred))
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
    for edge in traversal.by_ref() {
        if let NodeEdge::End(end_id) = edge {
            if end_id == id {
                return;
            }
        }
    }
}

/// Stores license and copyright metadata and an associated tree data structure corresponding to the file system tree.
#[derive(Debug)]
pub struct CopyrightDataTree<T = Metadata> {
    tree_arena: Arena<Element>,
    root: NodeId,
    metadata: AtomTable<T, MetadataId>,
}

impl Extend<models::FileInformation> for CopyrightDataTree {
    fn extend<T: IntoIterator<Item = models::FileInformation>>(&mut self, iter: T) {
        for item in iter {
            self.accumulate(&item)
        }
    }
}
impl<T> CopyrightDataTree<T> {
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
        for node in visit_order {
            if let Some(child_metadata_id) = self.get_common_child_metadata_id_if_any(node) {
                self.set_metadata_id_for_node(node, child_metadata_id);
            }
        }
    }
}

impl<T: Clone + Hash + Eq> CopyrightDataTree<T> {
    fn new() -> Self {
        let mut arena = Arena::new();
        let root = arena.new_node(Element::new("."));
        Self {
            tree_arena: arena,
            root,
            metadata: Default::default(),
        }
    }

    /// Search for the provided metadata, returning its ID if it is already known.
    /// If it is not known, add it to our collection, assign an ID, and return that ID.
    fn find_or_insert_metadata(&mut self, metadata: T) -> MetadataId {
        self.metadata.get_or_create_id_for_owned_value(metadata)
    }
}

impl CopyrightDataTree<Metadata> {
    /// Add a single element of SPDX FileInformation to the tree, after cleanup and processing.
    fn accumulate(&mut self, item: &models::FileInformation) {
        let license = item.license_information_in_file.clone();
        let copyright_text = cleanup_copyright_text(&item.copyright_text).join("\n");
        let metadata_id = self.find_or_insert_metadata(Metadata {
            copyright_text,
            license,
        });
        let filename = item.file_name.trim_start_matches("./");
        let id = find_or_create_node(&mut self.tree_arena, self.root, filename);
        let node = self.tree_arena.get_mut(id).unwrap();
        node.get_mut().metadata = Some(metadata_id);
    }
}

impl MetadataStore for CopyrightDataTree {
    type CopyrightType = String;

    fn get_license_for_id(&self, id: MetadataId) -> Option<&Vec<SpdxExpression>> {
        self.metadata.get(id).map(|m| &m.license)
    }

    fn get_copyright_text_for_id(&self, id: MetadataId) -> Option<&Self::CopyrightType> {
        self.metadata.get(id).map(|m| &m.copyright_text)
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct ParsedMetadata {
    license: Vec<SpdxExpression>,
    copyright: Copyright,
}

impl MetadataStore for CopyrightDataTree<ParsedMetadata> {
    type CopyrightType = Copyright;

    fn get_license_for_id(&self, id: MetadataId) -> Option<&Vec<SpdxExpression>> {
        self.metadata.get(id).map(|m| &m.license)
    }

    fn get_copyright_text_for_id(&self, id: MetadataId) -> Option<&Self::CopyrightType> {
        self.metadata.get(id).map(|m| &m.copyright)
    }
}

impl CopyrightDataTree {
    fn perform_copyright_decomposition(
        self,
        options: impl YearRangeNormalizationOptions + Copy,
    ) -> Result<CopyrightDataTree<ParsedMetadata>, CopyrightDecompositionError> {
        let metadata = self
            .metadata
            .try_transform_res(
                |metadata| -> Result<ParsedMetadata, CopyrightDecompositionError> {
                    let copyright = Copyright::try_parse(options, &metadata.copyright_text)?;
                    Ok(ParsedMetadata {
                        license: metadata.license.clone(),
                        copyright,
                    })
                },
            )
            .map_err(|e| {
                e.as_transform_function_error()
                    .expect("Our transform should be acceptable")
                    .clone()
            })?;
        Ok(CopyrightDataTree {
            tree_arena: self.tree_arena,
            root: self.root,
            metadata,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct LicenseAndHolders {
    license: Vec<SpdxExpression>,
    holders: Vec<String>,
}

impl LicenseAndHolders {
    fn new(license: Vec<SpdxExpression>, holders: impl IntoIterator<Item = String>) -> Self {
        let holders: Vec<String> = holders.into_iter().sorted().collect();
        Self { license, holders }
    }
}
#[derive(Debug, Clone)]
struct UsageCount<T> {
    data: HashMap<T, usize>,
}

impl<T: Hash + Clone + Eq> UsageCount<T> {
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Increases the count for the given value.
    /// Returns the current count after updating it.
    fn increment(&mut self, val: T) -> usize {
        match self.data.entry(val) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let new_count = e.get() + 1;
                e.insert(new_count)
            }
            std::collections::hash_map::Entry::Vacant(e) => *e.insert(1),
        }
    }

    /// Gets the current count for a value
    fn get(&self, val: &T) -> usize {
        self.data.get(val).copied().unwrap_or_default()
    }
}

impl<T: Hash + Clone + Eq> Default for UsageCount<T> {
    fn default() -> Self {
        Self::new()
    }
}

struct SummarizerOutput {
    metadata: ParsedMetadata,
    usage_count: usize,
    metadata_ids: HashSet<MetadataId>,
}

#[derive(Debug, Clone, Default)]
struct SubtreeSummarizer {
    ranges_per_holder: HashMap<String, YearRangeCollection>,
    license_and_holders_metadata_ids: HashMap<LicenseAndHolders, HashSet<MetadataId>>,
    metadata_id_usage_count: UsageCount<MetadataId>,
}

impl SubtreeSummarizer {
    fn record_ranges_for_line_holder(&mut self, line: &DecomposedCopyright) {
        self.ranges_per_holder
            .entry(line.holder.clone())
            .or_default()
            .extend(line.years.iter().cloned());
    }
    fn accumulate(
        &mut self,
        metadata_source: &impl MetadataStore<CopyrightType = Copyright>,
        metadata_id: MetadataId,
    ) {
        let id_usage_count = self.metadata_id_usage_count.increment(metadata_id);
        if id_usage_count > 1 {
            // we already processed this metadata ID
            return;
        }

        let copyright = metadata_source.get_copyright_text_for_id(metadata_id);
        let license = metadata_source.get_license_for_id(metadata_id);
        if let (Some(copyright), Some(license)) = (copyright, license) {
            let license_and_holders = match copyright {
                Copyright::Decomposable(single_line) => {
                    self.record_ranges_for_line_holder(single_line);
                    LicenseAndHolders::new(license.clone(), iter::once(single_line.holder.clone()))
                }
                Copyright::MultilineDecomposable(lines) => {
                    for line in lines {
                        self.record_ranges_for_line_holder(line);
                    }

                    LicenseAndHolders::new(
                        license.clone(),
                        lines.iter().map(|item| item.holder.clone()),
                    )
                }
                Copyright::Complex(_) => panic!("hey we didn't consider this case"),
            };
            self.license_and_holders_metadata_ids
                .entry(license_and_holders)
                .or_default()
                .insert(metadata_id);
        }
    }

    fn into_results(self) -> Vec<SummarizerOutput> {
        let mut ranges_per_holder = self.ranges_per_holder;
        let metadata_id_usage_count = &self.metadata_id_usage_count;
        let mut ret = vec![];
        for (license_and_holders, metadata_ids) in self.license_and_holders_metadata_ids.into_iter()
        {
            let usage_count = metadata_ids
                .iter()
                .map(|id| metadata_id_usage_count.get(id))
                .sum();
            let license = license_and_holders.license;
            let mut copyrights = license_and_holders
                .holders
                .into_iter()
                .map(|holder| {
                    let years = ranges_per_holder
                        .remove(&holder)
                        .expect("Should only get here if we've seen this holder")
                        .into_coalesced_vec()
                        .into_iter()
                        .map(|yr| {
                            if yr.is_single_year() {
                                YearSpec::SingleYear(yr.begin())
                            } else {
                                YearSpec::ClosedRange(yr)
                            }
                        })
                        .collect_vec();
                    DecomposedCopyright { years, holder }
                })
                .collect_vec();
            let copyright = if copyrights.len() == 1 {
                Copyright::Decomposable(copyrights.pop().expect("know this will succeed"))
            } else {
                Copyright::MultilineDecomposable(copyrights)
            };
            ret.push(SummarizerOutput {
                metadata: ParsedMetadata { license, copyright },
                usage_count,
                metadata_ids,
            });
        }
        ret
    }
}

pub fn summarize_metadata(
    tree: &CopyrightDataTree,
    node: NodeId,
    options: impl YearRangeNormalizationOptions + Copy,
) {
    let all_child_metadata = node.children(&tree.tree_arena).flat_map(|child_id| {
        tree.tree_arena
            .get(child_id)
            .and_then(|node| node.get().metadata)
    });
    let unique_metadata = all_child_metadata.unique();
    let _parsed: HashMap<MetadataId, Copyright> = unique_metadata
        .flat_map(|metadata_id| tree.metadata.get(metadata_id).map(|d| (metadata_id, d)))
        .map(|(metadata_id, metadata)| {
            (
                metadata_id,
                Copyright::try_parse(options, &metadata.copyright_text).unwrap(),
            )
        })
        .collect();
}

fn process_file_pattern(path: &str) -> String {
    path.trim_start_matches("./").replace(' ', "?") // apparently space is a reserved separator
}

pub fn make_paragraphs(cdt: CopyrightDataTree) -> impl Iterator<Item = FilesParagraph> {
    let mut paras = vec![];
    let grouped = NodeIdsWithMetadata::new(&cdt).chunk_by(|&id| cdt.get_metadata_id(id));
    for (key, grouped_ids) in &grouped {
        let metadata_id = key.unwrap();
        if let Some(metadata) = cdt.metadata.get(metadata_id) {
            let files = grouped_ids
                .filter_map(|id| cdt.get_pattern(id))
                .sorted_unstable()
                .map(|path| process_file_pattern(&path))
                .collect_vec()
                .join("\n");

            // Parenthesize complex expressions before merging
            let initial_license_string = metadata
                .license
                .iter()
                .map(|expr| {
                    if expr.licenses().len() == 1 {
                        expr.to_string()
                    } else {
                        format!("({})", expr)
                    }
                })
                .join(" OR ");

            // Re-parse as expression, in case this simplifies things.
            let license_string =
                SpdxExpression::parse(&initial_license_string).map(|expr| expr.to_string());

            // Use Debian names for licenses
            let license_string = license_string
                .unwrap_or(initial_license_string)
                .licenses_spdx_to_debian();

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
