# josefine

> So perhaps we shall not miss so very much after all, while Josephine, for her part, delivered from earthly afflictions, which however to her mind are the privilege of chosen spirits, will happily lose herself in the countless throng of the heroes of our people, and soon, since we pursue no history, be accorded the heightened relief of being forgotten along with all her brethren.

\- Franz Kafka, *Josefine, die Sängerin oder Das Volk der Mäuse*

### Project Description

The purpose of this project is to explore the Rust Programming language and patterns in distributed programming by implementing a toy zK-less Kafka clone. Currently, there are no guaruntees that the code works or even compiles. The long term goal is to implement a subset of the Kafka consumer protocol, such that topics can be created and messages produced/consumed.

#### Crates

- `josefine-raft` is a [Raft](raft.github.io) implementation, that takes inspiration from [the following excellent blog post](https://hoverbear.org/blog/rust-state-machine-pattern/), as well as several other Raft implementations in Rust.
- `josefine-broker` defines the Kafka broker.
- `josefine-core` has shared common utilities.
- `josefine-kafka` includes Kafka specific protocol code.

#### TODO

- [x] Leader election
- [ ] State machine replication
- [ ] Kafka protocol
