# Learning rust by implementing things

Misc things implemented in rust to learn about the language and the misc things themselves.

## Datastructures

### BTree

B-tree with a variable `order` (max number of children of a node).

### RBTree

Left-leaning Red-Black Tree.

## Networking

### FramedTcpStream

Wrapper over a tcpstream for sending/receiving length preceded messages

### Sync Server

Message-based TCP Server using blocking io and threads.
Example usage: `examples/echo_sync_server`

### Async Server

Message-based TCP Server using asynchronous io (mio).
Example usage: `examples/echo_async_server`

### Echo client

Interactive and "benchmarking" client for use with the echo servers
