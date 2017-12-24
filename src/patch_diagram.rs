use diagram::{Diagram, Node};
use fixgraph::NodeIndex;
use graph_diagram::GraphDiagram;
use tiny_map;
use tiny_map::TinyMap;

#[derive(Clone, Debug)]
pub struct PatchDiagram<'a> {
    graph_diagram: &'a GraphDiagram,
    next_node: usize,
    node_map: TinyMap<NodeIndex, Node>,
    match_targets: TinyMap<NodeIndex, Option<NodeIndex>>,
    refute_targets: TinyMap<NodeIndex, Option<NodeIndex>>,
    match_sources: TinyMap<NodeIndex, Vec<NodeIndex>>,
    refute_sources: TinyMap<NodeIndex, Vec<NodeIndex>>,
}

impl<'a> PatchDiagram<'a> {
    pub fn new(graph_diagram: &'a GraphDiagram) -> Self {
        PatchDiagram {
            graph_diagram,
            next_node: graph_diagram.len(),
            node_map: TinyMap::new(),
            match_targets: TinyMap::new(),
            refute_targets: TinyMap::new(),
            match_sources: TinyMap::new(),
            refute_sources: TinyMap::new(),
        }
    }
}

fn remove_source(
    sources: &mut TinyMap<NodeIndex, Vec<NodeIndex>>,
    src: NodeIndex,
    target: NodeIndex,
) {
    let sources = sources
        .get_mut(&target)
        .expect("Should only be removing source which exists");
    let index = sources
        .iter()
        .position(|&s| s == src)
        .expect("src should be present in the sources of target");
    sources.remove(index);
}

fn set_target<'a, F: FnOnce(NodeIndex) -> &'a [NodeIndex], G: FnOnce() -> Option<NodeIndex>>(
    targets: &mut TinyMap<NodeIndex, Option<NodeIndex>>,
    sources: &mut TinyMap<NodeIndex, Vec<NodeIndex>>,
    diagram_old_target_sources: F,
    diagram_old_target: G,
    src: NodeIndex,
    target: Option<NodeIndex>,
) {
    match targets.entry(src) {
        tiny_map::Entry::Occupied(mut entry) => {
            if let Some(old_target) = *entry.get() {
                remove_source(sources, src, old_target);
            }
            *entry.get_mut() = target;
        }
        tiny_map::Entry::Vacant(entry) => {
            if let Some(old_target) = diagram_old_target() {
                let mut old_target_sources = diagram_old_target_sources(old_target).to_owned();
                let index = old_target_sources
                    .iter()
                    .position(|&s| s == src)
                    .expect("src should be present in the sources of target");
                old_target_sources.remove(index);
                sources.insert(old_target, old_target_sources);
            }
            entry.insert(target);
        }
    }
}

fn set_sources<'a, F: FnOnce() -> Option<&'a [NodeIndex]>>(
    sources: &mut TinyMap<NodeIndex, Vec<NodeIndex>>,
    diagram_sources: F,
    src: NodeIndex,
    target: NodeIndex,
) {
    match sources.entry(target) {
        tiny_map::Entry::Occupied(mut entry) => {
            entry.get_mut().push(src);
        }
        tiny_map::Entry::Vacant(entry) => {
            let mut sources = diagram_sources()
                .map(|s| s.to_owned())
                .unwrap_or_else(|| Vec::new());
            sources.push(src);
            entry.insert(sources);
        }
    }
}

impl<'a> Diagram for PatchDiagram<'a> {
    fn insert_node(&mut self, node: Node) -> NodeIndex {
        let node_index = NodeIndex(self.next_node);
        self.next_node += 1;
        self.node_map.insert(node_index, node);
        node_index
    }

    fn get_node(&self, index: NodeIndex) -> &Node {
        if let Some(node) = self.node_map.get(&index) {
            node
        } else {
            self.graph_diagram.get_node(index)
        }
    }

    fn get_node_mut(&mut self, index: NodeIndex) -> &mut Node {
        match self.node_map.entry(index) {
            tiny_map::Entry::Occupied(entry) => entry.into_mut(),
            tiny_map::Entry::Vacant(entry) => {
                let node = self.graph_diagram.get_node(index);
                entry.insert(node.clone())
            }
        }
    }

    fn set_on_match(&mut self, src: NodeIndex, target: NodeIndex) {
        let diagram = self.graph_diagram;
        set_target(
            &mut self.match_targets,
            &mut self.match_sources,
            |target| diagram.get_match_sources(target).unwrap(),
            || diagram.get_on_match(src),
            src,
            Some(target),
        );
        set_sources(
            &mut self.match_sources,
            || diagram.get_match_sources(target),
            src,
            target,
        );
    }

    fn set_on_refute(&mut self, src: NodeIndex, target: NodeIndex) {
        let diagram = self.graph_diagram;
        set_target(
            &mut self.refute_targets,
            &mut self.refute_sources,
            |target| diagram.get_refute_sources(target).unwrap(),
            || diagram.get_on_refute(src),
            src,
            Some(target),
        );
        set_sources(
            &mut self.refute_sources,
            || diagram.get_refute_sources(target),
            src,
            target,
        );
    }

    fn clear_on_match(&mut self, src: NodeIndex) {
        let diagram = self.graph_diagram;
        set_target(
            &mut self.match_targets,
            &mut self.match_sources,
            |target| diagram.get_match_sources(target).unwrap(),
            || diagram.get_on_match(src),
            src,
            None,
        );
    }

    fn clear_on_refute(&mut self, src: NodeIndex) {
        let diagram = self.graph_diagram;
        set_target(
            &mut self.refute_targets,
            &mut self.refute_sources,
            |target| diagram.get_refute_sources(target).unwrap(),
            || diagram.get_on_refute(src),
            src,
            None,
        );
    }

    fn get_on_match(&self, src: NodeIndex) -> Option<NodeIndex> {
        if let Some(target) = self.match_targets.get(&src) {
            *target
        } else {
            self.graph_diagram.get_on_match(src)
        }
    }

    fn get_on_refute(&self, src: NodeIndex) -> Option<NodeIndex> {
        if let Some(target) = self.refute_targets.get(&src) {
            *target
        } else {
            self.graph_diagram.get_on_refute(src)
        }
    }

    fn len(&self) -> usize {
        self.next_node
    }

    fn get_match_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        if let Some(sources) = self.match_sources.get(&target) {
            Some(sources)
        } else {
            self.graph_diagram.get_match_sources(target)
        }
    }

    fn get_refute_sources(&self, target: NodeIndex) -> Option<&[NodeIndex]> {
        if let Some(sources) = self.refute_sources.get(&target) {
            Some(sources)
        } else {
            self.graph_diagram.get_refute_sources(target)
        }
    }

    fn get_num_registers(&self) -> usize {
        self.graph_diagram.get_num_registers()
    }
}
