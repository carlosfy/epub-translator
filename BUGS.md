# Known Issues

This document lists known bugs and unexpected behaviors in our project or dependencies.

## 1. Inconsistent Text Node Iteration with kuchiki

**Library**: kuchiki 0.8.1

**Description**: When iterating over HTML text nodes, behavior differs based on iterator creation context and HTML structure.

**Example**: See `src/bin/bug_node_iterator.rs`

**Symptoms**:
- All nodes iterated when iterator used in creation scope
- Only first node (usually in `<head>`) iterated when returned from function
- Issue only occurs with `<head>` element present

**Workaround**: Create and use iterator in same scope, or pass entire `NodeRef`

**Notes**: kuchiki is no longer maintained

