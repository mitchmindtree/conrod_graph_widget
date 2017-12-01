conrod_graph_widget [![Build Status](https://travis-ci.org/mitchmindtree/conrod_graph_widget.svg?branch=master)](https://travis-ci.org/mitchmindtree/conrod_graph_widget) [![Crates.io](https://img.shields.io/crates/v/conrod_graph_widget.svg)](https://crates.io/crates/conrod_graph_widget) [![Crates.io](https://img.shields.io/crates/l/conrod_graph_widget.svg)](https://github.com/mitchmindtree/conrod_graph_widget/blob/master/LICENSE-MIT) [![docs.rs](https://docs.rs/conrod_graph_widget/badge.svg)](https://docs.rs/conrod_graph_widget/)
===

A general use widget for viewing and controlling graphs.

Designed to be a foundation for node-graph GUIs similar in design to Max/MSP,
Pure Data, Touch Designer, etc.

Features
--------

- Allows for using arbitrary/custom widgets to represent each node and edge.
- Use any graph data structure, as long as you can provide an iterator yielding
  node identifiers and edges described via node identifier pairs.
- Provides `widget::Id`s to use for each node and edge within the graph.
- Yields events for adding and removing nodes and edges, dragging nodes,
  selections, etc.
