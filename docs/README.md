# yary

Yet Another Rust YAML, or _yary_, is a library for efficiently parsing YAML
document streams. It primary design goals are:

1. Full implementation of the YAML 1.2 schema
2. Ergonomic, easy to use, well documented API
3. Safe handling of untrusted inputs
4. Zero copy deserialization (where allowed)
5. Lazy evaluation of stream content

## Library status

**alpha**

This library is still in the early stages of development. It does have a fully
functional YAML 1.2 parser, but no high level bindings, in-memory graph
representation, or safety features.

It does expose a single low level API for iterating over YAML stream events, in
`lib/event`, although it is not expected that most users would directly rely on
this module's API, instead using higher level constructs.

## MSRV

**1.53**

We make no strong guarantees about when this number will jump, and it will be moved
as we consume features of newer Rust versions.
