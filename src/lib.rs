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
//! ```
//! // Without `crossfig`:
//!
//! #[cfg(feature = "fastest")]
//! # type A = ();
//! // Use the fastest algorithm!
//!
//! #[cfg(all(not(feature = "fastest"), feature = "fast"))]
//! # type B = ();
//! // Use the fast algorithm
//!
//! #[cfg(all(not(feature = "fastest"), not(feature = "fast")))]
//! # type C = ();
//! // Use the slow algorithm
//! ```
//!
//! ```
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
//! ```
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
//! ```
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
//! ```
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
/// # extern crate crossfig;
/// # use crossfig::{switch, alias};
/// # fn log<T>(_: T) {}
/// # fn foo<T>(_: T) {}
/// # alias! {
/// #    std: { all() }
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
    // Empty invocations should return nothing
    () => {};

    // Allow switch!{{ ... }} to act as an expression in certain contexts
    ({$($tt:tt)*}) => {
        { $crate::switch! { $($tt)* } }
    };

    // # Operation: not(...)
    (
        not($($args:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $($args)* => {
                $crate::switch! { $($arms)* }
            }
            _ => $output
        }
    };

    // # Operation: all(...)
    // ## Empty
    (
        all() => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! { _ => $output }
    };
    // ## Inner Op
    (
        all($op:ident($($cond:tt)*)) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $op($($cond)*) => $output
            $($arms)*
        }
    };
    // ## Inner Op & More
    (
        all($op:ident($($cond:tt)*), $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $op($($cond)*) => {
                $crate::switch! {
                    all($($rest)*) => $output
                    $($arms)*
                }
            }
            $($arms)*
        }
    };
    // ## Inner Meta
    (
        all(#[cfg($meta:meta)]) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            #[cfg($meta)] => $output
            $($arms)*
        }
    };
    // ## Inner Meta & More
    (
        all(#[cfg($meta:meta)], $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            #[cfg($meta)] => {
                $crate::switch! {
                    all($($rest)*) => $output
                    $($arms)*
                }
            }
            $($arms)*
        }
    };
    // ## Inner Alias
    (
        all($cond:path) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $cond => $output
            $($arms)*
        }
    };
    // ## Inner Alias & More
    (
        all($cond:path, $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $cond => {
                $crate::switch! {
                    all($($rest)*) => $output
                    $($arms)*
                }
            }
            $($arms)*
        }
    };

    // # Operation: any(...)
    // ## Empty
    (
        any() => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! { $($arms)* }
    };
    // ## Inner Op
    (
        any($op:ident($($cond:tt)*)) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $op($($cond)*) => $output
            $($arms)*
        }
    };
    // ## Inner Op & More
    (
        any($op:ident($($cond:tt)*), $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $op($($cond)*) => $output
            _ => {
                $crate::switch! {
                    any($($rest)*) => $output
                    $($arms)*
                }
            }
        }
    };
    // ## Inner Meta
    (
        any(#[cfg($meta:meta)]) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            #[cfg($meta)] => $output
            $($arms)*
        }
    };
    // ## Inner Meta & More
    (
        any(#[cfg($meta:meta)], $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            #[cfg($meta)] => $output
            any($($rest)*) => $output
            $($arms)*
        }
    };
    // ## Inner Alias
    (
        any($cond:path) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $cond => $output
            $($arms)*
        }
    };
    // ## Inner Alias & More
    (
        any($cond:path, $($rest:tt)*) => $output:tt
        $($arms:tt)*
    ) => {
        $crate::switch! {
            $cond => $output
            any($($rest)*) => $output
            $($arms)*
        }
    };

    // # Wildcard Branch
    (
        _ => { $($output:tt)* }
    ) => {
        $($output)*
    };
    // ## Common Mistake: arms after wildcard
    (
        _ => $output:tt
        $($arms:tt)+
    ) => {
        compile_error!(concat!("patterns after a wildcard are ignored: `", stringify!($($arms)+), "`"));
    };

    // # cfg(...) Integration
    (
        #[cfg($meta:meta)] => $output:tt
        $($arms:tt)*
    ) => {
        #[cfg($meta)]
        $crate::switch! { _ => $output }

        #[cfg(not($meta))]
        $crate::switch! { $($arms)* }
    };

    // # Alias Integration
    (
        $cond:path => $output:tt
        $($arms:tt)*
    ) => {
        $cond! {
            if { $crate::switch! { _ => $output } }
            else { $crate::switch! { $($arms)* } }
        }
    };
}

/// # Examples
///
/// ## As a `boolean`
///
/// ```
/// # extern crate crossfig;
/// assert_eq!(crossfig::disabled!(), false);
/// ```
///
/// ## As a Conditional Compilation Guard
///
/// ```
/// # extern crate crossfig;
/// crossfig::disabled! {
///     use a_crate_that_is_not_available::*;
///     // ...
/// }
/// ```
///
/// ## As an `if`-Statement
///
/// ```
/// # extern crate crossfig;
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
/// # extern crate crossfig;
/// assert_eq!(crossfig::enabled!(), true);
/// ```
///
/// ## As a Conditional Compilation Guard
///
/// ```
/// # extern crate crossfig;
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
/// # extern crate crossfig;
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
/// # extern crate crossfig;
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
///    # extern crate crossfig;
///    # use crossfig::alias;
///    # alias! {
///    #    std: { all() }
///    # }
///    if std!() {
///        // Have `std`!
///    } else {
///        // No `std`...
///    }
///    ```
/// 2. Pass a single code block which will only be compiled if the alias is active.
///    ```
///    # extern crate crossfig;
///    # use crossfig::alias;
///    # alias! {
///    #    std: { all() }
///    # }
///    std! {
///        // Have `std`!
///    # ()
///    }
///    ```
/// 3. Pass a single `if { ... } else { ... }` expression to conditionally compile either the first
///    or the second code block.
///    ```
///    # extern crate crossfig;
///    # use crossfig::alias;
///    # alias! {
///    #    std: { all() }
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
///    # extern crate crossfig;
///    # use crossfig::{alias, switch};
///    # alias! {
///    #    std: { all() }
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
    // Empty invocations should return nothing
    () => {};

    // Single arm with no trailing comma
    (
        $(#[$p_meta:meta])*
        $vis:vis $p:ident: { $($cond:tt)+ }
    ) => {
        $crate::alias! {
            $(#[$p_meta])*
            $vis $p: { $($cond)+ },
        }
    };

    // Some number of arms with trailing comma
    (
        $(#[$p_meta:meta])*
        $vis:vis $p:ident: { $($cond:tt)+ },
        $($rest:tt)*
    ) => {
        $crate::switch! {
            $($cond)+ => {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro passes the provided code because `", stringify!($($cond)+), "` is currently active.")]
                $vis use $crate::enabled as $p;
            }
            _ => {
                $(#[$p_meta])*
                #[doc(inline)]
                ///
                #[doc = concat!("This macro suppresses the provided code because `", stringify!($($cond)+), "` is _not_ currently active.")]
                $vis use $crate::disabled as $p;
            }
        }

        $crate::alias! {
            $($rest)*
        }
    };
}

#[cfg(test)]
mod alias_tests {
    #![allow(unused_imports)]

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
mod switch_tests {
    use super::{disabled, enabled, switch};

    #[test]
    fn tests() {
        let _a: ();
        switch! {
            all() => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            any() => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            #[cfg(all())] => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            not(#[cfg(all())]) => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            any(#[cfg(all())]) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            all(#[cfg(all())]) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            enabled => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            not(enabled) => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            any(enabled) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            all(enabled) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            disabled => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            not(disabled) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            any(disabled) => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            all(disabled) => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            any(disabled, disabled, enabled) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            all(enabled, enabled, enabled) => {
                _a = ();
            }
            _ => {
                _a = ();
                compile_error!("expected skip");
            }
        }

        let _a: ();
        switch! {
            not(not(disabled)) => {
                _a = ();
                compile_error!("expected skip");
            }
            _ => {
                _a = ();
            }
        }

        let _a: ();
        switch! {
            all(any(any(not(disabled), enabled, disabled))) => {
                _a = ();
            }
            _ => { compile_error!("expected skip"); }
        }

        let _a: ();
        switch! {
            any(not(enabled), disabled, all(enabled), #[cfg(test)]) => {
                _a = ();
            }
            _ => { compile_error!("expected skip"); }
        }
    }
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
            mod thread_pool {
                pub const PARALLEL: bool = true;
            }

            pub use self::thread_pool::*;
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
