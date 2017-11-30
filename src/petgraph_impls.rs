//! `GraphType` implementations for the commonly used petgraph types.

use {Graph, Layout, NodeId};
use petgraph;
use std;

impl<'a, E, Ix> Graph<'a, petgraph::graph::NodeIndices<Ix>, GraphEdges<'a, E, Ix>>
where
    Ix: petgraph::csr::IndexType,
    petgraph::graph::NodeIndex<Ix>: NodeId,
{
    /// Construct a `Graph` widget for the given petgraph `Graph`.
    pub fn from_graph<N, Ty>(
        graph: &'a petgraph::Graph<N, E, Ty, Ix>,
        layout: &'a Layout<petgraph::graph::NodeIndex<Ix>>,
    ) -> Self
    where
        Ty: petgraph::EdgeType,
    {
        let node_indices = graph.node_indices();
        let edges = GraphEdges { edges: graph.raw_edges().iter() };
        Self::new(node_indices, edges, layout)
    }
}

/// An iterator yielding all edges within the graph.
#[derive(Clone)]
pub struct GraphEdges<'a, E: 'a, Ix: 'a> {
    edges: std::slice::Iter<'a, petgraph::graph::Edge<E, Ix>>,
}

impl<'a, E, Ix> Iterator for GraphEdges<'a, E, Ix>
where
    Ix: petgraph::csr::IndexType,
{
    type Item = (petgraph::graph::NodeIndex<Ix>, petgraph::graph::NodeIndex<Ix>);
    fn next(&mut self) -> Option<Self::Item> {
        self.edges
            .next()
            .map(|e| (e.source(), e.target()))
    }
}
