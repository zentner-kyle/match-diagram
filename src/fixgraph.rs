use std::fmt;
use std::iter;
use std::slice;
use std::usize;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeIndex(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EdgeIndex(usize);

const INVALID_NODE_INDEX: NodeIndex = NodeIndex(usize::MAX);

pub struct FixGraph<N> {
    edges_per_node: usize,
    nodes: Vec<N>,
    edges: Vec<NodeIndex>,
}

impl<N> FixGraph<N> {
    pub fn with_capacity(capacity: usize, edges_per_node: usize) -> Self {
        FixGraph {
            edges_per_node,
            nodes: Vec::with_capacity(capacity),
            edges: Vec::with_capacity(capacity * edges_per_node),
        }
    }

    pub fn new(edges_per_node: usize) -> Self {
        Self::with_capacity(16, edges_per_node)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn push(&mut self, node: N) -> NodeIndex {
        let result = NodeIndex(self.nodes.len());
        self.nodes.push(node);
        self.edges
            .extend(iter::repeat(INVALID_NODE_INDEX).take(self.edges_per_node));
        result
    }

    fn edge_num_to_index(&self, node: NodeIndex, edge: EdgeIndex) -> usize {
        (node.0 / self.edges_per_node) + edge.0
    }

    pub fn set_edge(&mut self, source: NodeIndex, edge: EdgeIndex, target: NodeIndex) {
        if target.0 >= self.nodes.len() {
            panic!("target is outside of this FixGraph");
        }
        let idx = self.edge_num_to_index(source, edge);
        self.edges[idx] = target;
    }

    pub fn get_edge(&self, source: NodeIndex, edge: EdgeIndex) -> Option<NodeIndex> {
        let idx = self.edge_num_to_index(source, edge);
        // This one shouldn't need to be checked.
        let node = self.edges[idx];
        if node == INVALID_NODE_INDEX {
            None
        } else {
            Some(node)
        }
    }

    pub fn get_node(&self, node: NodeIndex) -> &N {
        &self.nodes[node.0]
    }

    pub fn get_node_mut(&mut self, node: NodeIndex) -> &mut N {
        &mut self.nodes[node.0]
    }

    pub fn iter(&self) -> NodeIter<N> {
        NodeIter {
            inner: self.nodes.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> NodeIterMut<N> {
        NodeIterMut {
            inner: self.nodes.iter_mut(),
        }
    }

    pub fn edge_iter(&self, node: NodeIndex) -> Edges<N> {
        Edges {
            graph: self,
            node,
            edge: EdgeIndex(0),
        }
    }

    pub fn group_iter(&self) -> GroupIter<N> {
        GroupIter {
            graph: self,
            node: NodeIndex(0),
        }
    }
}

impl<N: Clone> Clone for FixGraph<N> {
    fn clone(&self) -> Self {
        FixGraph {
            edges_per_node: self.edges_per_node,
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Group<'a, N: 'a> {
    pub node: &'a N,
    pub edges: Edges<'a, N>,
}

impl<N: fmt::Debug> fmt::Debug for FixGraph<N> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_list().entries(self.group_iter()).finish()
    }
}

pub struct Edges<'a, N: 'a> {
    graph: &'a FixGraph<N>,
    node: NodeIndex,
    edge: EdgeIndex,
}

// derive uses the wrong bounds for Edges.
// Specifically, it requires N: Clone.
// This is due to https://github.com/rust-lang/rust/issues/26925
// To work around this Clone is implemented manually for Edges.
impl<'a, N: 'a> Clone for Edges<'a, N> {
    fn clone(&self) -> Self {
        Edges {
            graph: self.graph,
            node: self.node,
            edge: self.edge,
        }
    }
}

impl<'a, N: 'a> Iterator for Edges<'a, N> {
    type Item = Option<NodeIndex>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.edge.0 >= self.graph.edges_per_node {
            None
        } else {
            let result = self.graph.get_edge(self.node, self.edge);
            self.edge = EdgeIndex(self.edge.0 + 1);
            Some(result)
        }
    }
}

impl<'a, N: 'a + fmt::Debug> fmt::Debug for Edges<'a, N> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_list().entries(self.clone()).finish()
    }
}

pub struct GroupIter<'a, N: 'a> {
    graph: &'a FixGraph<N>,
    node: NodeIndex,
}

impl<'a, N: 'a> Iterator for GroupIter<'a, N> {
    type Item = Group<'a, N>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.node.0 >= self.graph.len() {
            None
        } else {
            let result = Group {
                node: self.graph.get_node(self.node),
                edges: self.graph.edge_iter(self.node),
            };
            self.node = NodeIndex(self.node.0 + 1);
            Some(result)
        }
    }
}

pub struct NodeIter<'a, N: 'a> {
    inner: slice::Iter<'a, N>,
}

impl<'a, N: 'a> Iterator for NodeIter<'a, N> {
    type Item = &'a N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub struct NodeIterMut<'a, N: 'a> {
    inner: slice::IterMut<'a, N>,
}

impl<'a, N: 'a> Iterator for NodeIterMut<'a, N> {
    type Item = &'a mut N;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_create_graph() {
        let _ = FixGraph::<()>::new(0);
    }

    #[test]
    fn can_set_and_get_edge() {
        let mut g = FixGraph::<i32>::with_capacity(0, 1);
        let zero = g.push(0);
        let one = g.push(1);
        g.set_edge(zero, EdgeIndex(0), one);
        assert_eq!(Some(one), g.get_edge(zero, EdgeIndex(0)));
    }
}
