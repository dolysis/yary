
```rust
/*
00) ---
01) &key1 {                | A
02)   name1: &val1 data1,  | B C
03)    name2: *key1,       | D E
04)    name3: *val1,       | F G
05)    name4: [            | H J
06)      data2,            | K
07)      *key1,            | L
08)      *val1             | M
09)    ],
10) }
*/

enum Node<'a>
{
    // B:A C:A D:A F:A H:A K:J
    Leaf(ScalarNode<'a>),
    // A
    Map(MappingNode),
    // J:A
    List(SequenceNode),
    // E:A->A G:A->C L:J->A M:J->C
    Alias(AliasNode),
}
```
