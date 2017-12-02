use conrod::{widget, color, Color, Point, Positionable, Scalar, Sizeable, Widget};
use conrod::position::{Axis, Direction, Range, Rect};
use conrod::widget::primitive::shape::triangles::{ColoredPoint, Triangle};
use std::iter::once;
use std::ops::{Deref, DerefMut};

/// A widget that acts as a convenience container for some `Node`'s unique widgets.
///
/// 
#[derive(Clone, Debug, WidgetCommon)]
pub struct Node<W> {
    /// Data necessary and common for all widget builder types.
    #[conrod(common_builder)]
    pub common: widget::CommonBuilder,
    /// Unique styling for the **Node**.
    pub style: Style,
    /// The widget wrapped by this node container.
    pub widget: W,
    /// The number of input sockets on the node.
    pub inputs: usize,
    /// The number of output sockets on the node.
    pub outputs: usize,
}

pub const DEFAULT_BORDER_THICKNESS: Scalar = 6.0;
pub const DEFAULT_SOCKET_LENGTH: Scalar = DEFAULT_BORDER_THICKNESS;

/// Unique styling for the **BorderedRectangle** widget.
#[derive(Copy, Clone, Debug, Default, PartialEq, WidgetStyle)]
pub struct Style {
    /// Shape color for the inner rectangle.
    #[conrod(default = "color::TRANSPARENT")]
    pub color: Option<Color>,
    /// The length of each rectangle along its `SocketSide`.
    #[conrod(default = "6.0")]
    pub socket_length: Option<Scalar>,
    /// The widget of the border around the widget.
    ///
    /// this should always be a positive value in order for sockets to remain visible.
    #[conrod(default = "6.0")]
    pub border: Option<Scalar>,
    /// Color of the border.
    #[conrod(default = "color::DARK_CHARCOAL")]
    pub border_color: Option<Color>,
    /// Color of the sockets.
    #[conrod(default = "color::DARK_GREY")]
    pub socket_color: Option<Color>,
    /// Default layout for input sockets.
    #[conrod(default = "SocketLayout { side: SocketSide::Left, direction: Direction::Backwards }")]
    pub input_socket_layout: Option<SocketLayout>,
    /// Default layout for node output sockets.
    #[conrod(default = "SocketLayout { side: SocketSide::Right, direction: Direction::Backwards }")]
    pub output_socket_layout: Option<SocketLayout>,
}

/// Describes the layout of either input or output sockets.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SocketLayout {
    /// Represents the side of a node widget's bounding rectangle.
    pub side: SocketSide,
    /// The direction in which sockets will be laid out over the side.
    pub direction: Direction,
}

/// Represents the side of a node widget's bounding rectangle.
///
/// This is used to describe default node socket layout.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SocketSide {
    Left,
    Right,
    Top,
    Bottom,
}

widget_ids! {
    struct Ids {
        // Use triangles to describe graphics for the entire widget.
        //
        // The `Node` widget will be used a lot, so the less `widget::Id`s required the better.
        //
        // Triangulation order is as follows:
        //
        // 1. Inner rectangle surface (two triangles).
        // 2. Border (eight triangles).
        // 3. Sockets (two triangles per socket).
        triangles,
        // The unique identifier for the wrapped widget.
        widget,
    }
}

/// Unique state for the `Node`.
pub struct State {
    ids: Ids,
}

impl<W> Node<W> {
    /// Begin building a new `Node` widget.
    pub fn new(widget: W) -> Self {
        Node {
            common: widget::CommonBuilder::default(),
            style: Style::default(),
            widget,
            inputs: 0,
            outputs: 0,
        }
    }

    /// Specify the number of input sockets for the node.
    pub fn inputs(mut self, inputs: usize) -> Self {
        self.inputs = inputs;
        self
    }

    /// Specify the number of output sockets for the node.
    pub fn outputs(mut self, outputs: usize) -> Self {
        self.outputs = outputs;
        self
    }

    /// Specify the color for the node's inner rectangle.
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = Some(color);
        self
    }

    /// The thickness of the border around the inner widget.
    ///
    /// This must always be a positive value in order for sockets to remain visible.
    pub fn border_thickness(mut self, thickness: Scalar) -> Self {
        assert!(thickness > 0.0);
        self.style.border = Some(thickness);
        self
    }

    /// Specify the color for the node's border.
    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    /// Specify the layout of the input sockets.
    pub fn input_socket_layout(mut self, layout: SocketLayout) -> Self {
        self.style.input_socket_layout = Some(layout);
        self
    }

    /// Specify the layout of the input sockets.
    pub fn output_socket_layout(mut self, layout: SocketLayout) -> Self {
        self.style.output_socket_layout = Some(layout);
        self
    }
}

/// The event produced by 
#[derive(Clone, Debug)]
pub struct Event<W> {
    /// The event produced by the inner widget `W`.
    pub widget_event: W,
}

impl<W> Deref for Event<W> {
    type Target = W;
    fn deref(&self) -> &Self::Target {
        &self.widget_event
    }
}

impl<W> DerefMut for Event<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget_event
    }
}

impl<W> Widget for Node<W>
where
    W: Widget,
{
    type State = State;
    type Style = Style;
    type Event = Event<W::Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, style, rect, ui, .. } = args;
        let Node { widget, inputs, outputs, .. } = self;
        let socket_length = style.socket_length(&ui.theme);
        let border = style.border(&ui.theme);

        // The triangles for the inner rectangle surface first.
        let inner_rect = rect.pad(border);
        let (inner_tri_a, inner_tri_b) = widget::primitive::shape::rectangle::triangles(inner_rect);
        let inner_triangles = once(inner_tri_a).chain(once(inner_tri_b));

        // Triangles for the border.
        let border_triangles = widget::bordered_rectangle::border_triangles(rect, border).unwrap();

        // Axis from a given side and the scalar offset from the centre.
        let side_axis_and_scalar = |side| match side {
            SocketSide::Left => (Axis::Y, rect.left() + border / 2.0),
            SocketSide::Right => (Axis::Y, rect.right() - border / 2.0),
            SocketSide::Bottom => (Axis::X, rect.bottom() + border / 2.0),
            SocketSide::Top => (Axis::X, rect.top() - border / 2.0),
        };

        // A socket rectangle given some side.
        let socket_rect_dim = |axis| match axis {
            Axis::Y => [border, socket_length],
            Axis::X => [socket_length, border],
        };

        // A multiplier for the scalar direction.
        fn direction_scalar(direction: Direction) -> Scalar {
            match direction {
                Direction::Forwards => 1.0,
                Direction::Backwards => -1.0,
            }
        }

        // The range along which socket positions can be placed.
        let socket_range = |axis| -> Range {
            match axis {
                Axis::X => inner_rect.x,
                Axis::Y => inner_rect.y,
            }
        };

        // The gap between each socket.
        let socket_step_and_start = |n_sockets, axis, direction, side_scalar| -> ([Scalar; 2], Point) {
            let direction_scalar = direction_scalar(direction);
            let socket_range = socket_range(axis);
            let socket_position_range = socket_range.pad(socket_length / 2.0);
            let socket_start_scalar = match direction {
                Direction::Forwards => socket_position_range.start,
                Direction::Backwards => socket_position_range.end,
            };
            let step = socket_position_range.len() * direction_scalar / (n_sockets - 1) as Scalar;
            let (step, socket_start_position) = match axis {
                Axis::X => {
                    let step = [step, 0.0];
                    let x = socket_start_scalar;
                    let y = side_scalar;
                    (step, [x, y])
                },
                Axis::Y => {
                    let step = [0.0, step];
                    let x = side_scalar;
                    let y = socket_start_scalar;
                    (step, [x, y])
                },
            };
            (step, socket_start_position)
        };

        // The position of the socket at the given index.
        fn socket_position(index: usize, start_pos: Point, step: [Scalar; 2]) -> Point {
            let x = start_pos[0] + step[0] * index as Scalar;
            let y = start_pos[1] + step[1] * index as Scalar;
            [x, y]
        }

        // A function for producing the triangles of sockets along some axis.
        let socket_triangles = |n_sockets, SocketLayout { side, direction }| {
            let (axis, side_scalar) = side_axis_and_scalar(side);
            let (step, start_pos) = socket_step_and_start(n_sockets, axis, direction, side_scalar);
            let socket_dim = socket_rect_dim(axis);
            (0..n_sockets)
                .flat_map(move |i| {
                    let xy = socket_position(i, start_pos, step);
                    let rect = Rect::from_xy_dim(xy, socket_dim);
                    let (tri_a, tri_b) = widget::primitive::shape::rectangle::triangles(rect);
                    once(tri_a).chain(once(tri_b))
                })
        };

        // Triangles for sockets.
        let input_socket_layout = style.input_socket_layout(&ui.theme);
        let output_socket_layout = style.output_socket_layout(&ui.theme);
        let input_socket_triangles = socket_triangles(inputs, input_socket_layout);
        let output_socket_triangles = socket_triangles(outputs, output_socket_layout);

        // Colors the given triangle with the given color.
        fn color_triangle(Triangle(arr): Triangle<Point>, color: color::Rgba) -> Triangle<ColoredPoint> {
            Triangle([(arr[0], color), (arr[1], color), (arr[2], color)])
        }

        // Retrieve colours from the style.
        let inner_color = style.color(&ui.theme).into();
        let border_color = style.border_color(&ui.theme).into();
        let socket_color = style.socket_color(&ui.theme).into();

        // Submit the triangles for the graphical elements of the widget.
        let triangles = inner_triangles.map(|tri| color_triangle(tri, inner_color))
            .chain(border_triangles.iter().cloned().map(|tri| color_triangle(tri, border_color)))
            .chain(input_socket_triangles.map(|tri| color_triangle(tri, socket_color)))
            .chain(output_socket_triangles.map(|tri| color_triangle(tri, socket_color)));
        widget::Triangles::multi_color(triangles)
            .with_bounding_rect(rect)
            .graphics_for(id)
            .parent(id)
            .set(state.ids.triangles, ui);

        // Instantiate the widget.
        let widget_event = widget
            .wh(inner_rect.dim())
            .xy(inner_rect.xy())
            .parent(id)
            .set(state.ids.widget, ui);

        Event { widget_event }
    }
}
