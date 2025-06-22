//! Provides helpful configuration macros, allowing features to be introspected
//! across crates at compile time.
//!
//! The two main features that power this crate are [`switch`] and [`alias`].
//!
//! # `switch`
//!
//! [`switch`] provides a `match`-like syntax to make complex compile-time configuration
//! more ergonomic.
//!
//! ```standalone_crate
//! // Without `crossfig`:
//!
//! #[cfg(feature = "fastest")]
//! // Use the fastest algorithm!
//!
//! #[cfg(all(not(feature = "fastest"), feature = "fast"))]
//! // Use the fast algorithm
//!
//! #[cfg(all(not(feature = "fastest"), not(feature = "fast")))]
//! // Use the slow algorithm
//! ```
//!
//! ```standalone_crate
//! // With `crossfig`:
//!
//! crossfig::switch! {
//!     #[cfg(feature = "fastest")] => {
//!         // Use the fastest algorithm!
//!     }
//!     #[cfg(feature = "fast")] => {
//!         // Use the fast algorithm
//!     }
//!     _ => {
//!         // Use the slow algorithm
//!     }
//! }
//! ```
//!
//! # `alias`
//!
//! [`alias`] instead allows for creating a short-hand for common configurations.
//!
//! ```standalone_crate
//! # use crossfig::alias;
//! // Define an alias
//! alias! {
//!     std: { #[cfg(feature = "std")] }
//! }
//!
//! // Use it for conditional compilation
//! std! {
//!     // This code is only compiled with feature = "std"
//! #   ()
//! }
//! ```
//!
//! # Combined
//!
//! Individually, both are useful tools for reducing repetition in conditional compilation.
//! But where they really shine is when combined:
//!
//! ```standalone_crate
//! # use crossfig::{alias, switch};
//! alias! {
//!     std: { #[cfg(feature = "std")] },
//!     log: { #[cfg(feature = "log")] },
//! }
//!
//! switch! {
//!     log => {
//!         // Use the log crate
//!     }
//!     std => {
//!         // Use println! for logging
//!     }
//!     _ => {
//!         // Don't bother logging
//!     }
//! }
//! ```
//!
//! # Cross-Crate Configuration Introspection
//!
//! One last trick; aliases created with [`alias`] can be _exported_ in
//! your public API.
//!
//! ```standalone_crate
//! // In the `foo` crate:
//! # mod foo {
//! crossfig::alias! {
//!     pub multi_threading: { #[cfg(feature = "multi_threading")] }
//! }
//! # }
//!
//! // In a consumer's crate:
//! # mod bar {
//! # use super::foo;
//! foo::multi_threading! {
//!     // This code will be compiled if and only if foo/multi_threading
//!     // is enabled.
//! #   ()
//! }
//! # }
//! #
//! # fn main() {}
//! ```
//!
//! This can be extremely powerful, as it allows _consuming_ crates to introspect
//! on the enabled features of their _dependencies_.
//!
//! # `macro_rules_attribute`
//!
//! As a quality of life improvement, you can also use [`macro_rules_attribute`].
//! This provides an attribute proc-macro `apply`, which can be used to apply aliases
//! to items as an attribute.
//!
//! ```ignore
//! #![no_std]
//!
//! use macro_rules_attribute::apply;
//!
//! alias! {
//!     std: { #[cfg(feature = "std")] }
//! }
//!
//! #[apply(std)]
//! extern crate std;
//! ```
//!
//! [`macro_rules_attribute`]: https://docs.rs/macro_rules_attribute

#![no_std]
#![no_implicit_prelude]
#![forbid(unsafe_code)]
#![cfg_attr(crossfig_no_core, feature(no_core))]
#![cfg_attr(crossfig_no_core, no_core)]

/// Provides a `match`-like expression similar to [`cfg_if`] and based on the experimental
/// [`cfg_match`].
/// The name `switch` is used to avoid conflict with the `match` keyword.
/// Arms are evaluated top to bottom, and an optional wildcard arm can be provided if no match
/// can be made.
///
/// An arm can either be:
/// - a `cfg(...)` pattern (e.g., `feature = "foo"`)
/// - a wildcard `_`
/// - an alias defined using [`alias`]
///
/// Note that aliases are evaluated from the context of the defining crate, not the consumer.
/// This allows a library to export aliases for use in consuming crates.
///
/// # Examples
///
/// ```
/// # use crossfig::{switch, alias};
/// # fn log<T>(_: T) {}
/// # fn foo<T>(_: T) {}
/// # alias! {
/// #    std: { true }
/// # }
/// #
/// switch! {
///     #[cfg(feature = "foo")] => {
///         foo("We have the `foo` feature!")
///     }
///     std => {
///         extern crate std;
///         std::println!("No `foo`, but we have `std`!");
///     }
///     _ => {
///         log("Don't have `std` or `foo`");
///     }
/// }
/// ```
///
/// [`cfg_if`]: https://crates.io/crates/cfg-if
/// [`cfg_match`]: https://github.com/rust-lang/rust/issues/115585
#[macro_export]
macro_rules! switch {
    // Allow switch!{{ ... }} to act as an expression in certain contexts
    (
        { $($tt:tt)* }
    ) => {
        {
            $crate::switch! { $($tt)* }
        }
    };

    // Common mistake: arms after wildcard
    (
        _ => $output:tt
        $( $rest:tt )+
    ) => {
        compile_error!(concat!("patterns after a wildcard are ignored: `", stringify!($($cond)+), "`"));
    };

    // Wildcard branch
    (
        _ => { $($output:tt)* }
    ) => {
        $($output)*
    };

    // #[cfg(...)] integration
    (
        #[cfg($cfg:meta)] => $output:tt
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { }
            { #[cfg($cfg)] }
        );
    };
    (
        #[cfg($cfg:meta)] => $output:tt
        $( $rest:tt )+
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { $crate::switch! { $($rest)+ } }
            { #[cfg($cfg)] }
        );
    };

    // ops integration
    (
        $op:ident($($args:tt)*) => $output:tt
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { }
            { $op($($args)*) }
        );
    };
    (
        $op:ident($($args:tt)*) => $output:tt
        $( $rest:tt )+
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { $crate::switch! { $($rest)+ } }
            { $op($($args)*) }
        );
    };

    // alias integration
    (
        $cond:path => $output:tt
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { }
            { $cond }
        );
    };
    (
        $cond:path => $output:tt
        $( $rest:tt )+
    ) => {
        $crate::eval!(
            { $crate::switch! { _ => $output } }
            { $crate::switch! { $($rest)+ } }
            { $cond }
        );
    };
}

/// # Examples
///
/// ## As a `boolean`
///
/// ```
/// assert_eq!(crossfig::disabled!(), false);
/// ```
///
/// ## As a Conditional Compilation Guard
///
/// ```
/// crossfig::disabled! {
///     use a_crate_that_is_not_available::*;
///     // ...
/// }
/// ```
///
/// ## As an `if`-Statement
///
/// ```
/// crossfig::disabled! {
///     if {
///         let was_disabled = false;
///     } else {
///         let was_disabled = true;
///     }
/// }
///
/// assert!(was_disabled);
/// ```
#[macro_export]
macro_rules! disabled {
    () => { false };
    (if { $($p:tt)* } else { $($n:tt)* }) => { $($n)* };
    ($($p:tt)*) => {};
}

/// # Examples
///
/// ## As a `boolean`
///
/// ```
/// assert_eq!(crossfig::enabled!(), true);
/// ```
///
/// ## As a Conditional Compilation Guard
///
/// ```
/// crossfig::enabled! {
/// # /*
///     use a_crate_that_is_available::*;
/// # */
///     // ...
/// #   ()
/// }
/// ```
///
/// ## As an `if`-Statement
///
/// ```
/// crossfig::enabled! {
///     if {
///         let was_enabled = true;
///     } else {
///         let was_enabled = false;
///     }
/// }
///
/// assert!(was_enabled);
/// ```
#[macro_export]
macro_rules! enabled {
    () => { true };
    (if { $($p:tt)* } else { $($n:tt)* }) => { $($p)* };
    ($($p:tt)*) => { $($p)* };
}

/// Defines an alias for a particular configuration.
/// This has two advantages over directly using `#[cfg(...)]`:
///
/// 1. Complex configurations can be abbreviated to more meaningful shorthand.
/// 2. Features are evaluated in the context of the _defining_ crate, not the consuming.
///
/// The second advantage is a particularly powerful tool, as it allows consuming crates to use
/// functionality in a defining crate regardless of what crate in the dependency graph enabled the
/// relevant feature.
///
/// For example, consider a crate `foo` that depends on another crate `bar`.
/// `bar` has a feature "`faster_algorithms`".
/// If `bar` defines a "`faster_algorithms`" alias:
///
/// ```
/// # use crossfig::alias;
/// alias! {
///     pub faster_algorithms: {
///         #[cfg(feature = "faster_algorithms")]
///     }
/// }
/// ```
///
/// Now, `foo` can gate its usage of those faster algorithms on the alias, avoiding the need to
/// expose its own "`faster_algorithms`" feature.
/// This also avoids the unfortunate situation where one crate activates "`faster_algorithms`" on
/// `bar` without activating that same feature on `foo`.
///
/// Once an alias is defined, there are 4 ways you can use it:
///
/// 1. Evaluate with no contents to return a `bool` indicating if the alias is active.
///    ```
///    # use crossfig::alias;
///    # alias! {
///    #    std: { true }
///    # }
///    if std!() {
///        // Have `std`!
///    } else {
///        // No `std`...
///    }
///    ```
/// 2. Pass a single code block which will only be compiled if the alias is active.
///    ```
///    # use crossfig::alias;
///    # alias! {
///    #    std: { true }
///    # }
///    std! {
///        // Have `std`!
///    # ()
///    }
///    ```
/// 3. Pass a single `if { ... } else { ... }` expression to conditionally compile either the first
///    or the second code block.
///    ```
///    # use crossfig::alias;
///    # alias! {
///    #    std: { true }
///    # }
///    std! {
///        if {
///            // Have `std`!
///        } else {
///            // No `std`...
///        }
///    }
///    ```
/// 4. Use in a [`switch`] arm for more complex conditional compilation.
///    ```
///    # use crossfig::{alias, switch};
///    # alias! {
///    #    std: { true }
///    # }
///    switch! {
///        std => {
///            // Have `std`!
///        }
///        alloc => {
///            // No `std`, but do have `alloc`!
///        }
///        _ => {
///            // No `std` or `alloc`...
///        }
///    }
///    ```
#[macro_export]
macro_rules! alias {
    () => {};
    (
        $(#[$p_meta:meta])*
        $vis:vis $p:ident: { $($cond:tt)+ }
    ) => {
        $crate::alias! {
            $(#[$p_meta])*
            $vis $p: { $($cond)+ },
        }
    };
    (
        $(#[$p_meta:meta])*
        $vis:vis $p:ident: { $($cond:tt)+ },

        $($rest:tt)*
    ) => {
        $crate::eval!(
            {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro passes the provided code because `", stringify!($($cond)+), "` is currently active.")]
                $vis use $crate::enabled as $p;
            }
            {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro suppresses the provided code because `", stringify!($($cond)+), "` is _not_ currently active.")]
                $vis use $crate::disabled as $p;
            }
            { $($cond)+ }
        );

        $crate::alias! {
            $($rest)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! eval {
    ({ $($truthy:tt)* } { $($falsy:tt)* } { true }) => {
        $($truthy)*
    };
    ({ $($truthy:tt)* } { $($falsy:tt)* } { false }) => {
        $($falsy)*
    };

    ($truthy:tt $falsy:tt { _ }) => {
        $crate::eval!(
            $truthy
            $falsy
            { true }
        );
    };
    ($truthy:tt $falsy:tt { all() }) => {
        $crate::eval!(
            $truthy
            $falsy
            { true }
        );
    };
    ($truthy:tt $falsy:tt { any() }) => {
        $crate::eval!(
            $truthy
            $falsy
            { false }
        );
    };
    ($truthy:tt $falsy:tt { #[cfg($meta:meta)] }) => {
        #[cfg($meta)]
        $crate::eval!(
            $truthy
            $falsy
            { true }
        );

        #[cfg(not($meta))]
        $crate::eval!(
            $truthy
            $falsy
            { false }
        );
    };

    ($truthy:tt $falsy:tt { not($($cond:tt)*) }) => {
        $crate::eval!(
            $falsy
            $truthy
            { $($cond)* }
        );
    };

    ($truthy:tt $falsy:tt { all(#[cfg($meta:meta)]) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { all(#[cfg($meta)],) }
        );
    };
    ($truthy:tt $falsy:tt { all($op:ident($($cond:tt)*)) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { all($op($($cond)*),) }
        );
    };
    ($truthy:tt $falsy:tt { all($cond:path) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { all($cond,) }
        );
    };
    ($truthy:tt $falsy:tt { all(#[cfg($meta:meta)], $($rest:tt)*) }) => {
        $crate::eval!(
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { all($($rest)*) }
                );
            }
            $falsy
            { #[cfg($meta)] }
        );
    };
    ($truthy:tt $falsy:tt { all($op:ident($($cond:tt)*), $($rest:tt)*) }) => {
        $crate::eval!(
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { all($($rest)*) }
                );
            }
            $falsy
            { $op($($cond)*) }
        );
    };
    ($truthy:tt $falsy:tt { all($cond:path, $($rest:tt)*) }) => {
        $crate::eval!(
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { all($($rest)*) }
                );
            }
            $falsy
            { $cond }
        );
    };

    ($truthy:tt $falsy:tt { any(#[cfg($meta:meta)]) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { any(#[cfg($meta)],) }
        );
    };
    ($truthy:tt $falsy:tt { any($op:ident($($cond:tt)*)) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { any($op($($cond)*),) }
        );
    };
    ($truthy:tt $falsy:tt { any($cond:path) }) => {
        $crate::eval!(
            $truthy
            $falsy
            { any($cond,) }
        );
    };
    ($truthy:tt $falsy:tt { any(#[cfg($meta:meta)], $($rest:tt)*) }) => {
        $crate::eval!(
            $truthy
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { any($($rest)*) }
                );
            }
            { #[cfg($meta)] }
        );
    };
    ($truthy:tt $falsy:tt { any($op:ident($($cond:tt)*), $($rest:tt)*) }) => {
        $crate::eval!(
            $truthy
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { any($($rest)*) }
                );
            }
            { $op($($cond)*) }
        );
    };
    ($truthy:tt $falsy:tt { any($cond:path, $($rest:tt)*) }) => {
        $crate::eval!(
            $truthy
            {
                $crate::eval!(
                    $truthy
                    $falsy
                    { any($($rest)*) }
                );
            }
            { $cond }
        );
    };
    ($truthy:tt $falsy:tt { $cond:path }) => {
        $cond! {
            if {
                $crate::eval!(
                    $truthy
                    $falsy
                    { true }
                );
            } else {
                $crate::eval!(
                    $truthy
                    $falsy
                    { false }
                );
            }
        }
    };
}

#[cfg(test)]
mod alias_tests {
    use super::alias;

    alias! {
        a: { #[cfg(all())] },
        b: { #[cfg(all())] },
        c: { a },
        d: { all(a, b, c) },
        e: { any(not(a), b, all(c), #[cfg(test)]) },
        pub f: { e },
    }
}

#[cfg(test)]
mod eval_tests {
    use super::{disabled, enabled, eval};

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { true }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { false }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { _ }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { all() }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { any() }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { #[cfg(test)] }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { not(#[cfg(test)]) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { any(#[cfg(test)]) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { all(#[cfg(test)]) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { enabled }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { not(enabled) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { any(enabled) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { all(enabled) }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { disabled }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { not(disabled) }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { any(disabled) }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { all(disabled) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { any(disabled, disabled, enabled) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { all(enabled, enabled, enabled) }
    );

    eval!(
        { compile_error!("expected falsy"); }
        { }
        { not(not(disabled)) }
    );

    eval!(
        { }
        { compile_error!("expected truthy"); }
        { all(any(any(not(disabled), enabled, disabled))) }
    );
}

#[cfg(test)]
mod forte_tests {
    use super::{alias, switch};

    mod cfg {
        use super::alias;

        alias! {
            pub parallel: { #[cfg(all())] }
        }
    }

    switch! {
        cfg::parallel => {
            mod blocker {}
            mod job {}
            mod scope {}
            mod signal {}
            mod thread_pool {
                pub const PARALLEL: bool = true;
            }

            pub use self::thread_pool::*;
            pub use self::scope::*;
        }
        _ => {
            mod fallback {
                pub const PARALLEL: bool = false;
            }

            pub use self::fallback::*;
        }
    }

    #[test]
    fn is_parallel() {
        assert!(PARALLEL);
    }
}

#[cfg(test)]
mod switch_as_value_tests {
    use super::switch;

    const PASSED: bool = switch! {
        _ => { true }
    };

    #[test]
    fn did_pass() {
        assert!(PASSED);
    }
}
