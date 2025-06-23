# `crossfig`

Provides two macros to assist with managing conditional compilation in Rust.

- `no_std`
- _No_ `build.rs`
- _Zero_ features
- _Zero_ dependencies
- _No_ `proc-macros`

Instead, this crate defines 4 `macro_rules` macros:

- `switch`
- `alias`
- `enabled`
- `disabled`

## Tl;Dr

```rust
crossfig::alias! {
    /// Indicates whether the `std` feature is enabled.
    pub std: { #[cfg(feature = "std")] }
}

crossfig::switch! {
    std => {
        extern crate std;
        std::println!("Can use std!");
    }
    _ => {
        // no_std
    }
}
```

## `enabled` and `disabled`

`enabled` and `disabled` are the simplest to explain: they either pass their contents unchanged _or_ suppress them.

```rust
crossfig::enabled! {
    println!("I will print!");
}

crossfig::disabled! {
    compile_error!("I won't be triggered!");
}
```

If no content is provided to either macro, an appropriate `bool` value will be returned, allowing use in `if` statements and other logical operations.

```rust
if crossfig::enabled!() {
    println!("I will print!");
}

if crossfig::disabled!() {
    println!("I won't print!");
}
```

## `alias`

On their own, these macros aren't very helpful.
That's where `alias` comes in.
The `alias` macro will `use` either `enabled` or `disabled` with a new provided identifier based on an arbitrary conditional compilation configuration.

```rust
crossfig::alias! {
    std: { #[cfg(feature = "std")] }
}

// Is roughly equivalent to:
#[cfg(feature = "std")]
use crossfig::enabled as std;
#[cfg(not(feature = "std"))]
use crossfig::disabled as std;
```

Since the aliases created by `alias` are `macro_rules` items, they can be documented and even _publicly exported_.

```rust
crossfig::alias! {
    /// Indicates the log feature is enabled.
    pub log: { #[cfg(feature = "log")] }
}
```

When publicly exported, it's important to understand that an alias is evaluated as either `enabled` or `disabled` at the _definition_ site, not the _call_ site.
This means aliases can be used to determine what features are enabled in _dependencies_.

```rust
// In a crate `foo`
crossfig::alias! {
    /// Indicates the log feature is enabled.
    pub log: { #[cfg(feature = "log")] }
}

// In a consuming crate
if foo::log!() {
    // `foo`'s `log` feature has been enabled!
}
```

Note that aliases and standard `#[cfg(...)]` attributes can be mixed and matched within definitions, and combined with `not`, `any` and `all` operators:

```rust
crossfig::alias! {
    a: { #[cfg(feature = "a")] }
    b: { #[cfg(feature = "b")] }
    c: { all(a, not(b), #[cfg(feature = "c")]) }
}
```

While aliases are powerful, they still don't solve a common issue: ranked choice.
It's common to have multiple features in a crate which all contribute to a single choice.
For example, you may have a `parking_lot`, `std` and `spin` set of features to choose what `Mutex` implementation is used internally.

## `switch`

To make this particular problem nicer, we use the final macro in this crate, `switch`:

```rust
crossfig::alias! {
    parking_lot: { #[cfg(feature = "parking_lot")] }
    std: { #[cfg(feature = "std")] }
    spin: { #[cfg(feature = "spin")] }
}

crossfig::switch! {
    parking_lot => {
        use parking_lot::Mutex;
    }
    std => {
        use std::sync::Mutex;
    }
    spin => {
        use spin::Mutex;
    }
    _ => {
        compile_error!("Must select a `Mutex` provider!");
    }
}
```

## MSRV

The minimum supported Rust version for this crate is 1.54.0, with the 2015 edition, allowing it to be used in virtually any Rust project.
Note that support for earlier versions are blocked by the unavailability of `#![no_std]`, `vis` types in `macro_rules`, and `concat!` in documentation.
If support for earlier versions of Rust would help you, please create an issue!
