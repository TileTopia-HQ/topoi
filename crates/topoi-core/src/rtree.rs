use crate::Envelope;
use crate::geometry::Coord;

/// An R-tree for spatial indexing of 2D bounding boxes.
///
/// Supports bulk-loading (STR packing) and query operations
/// (search, nearest neighbor).
#[derive(Debug, Clone)]
pub struct RTree {
    nodes: Vec<RTreeNode>,
    item_envelopes: Vec<Envelope>,
    root: usize,
}

#[derive(Debug, Clone)]
struct RTreeNode {
    envelope: Envelope,
    children: Vec<usize>, // indices into nodes array (for internal nodes)
    items: Vec<usize>,    // indices into original items (for leaf nodes)
    is_leaf: bool,
}

impl RTree {
    /// Build an R-tree from a set of envelopes using STR bulk-loading.
    ///
    /// # Arguments
    /// * `envelopes` — bounding boxes of the items to index
    /// * `max_children` — maximum entries per node (typically 8-16)
    pub fn bulk_load(envelopes: &[Envelope], max_children: usize) -> Self {
        let max_children = max_children.max(2);

        if envelopes.is_empty() {
            let root_node = RTreeNode {
                envelope: Envelope::new(0.0, 0.0, 0.0, 0.0),
                children: Vec::new(),
                items: Vec::new(),
                is_leaf: true,
            };
            return Self {
                nodes: vec![root_node],
                item_envelopes: Vec::new(),
                root: 0,
            };
        }

        let mut nodes = Vec::new();
        let indices: Vec<usize> = (0..envelopes.len()).collect();
        let root = str_pack(envelopes, &indices, max_children, &mut nodes);

        Self {
            nodes,
            item_envelopes: envelopes.to_vec(),
            root,
        }
    }

    /// Build an R-tree with default fanout (9).
    pub fn new(envelopes: &[Envelope]) -> Self {
        Self::bulk_load(envelopes, 9)
    }

    /// Search for all items whose envelope intersects the query envelope.
    pub fn search(&self, query: &Envelope) -> Vec<usize> {
        let mut results = Vec::new();
        if self.nodes.is_empty() {
            return results;
        }
        self.search_node(self.root, query, &mut results);
        results
    }

    fn search_node(&self, node_idx: usize, query: &Envelope, results: &mut Vec<usize>) {
        let node = &self.nodes[node_idx];
        if !node.envelope.intersects(query) {
            return;
        }

        if node.is_leaf {
            for &item_idx in &node.items {
                if self.item_envelopes[item_idx].intersects(query) {
                    results.push(item_idx);
                }
            }
        } else {
            for &child in &node.children {
                self.search_node(child, query, results);
            }
        }
    }

    /// Find the k nearest items to a query point.
    pub fn nearest(&self, point: &Coord, k: usize) -> Vec<(usize, f64)> {
        let mut candidates: Vec<(usize, f64)> = Vec::new();
        if self.nodes.is_empty() {
            return candidates;
        }
        self.nearest_node(self.root, point, k, &mut candidates);
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        candidates.truncate(k);
        candidates
    }

    fn nearest_node(
        &self,
        node_idx: usize,
        point: &Coord,
        k: usize,
        candidates: &mut Vec<(usize, f64)>,
    ) {
        let node = &self.nodes[node_idx];

        if node.is_leaf {
            for &item_idx in &node.items {
                let dist = self.item_envelopes[item_idx].distance_to_point(point);
                candidates.push((item_idx, dist));
                candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                if candidates.len() > k * 2 {
                    candidates.truncate(k);
                }
            }
        } else {
            // Sort children by distance to point
            let mut child_dists: Vec<(usize, f64)> = node
                .children
                .iter()
                .map(|&c| (c, self.nodes[c].envelope.distance_to_point(point)))
                .collect();
            child_dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

            for (child_idx, child_dist) in child_dists {
                // Prune: if we have k candidates and this child is farther than the k-th
                if candidates.len() >= k {
                    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    if child_dist > candidates[k - 1].1 {
                        break;
                    }
                }
                self.nearest_node(child_idx, point, k, candidates);
            }
        }
    }

    /// Get the bounding envelope of all items.
    pub fn envelope(&self) -> &Envelope {
        &self.nodes[self.root].envelope
    }

    /// Number of indexed items.
    pub fn len(&self) -> usize {
        self.count_items(self.root)
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn count_items(&self, node_idx: usize) -> usize {
        let node = &self.nodes[node_idx];
        if node.is_leaf {
            node.items.len()
        } else {
            node.children.iter().map(|&c| self.count_items(c)).sum()
        }
    }
}

/// Sort-Tile-Recursive packing algorithm.
fn str_pack(
    envelopes: &[Envelope],
    indices: &[usize],
    max_children: usize,
    nodes: &mut Vec<RTreeNode>,
) -> usize {
    if indices.len() <= max_children {
        // Create a leaf node
        let mut env = envelopes[indices[0]];
        for &i in &indices[1..] {
            env = env.merge(&envelopes[i]);
        }
        let node = RTreeNode {
            envelope: env,
            children: Vec::new(),
            items: indices.to_vec(),
            is_leaf: true,
        };
        let idx = nodes.len();
        nodes.push(node);
        return idx;
    }

    // Sort by x-center, then split into sqrt(n/M) slices
    let num_slices = ((indices.len() as f64) / (max_children as f64))
        .sqrt()
        .ceil() as usize;
    let slice_size = indices.len().div_ceil(num_slices);

    let mut sorted_x: Vec<usize> = indices.to_vec();
    sorted_x.sort_by(|&a, &b| {
        let ca = envelopes[a].center_x();
        let cb = envelopes[b].center_x();
        ca.partial_cmp(&cb).unwrap()
    });

    let mut child_indices = Vec::new();

    for x_slice in sorted_x.chunks(slice_size) {
        // Sort each x-slice by y-center
        let mut sorted_y: Vec<usize> = x_slice.to_vec();
        sorted_y.sort_by(|&a, &b| {
            let ca = envelopes[a].center_y();
            let cb = envelopes[b].center_y();
            ca.partial_cmp(&cb).unwrap()
        });

        // Split into groups of max_children
        for group in sorted_y.chunks(max_children) {
            let child_idx = str_pack(envelopes, group, max_children, nodes);
            child_indices.push(child_idx);
        }
    }

    // If we have too many children, recurse
    if child_indices.len() <= max_children {
        let mut env = nodes[child_indices[0]].envelope;
        for &c in &child_indices[1..] {
            env = env.merge(&nodes[c].envelope);
        }
        let node = RTreeNode {
            envelope: env,
            children: child_indices,
            items: Vec::new(),
            is_leaf: false,
        };
        let idx = nodes.len();
        nodes.push(node);
        idx
    } else {
        // Build envelopes for the child nodes and recurse
        let child_envs: Vec<Envelope> = child_indices.iter().map(|&c| nodes[c].envelope).collect();
        let child_ids: Vec<usize> = (0..child_indices.len()).collect();
        // Re-pack the internal nodes
        str_pack_internal(&child_envs, &child_ids, &child_indices, max_children, nodes)
    }
}

/// Pack internal nodes when we have more children than max_children.
fn str_pack_internal(
    envelopes: &[Envelope],
    indices: &[usize],
    real_node_indices: &[usize],
    max_children: usize,
    nodes: &mut Vec<RTreeNode>,
) -> usize {
    if indices.len() <= max_children {
        let children: Vec<usize> = indices.iter().map(|&i| real_node_indices[i]).collect();
        let mut env = nodes[children[0]].envelope;
        for &c in &children[1..] {
            env = env.merge(&nodes[c].envelope);
        }
        let node = RTreeNode {
            envelope: env,
            children,
            items: Vec::new(),
            is_leaf: false,
        };
        let idx = nodes.len();
        nodes.push(node);
        return idx;
    }

    let num_slices = ((indices.len() as f64) / (max_children as f64))
        .sqrt()
        .ceil() as usize;
    let slice_size = indices.len().div_ceil(num_slices);

    let mut sorted_x: Vec<usize> = indices.to_vec();
    sorted_x.sort_by(|&a, &b| {
        envelopes[a]
            .center_x()
            .partial_cmp(&envelopes[b].center_x())
            .unwrap()
    });

    let mut child_node_indices = Vec::new();

    for x_slice in sorted_x.chunks(slice_size) {
        let mut sorted_y: Vec<usize> = x_slice.to_vec();
        sorted_y.sort_by(|&a, &b| {
            envelopes[a]
                .center_y()
                .partial_cmp(&envelopes[b].center_y())
                .unwrap()
        });

        for group in sorted_y.chunks(max_children) {
            let children: Vec<usize> = group.iter().map(|&i| real_node_indices[i]).collect();
            let mut env = nodes[children[0]].envelope;
            for &c in &children[1..] {
                env = env.merge(&nodes[c].envelope);
            }
            let node = RTreeNode {
                envelope: env,
                children,
                items: Vec::new(),
                is_leaf: false,
            };
            let idx = nodes.len();
            nodes.push(node);
            child_node_indices.push(idx);
        }
    }

    if child_node_indices.len() <= max_children {
        let mut env = nodes[child_node_indices[0]].envelope;
        for &c in &child_node_indices[1..] {
            env = env.merge(&nodes[c].envelope);
        }
        let node = RTreeNode {
            envelope: env,
            children: child_node_indices,
            items: Vec::new(),
            is_leaf: false,
        };
        let idx = nodes.len();
        nodes.push(node);
        idx
    } else {
        let child_envs: Vec<Envelope> = child_node_indices
            .iter()
            .map(|&c| nodes[c].envelope)
            .collect();
        let child_ids: Vec<usize> = (0..child_node_indices.len()).collect();
        str_pack_internal(
            &child_envs,
            &child_ids,
            &child_node_indices,
            max_children,
            nodes,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtree_search() {
        let envelopes = vec![
            Envelope::new(0.0, 0.0, 1.0, 1.0),
            Envelope::new(2.0, 2.0, 3.0, 3.0),
            Envelope::new(5.0, 5.0, 6.0, 6.0),
            Envelope::new(0.5, 0.5, 1.5, 1.5),
        ];
        let tree = RTree::new(&envelopes);
        assert_eq!(tree.len(), 4);

        // Query that hits items 0 and 3
        let query = Envelope::new(0.0, 0.0, 1.0, 1.0);
        let results = tree.search(&query);
        assert!(results.contains(&0));
        assert!(results.contains(&3));
        assert!(!results.contains(&2));
    }

    #[test]
    fn test_rtree_no_results() {
        let envelopes = vec![
            Envelope::new(0.0, 0.0, 1.0, 1.0),
            Envelope::new(2.0, 2.0, 3.0, 3.0),
        ];
        let tree = RTree::new(&envelopes);

        let query = Envelope::new(10.0, 10.0, 11.0, 11.0);
        let results = tree.search(&query);
        assert!(results.is_empty());
    }

    #[test]
    fn test_rtree_many_items() {
        // 100 items in a grid
        let mut envelopes = Vec::new();
        for i in 0..10 {
            for j in 0..10 {
                let x = i as f64 * 10.0;
                let y = j as f64 * 10.0;
                envelopes.push(Envelope::new(x, y, x + 1.0, y + 1.0));
            }
        }
        let tree = RTree::new(&envelopes);
        assert_eq!(tree.len(), 100);

        // Query a small area
        let query = Envelope::new(15.0, 15.0, 25.0, 25.0);
        let results = tree.search(&query);
        // Should find items at (1,1), (1,2), (2,1), (2,2) → indices 11, 12, 21, 22
        assert!(!results.is_empty());
        assert!(results.len() <= 10); // reasonable number
    }

    #[test]
    fn test_rtree_empty() {
        let tree = RTree::new(&[]);
        assert_eq!(tree.len(), 0);
        assert!(tree.is_empty());
        assert!(tree.search(&Envelope::new(0.0, 0.0, 1.0, 1.0)).is_empty());
    }

    #[test]
    fn test_rtree_nearest() {
        let envelopes = vec![
            Envelope::new(0.0, 0.0, 1.0, 1.0),     // center (0.5, 0.5)
            Envelope::new(10.0, 10.0, 11.0, 11.0), // center (10.5, 10.5)
            Envelope::new(2.0, 2.0, 3.0, 3.0),     // center (2.5, 2.5)
        ];
        let tree = RTree::new(&envelopes);

        let point = Coord::new(0.0, 0.0);
        let nearest = tree.nearest(&point, 2);
        assert_eq!(nearest.len(), 2);
        // Item 0 should be closest
        assert_eq!(nearest[0].0, 0);
    }
}
