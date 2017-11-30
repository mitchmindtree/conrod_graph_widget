#[macro_use] extern crate conrod;
#[macro_use] extern crate conrod_derive;
extern crate petgraph;

mod petgraph_impls;

use conrod::{color, widget, Color, Colorable, Point, Positionable, Scalar, Widget};
use conrod::utils::IterDiff;
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, Weak};

// /// Behaviour required by the **Graph** widget.
// pub trait GraphType {
//     /// The unique identifier type used to distinguish between different nodes.
//     type NodeId: Copy + Eq + Hash;
//     /// An iterator yielding a `NodeId` for every node that exists within the graph.
//     type NodeIds: Iterator<Item=Self::NodeId>;
//     /// An iterator yielding every edge within the graph.
//     type Edges: Iterator<Item=(Self::NodeId, Self::NodeId)>;
// 
//     /// Produce an iterator yielding a `NodeId` for every node in the graph.
//     fn node_ids(&self) -> Self::NodeIds;
//     /// Produce an iterator yielding every edge in the graph.
//     fn edges(&self) -> Self::Edges;
// }

/// Traits required by types that may be used as a graph node identifier.
///
/// This trait has a blanket implementation for all types that satisfy the bounds.
pub trait NodeId: 'static + Copy + Clone + PartialEq + Eq + Hash + Send {}
impl<T> NodeId for T where T: 'static + Copy + Clone + PartialEq + Eq + Hash + Send {}

/// Stores the layout of all nodes within the graph.
///
/// All positions are relative to the centre of the `Graph` widget.
///
/// Nodes can be moved by 
#[derive(Clone, Debug, PartialEq)]
pub struct Layout<NI>
where
    NI: Eq + Hash,
{
    map: HashMap<NI, Point>,
}

/// A widget used for visualising and manipulating **Graph** types.
#[derive(Clone, Debug, WidgetCommon)]
pub struct Graph<'a, N, E>
where
    N: Iterator,
    N::Item: NodeId,
    E: Iterator<Item=(N::Item, N::Item)>,
{
    /// Data necessary and common for all widget builder types.
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    /// Unique styling for the **BorderedRectangle**.
    pub style: Style,
    /// All nodes within the graph that the widget is to represent.
    pub nodes: N,
    /// All edges within the graph.
    pub edges: E,
    /// The position of each node within the graph.
    pub layout: &'a Layout<N::Item>,
}

// A list of `widget::Id`s for a specific type.
#[derive(Default)]
struct TypeWidgetIds {
    // The index of the next `widget::Id` to use for this type.
    next_index: usize,
    // The list of widget IDs.
    widget_ids: Vec<widget::Id>,
}

impl TypeWidgetIds {
    // Return the next `widget::Id` for a widget of the given type.
    //
    // If there are no more `Id`s available for the type, a new one will be generated from the
    // given `widget::id::Generator`.
    fn next_id(&mut self, generator: &mut widget::id::Generator) -> widget::Id {
        loop {
            match self.widget_ids.get(self.next_index).map(|&id| id) {
                None => self.widget_ids.push(generator.next()),
                Some(id) => {
                    self.next_index += 1;
                    break id;
                }
            }
        }
    }
}

// A mapping from types to their list of IDs.
#[derive(Default)]
struct WidgetIdMap {
    map: HashMap<TypeId, TypeWidgetIds>,
}

impl WidgetIdMap {
    // Resets the index for every `TypeWidgetIds` list to `0`.
    //
    // This should be called at the beginning of the `Graph` update to ensure each widget receives
    // a unique ID. If this is not called, the graph will request more and more `widget::Id`s every
    // update and quickly bloat the `Ui`'s inner widget graph.
    fn reset_indices(&mut self) {
        for type_widget_ids in self.map.values_mut() {
            type_widget_ids.next_index = 0;
        }
    }

    // Return the next `widget::Id` for a widget of the given type.
    //
    // If there are no more `Id`s available for the type, a new one will be generated from the
    // given `widget::id::Generator`.
    fn next_id<T>(&mut self, generator: &mut widget::id::Generator) -> widget::Id
    where
        T: Any,
    {
        let type_id = TypeId::of::<T>();
        let type_widget_ids = self.map.entry(type_id).or_insert_with(TypeWidgetIds::default);
        type_widget_ids.next_id(generator)
    }
}

/// An interaction has caused some event to occur.
//
// TODO:
//
// - Hovered near outlet.
// - Edge end hovered near an outlet?
#[derive(Clone, Debug, PartialEq)]
pub enum Event<NI> {
    /// Events associated with nodes.
    Node(NodeEvent<NI>),
    /// Events associated with edges.
    Edge(EdgeEvent<NI>),
}

/// Represents a socket connection on a node (can be input or output).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NodeSocket<NI> {
    id: NI,
    socket_index: usize,
}

/// Events related to adding and removing nodes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeEvent<NI> {
    /// The user attempted to remove the node with the given identifier.
    Remove(NI),
}

/// Events related to adding and removing edges.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EdgeEvent<NI> {
    /// The user has pressed the given node socket with the left mouse button to begin creating an
    /// edge.
    AddStart(NodeSocket<NI>),
    /// The user has attempted to create an edge between the two given node sockets.
    Add {
        start: NodeSocket<NI>,
        end: NodeSocket<NI>,
    },
    /// The user has cancelled creating an edge from the given socket.
    Cancelled(NodeSocket<NI>),
    /// The user has attempted to remove the edge connecting the two given sockets.
    Remove {
        start: NodeSocket<NI>,
        end: NodeSocket<NI>,
    },
}

/// Unique styling for the **BorderedRectangle** widget.
#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    /// Shape styling for the inner rectangle.
    #[conrod(default = "color::TRANSPARENT")]
    pub background_color: Option<Color>,
}

widget_ids! {
    struct Ids {
        // The rectangle over which all nodes are placed.
        background,
    }
}

// // The inner petgraph data structure used for managing graph state.
// type PGraph = petgraph::Graph<NodeInfo, Edge>;
//
// struct NodeInfo {
//     point: Point,
// }
// 
// struct Edge;

/// Unique state for the `BorderedRectangle`.
pub struct State<NI> {
    ids: Ids,
    //graph: PGraph,
    shared: Arc<Mutex<Shared<NI>>>,
}

// State shared between the **Graph**'s **State** and the returned **Session**.
struct Shared<NI> {
    // A queue of events collected during `set` so that they may be emitted during
    // **SessionEvents**.
    events: VecDeque<Event<NI>>,
    // A mapping from node IDs to their data.
    nodes: HashMap<NI, NodeInner>,
    // A list of indices, one for each node in the graph.
    node_ids: Vec<NI>,
    // A list of all edges where (a, b) represents the directed edge a -> b.
    edges: Vec<(NI, NI)>,
    // A map from type identifiers to available `widget::Id`s for those types.
    widget_id_map: WidgetIdMap,
}

/// The camera used to view the graph.
///
/// The camera supports 2D positioning and zoom.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Camera {
    // The position of the camera over the floorplan.
    //
    // [0.0, 0.0] - the centre of the graph.
    point: Point,
    // The higher the zoom, the closer the floorplan appears.
    //
    // The zoom can be multiplied by a distance in metres to get the equivalent distance as a GUI
    // scalar value.
    //
    // 1.0 - Original resolution.
    // 0.5 - 50% view.
    zoom: Scalar,
}

/// A context for moving through the modes of graph widget instantiation in a type-safe manner.
///
/// The **Session** is shared between 3 stages:
///
/// 1. **SessionEvents**: Emit all graph events that have occurred since the last instantiation.
/// 2. **SessionNodes**: Instantiate all node widgets in the graph.
/// 3. **SessionEdges**: Instantiate all edge widgets in the graph.
///
/// NOTE: This should allow for different instantiation orders, e.g: nodes then edges, all
/// connected components in topo order, edges then nodes, etc.
pub struct Session<NI> {
    /// The unique identifier used to instantiate the graph widget.
    graph_id: widget::Id,
    // State shared with the `Graph` widget.
    shared: Weak<Mutex<Shared<NI>>>,
}

/// The first stage of the graph's **Session** event.
pub struct SessionEvents<NI> {
    session: Session<NI>,
}

/// The second stage of the graph's **Session** event.
pub struct SessionNodes<NI> {
    session: Session<NI>,
}

/// The third stage of the graph's **Session** event.
pub struct SessionEdges<NI> {
    session: Session<NI>,
}

/// An iterator yielding all pending events.
pub struct Events<'a, NI> {
    shared: Arc<Mutex<Shared<NI>>>,
    // Bind the lifetime to the `SessionEvents` so the user can't leak the `Shared` state.
    lifetime: PhantomData<&'a ()>,
}

/// An iterator-like type yielding a `Node` for every node in the graph.
///
/// Each `Node` can be used for instantiating a widget for each node in the graph.
pub struct Nodes<'a, NI: 'a> {
    // Index into the `node_ids`, indicating which node we're up to.
    index: usize,
    shared: Arc<Mutex<Shared<NI>>>,
    // The `widget::Id` of the parent graph widget.
    graph_id: widget::Id,
    // Bind the lifetime to the `SessionNodes` so the user can't leak the `Shared` state.
    lifetime: PhantomData<&'a NI>,
}

// Node data stored within the 
#[derive(Copy, Clone)]
struct NodeInner {
    point: Point,
}

/// A context for a node yielded during the node instantiation stage.
///
/// This type can be used to:
///
/// 1. Get the position of the node via `point()`.
/// 2. Get the ID for this node via `node_id()`.
/// 3. Convert into a `NodeWidget` ready for instantiation within the `Ui` via `widget(a_widget)`.
pub struct Node<'a, NI: 'a> {
    node_id: NI,
    point: Point,
    // The `widget::Id` of the `Node`'s parent `Graph` widget.
    graph_id: widget::Id,
    shared: Arc<Mutex<Shared<NI>>>,
    // Bind the lifetime to the `SessionNodes` so the user can't leak the `Shared` state.
    lifetime: PhantomData<&'a NI>,
}

/// Returned when a `Node` is assigned a widget.
///
/// This intermediary type allows for accessing the `widget::Id` before the widget itself is
/// instantiated.
pub struct NodeWidget<'a, NI: 'a, W> {
    node: Node<'a, NI>,
    widget: W,
    // `None` if not yet requested the `WidgetIdMap`. `Some` if it has.
    widget_id: Cell<Option<widget::Id>>,
}

/// An iterator-like type yielding a `Node` for every node in the graph.
///
/// Each `Node` can be used for instantiating a widget for each node in the graph.
pub struct Edges<'a, NI: 'a> {
    // The index into the `shared.edges` `Vec` that for the next `Edge` that is to be yielded.
    index: usize,
    shared: Arc<Mutex<Shared<NI>>>,
    // The `widget::Id` of the parent graph widget.
    graph_id: widget::Id,
    // Bind the lifetime to the `SessionEdges` so the user can't leak the `Shared` state.
    lifetime: PhantomData<&'a ()>,
}

/// A context for an edge yielded during the edge instantiation stage.
///
/// Tyis type can 
pub struct Edge<'a, NI> {
    // The `widget::Id` of the `Edge`'s parent `Graph` widget.
    graph_id: widget::Id,
    // The data shared with the graph state, used to access the `WidgetIdMap`.
    shared: Arc<Mutex<Shared<NI>>>,
    // The start of the edge.
    start: (NI, Point),
    // The end of the edge.
    end: (NI, Point),
    // Bind the lifetime to the `SessionEdges` so the user can't leak the `Shared` state.
    lifetime: PhantomData<&'a ()>,
}

/// Returned when an `Edge` is assigned a widget.
///
/// This intermediary type allows for accessing the `widget::Id` before the widget itself is
/// instantiated.
pub struct EdgeWidget<'a, NI: 'a, W> {
    edge: Edge<'a, NI>,
    widget: W,
    // `None` if not yet requested the `WidgetIdMap`. `Some` if it has.
    widget_id: Cell<Option<widget::Id>>,
}

// impl<NI> Layout<NI>
// where
//     NI: Eq + Hash,
// {
//     /// The position of the node at the given node identifier.
//     pub fn get(&self, node_id: NI) -> Option<&Point> {
//         self.map.get(&node_id)
//     }
//     /// The position of the node at the given node identifier.
//     pub fn get_mut(&mut self, node_id: NI) -> Option<&mut Point> {
//         self.map.get_mut(&node_id)
//     }
// }

impl<NI> From<HashMap<NI, Point>> for Layout<NI>
where
    NI: Eq + Hash,
{
    fn from(map: HashMap<NI, Point>) -> Self {
        Layout { map }
    }
}

impl<NI> Into<HashMap<NI, Point>> for Layout<NI>
where
    NI: Eq + Hash,
{
    fn into(self) -> HashMap<NI, Point> {
        let Layout { map } = self;
        map
    }
}

impl<NI> SessionEvents<NI> {
    /// All events that have occurred since the last 
    pub fn events(&self) -> Events<NI> {
        let shared = self.session.shared.upgrade().expect("failed to access `Shared` state");
        Events { shared, lifetime: PhantomData }
    }

    /// Transition from the **SessionEvents** into **SessionNodes** for instantiating nodes.
    pub fn next(self) -> SessionNodes<NI> {
        let SessionEvents { session } = self;
        SessionNodes { session }
    }
}

impl<'a, NI> Iterator for Events<'a, NI> {
    type Item = Event<NI>;
    fn next(&mut self) -> Option<Self::Item> {
        self.shared.lock()
            .ok()
            .and_then(|mut guard| guard.events.pop_front())
    }
}

impl<NI> SessionNodes<NI> {
    /// Produce an iterator yielding a `Node` for each node present in the graph.
    pub fn nodes(&mut self) -> Nodes<NI> {
        let graph_id = self.session.graph_id;
        let shared = self.session.shared.upgrade().expect("failed to access `Shared` state");
        Nodes { index: 0, shared, graph_id, lifetime: PhantomData }
    }

    /// Transition from the **SessionNodes** into **SessionEdges** for instantiating edges.
    pub fn next(self) -> SessionEdges<NI> {
        let SessionNodes { session } = self;
        SessionEdges { session }
    }
}

impl<'a, NI> Iterator for Nodes<'a, NI>
where
    NI: NodeId,
{
    type Item = Node<'a, NI>;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.shared.lock()
            .ok()
            .and_then(|guard| {
                guard.node_ids
                    .get(index)
                    .and_then(|&id| guard.nodes.get(&id).map(|&inner| (id, inner)))
            })
            .map(|(node_id, NodeInner { point })| {
                Node {
                    node_id,
                    point,
                    graph_id: self.graph_id,
                    shared: self.shared.clone(),
                    lifetime: PhantomData,
                }
            })
    }
}

impl<NI> SessionEdges<NI> {
    /// Produce an iterator yielding an `Edge` for each node present in the graph.
    pub fn edges(&mut self) -> Edges<NI> {
        let graph_id = self.session.graph_id;
        let shared = self.session.shared.upgrade().expect("failed to access `Shared` state");
        Edges { index: 0, shared, graph_id, lifetime: PhantomData }
    }
}

impl<'a, NI> Iterator for Edges<'a, NI>
where
    NI: NodeId,
{
    type Item = Edge<'a, NI>;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        self.index += 1;
        self.shared.lock()
            .ok()
            .and_then(|guard| {
                guard.edges.get(index).and_then(|&(start_id, end_id)| {
                    guard.nodes.get(&start_id).and_then(|start_inner| {
                        guard.nodes.get(&end_id).map(|end_inner| Edge {
                            graph_id: self.graph_id,
                            shared: self.shared.clone(),
                            start: (start_id, start_inner.point),
                            end: (end_id, end_inner.point),
                            lifetime: PhantomData,
                        })
                    })
                })
            })
    }
}

impl<'a, NI> Node<'a, NI>
where
    NI: Copy,
{
    /// The unique identifier associated with this node.
    pub fn node_id(&self) -> NI {
        self.node_id
    }

    /// The location of the node.
    pub fn point(&self) -> Point {
        self.point
    }

    /// Specify the widget to use 
    pub fn widget<W>(self, widget: W) -> NodeWidget<'a, NI, W> {
        NodeWidget {
            node: self,
            widget,
            widget_id: Cell::new(None),
        }
    }
}

impl<'a, NI, W> NodeWidget<'a, NI, W>
where
    W: 'static + Widget,
{
    /// Retrieve the `widget::Id` that will be used to instantiate this node's widget.
    pub fn widget_id(&self, ui: &mut conrod::UiCell) -> widget::Id {
        match self.widget_id.get() {
            Some(id) => id,
            None => {
                // Request a `widget::Id` from the `WidgetIdMap`.
                let mut shared = self.node.shared.lock().unwrap();
                let id = shared.widget_id_map.next_id::<W>(&mut ui.widget_id_generator());
                self.widget_id.set(Some(id));
                id
            },
        }
    }

    /// Map over the inner widget.
    pub fn map<M>(self, map: M) -> Self
    where
        M: FnOnce(W) -> W,
    {
        let NodeWidget { node, mut widget, widget_id } = self;
        widget = map(widget);
        NodeWidget { node, widget, widget_id }
    }

    /// Set the given widget for the node at `node_id()`.
    pub fn set(self, ui: &mut conrod::UiCell) -> W::Event {
        let widget_id = self.widget_id(ui);
        let NodeWidget { node, widget, .. } = self;
        widget
            .xy_relative_to(node.graph_id, node.point)
            .parent(node.graph_id)
            .set(widget_id, ui)
    }
}

impl<'a, NI, W> std::ops::Deref for NodeWidget<'a, NI, W> {
    type Target = Node<'a, NI>;
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<'a, NI> Edge<'a, NI>
where
    NI: NodeId,
{
    /// The start (or "input") for the edge.
    ///
    /// This is described via the node's `Id` and the position of its output socket.
    pub fn start(&self) -> (NI, Point) {
        self.start
    }

    /// The end (or "output") for the edge.
    ///
    /// This is described via the node's `Id` and the position of its input socket.
    pub fn end(&self) -> (NI, Point) {
        self.end
    }

    /// Calls `widget` with a straight `Line` widget.
    pub fn straight_line(self) -> EdgeWidget<'a, NI, widget::Line> {
        let (_, start_point) = self.start;
        let (_, end_point) = self.end;
        let line = widget::Line::abs(start_point, end_point);
        self.widget(line)
    }

    /// Specify the widget to use 
    pub fn widget<W>(self, widget: W) -> EdgeWidget<'a, NI, W> {
        EdgeWidget {
            edge: self,
            widget,
            widget_id: Cell::new(None),
        }
    }
}

impl<'a, NI, W> EdgeWidget<'a, NI, W>
where
    W: 'static + Widget,
{
    /// Retrieve the `widget::Id` that will be used to instantiate this edge's widget.
    pub fn widget_id(&self, ui: &mut conrod::UiCell) -> widget::Id {
        match self.widget_id.get() {
            Some(id) => id,
            None => {
                // Request a `widget::Id` from the `WidgetIdMap`.
                let mut shared = self.edge.shared.lock().unwrap();
                let id = shared.widget_id_map.next_id::<W>(&mut ui.widget_id_generator());
                self.widget_id.set(Some(id));
                id
            },
        }
    }

    /// Map over the inner widget.
    pub fn map<M>(self, map: M) -> Self
    where
        M: FnOnce(W) -> W,
    {
        let EdgeWidget { edge, mut widget, widget_id } = self;
        widget = map(widget);
        EdgeWidget { edge, widget, widget_id }
    }

    /// Set the given widget for the edge.
    pub fn set(self, ui: &mut conrod::UiCell) -> W::Event {
        let widget_id = self.widget_id(ui);
        let EdgeWidget { edge, widget, .. } = self;
        widget
            .parent(edge.graph_id)
            .set(widget_id, ui)
    }
}

impl<'a, N, E> Graph<'a, N, E>
where
    N: Iterator,
    N::Item: NodeId,
    E: Iterator<Item=(N::Item, N::Item)>,
{
    /// Begin building a new **Graph** widget.
    pub fn new<NI, EI>(nodes: NI, edges: EI, layout: &'a Layout<NI::Item>) -> Self
    where
        NI: IntoIterator<IntoIter=N, Item=N::Item>,
        EI: IntoIterator<IntoIter=E, Item=(N::Item, N::Item)>,
    {
        Graph {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            nodes: nodes.into_iter(),
            edges: edges.into_iter(),
            layout: layout,
        }
    }

    /// Color the **Graph**'s rectangular area with the given color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.style.background_color = Some(color);
        self
    }
}

impl<'a, N, E> Widget for Graph<'a, N, E>
where
    N: Iterator,
    N::Item: NodeId,
    E: Iterator<Item=(N::Item, N::Item)>,
{
    type State = State<N::Item>;
    type Style = Style;
    type Event = SessionEvents<N::Item>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        let events = VecDeque::new();
        let nodes = HashMap::new();
        let node_ids = Vec::new();
        let edges = Vec::new();
        let widget_id_map = WidgetIdMap { map: HashMap::new() };
        let shared = Shared { events, nodes, node_ids, edges, widget_id_map };
        State {
            ids: Ids::new(id_gen),
            shared: Arc::new(Mutex::new(shared)),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, style, rect, ui, .. } = args;
        let Graph { nodes, edges, layout, .. } = self;
        let mut shared = state.shared.lock().unwrap();

        // Reset the WidgetIdMap indices.
        shared.widget_id_map.reset_indices();

        // Compare the existing node indices with the new iterator.
        match conrod::utils::iter_diff(&shared.node_ids, nodes) {
            Some(diff) => match diff {
                IterDiff::FirstMismatch(i, mismatch) => {
                    shared.node_ids.truncate(i);
                    shared.node_ids.extend(mismatch);
                },
                IterDiff::Longer(remaining) => {
                    shared.node_ids.extend(remaining);
                },
                IterDiff::Shorter(total) => {
                    shared.node_ids.truncate(total);
                },
            },
            None => (),
        }

        // Compare the existing edges with the new iterator.
        match conrod::utils::iter_diff(&shared.edges, edges) {
            Some(diff) => match diff {
                IterDiff::FirstMismatch(i, mismatch) => {
                    shared.edges.truncate(i);
                    shared.edges.extend(mismatch);
                },
                IterDiff::Longer(remaining) => {
                    shared.edges.extend(remaining);
                },
                IterDiff::Shorter(total) => {
                    shared.edges.truncate(total);
                },
            },
            None => (),
        }

        // Use `shared.node_ids` and `shared.edges` to fill `shared.nodes`.
        shared.nodes.clear();
        for i in 0..shared.node_ids.len() {
            let node_id = shared.node_ids[i];
            let point = layout.map.get(&node_id).map(|&p| p).unwrap_or([0.0; 2]);
            let node = NodeInner { point };
            shared.nodes.insert(node_id, node);
        }

        // TODO: Drag widgets around (use a map from `node_id` -> `widget_id`). Generate an event
        // for each drag event and make it easy to use events to update `layout`.

        let background_color = style.background_color(&ui.theme);
        widget::Rectangle::fill(rect.dim())
            .xy(rect.xy())
            .color(background_color)
            .parent(id)
            .graphics_for(id)
            .set(state.ids.background, ui);

        let graph_id = id;
        let shared = Arc::downgrade(&state.shared);
        let session = Session {
            graph_id,
            shared,
        };
        SessionEvents { session }
    }
}
