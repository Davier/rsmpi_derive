# Derive macro for [rsmpi](https://github.com/bsteinb/rsmpi) trait `Equivalence`

The trait `mpi::datatype::Equivalence` describes a rust struct as a `MPI_Datatype` in order to synchronize it automatically.

This crate provides a derive macro to automatically implement it for plain structures composed recursively of:
- types that implement the `Equivalence` trait
- arrays of those types
- tuples of those types

# Example

The following example shows how to broadcast a struct.

```rust
use mpi::traits::*;
use mpi_derive::Equivalence;

#[derive(Equivalence, Default)]
struct ComplexDatatype {
    b: bool,
    ints: [i32; 4],
    tuple: ([f32; 2], u8),
}

fn main() {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();

    let root_process = world.process_at_rank(0);

    let mut data = if world.rank() == 0 {
        ComplexDatatype {
            b: true,
            ints: [1, -2, 3, -4],
            tuple: ([-0.1, 0.1], 7),
        }
    } else {
        ComplexDatatype::default()
    };

    root_process.broadcast_into(&mut data);

    assert_eq!(true, data.b);
    assert_eq!([1, -2, 3, -4], data.ints);
    assert_eq!([-0.1, 0.1], data.tuple.0);
    assert_eq!(7, data.tuple.1);
}
```

# Limitations

- Type aliases cannot be supported
- `enum`s are not implemented yet
