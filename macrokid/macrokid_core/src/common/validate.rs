//! Generic validation facade for cross-domain “validate with traits” flows.
//!
//! Pattern
//! - Many domains (graphics, DSLs) need to validate a runtime/config object against
//!   one or more trait-provided views (e.g., resource bindings, vertex layout).
//! - This module provides a tiny trait that domain crates can implement to standardize
//!   such flows without hard-coding types or dependencies in macrokid_core.
//!
//! Usage
//! ```ignore
//! use macrokid_core::common::validate::{ValidateExt, Validator};
//!
//! // Domain config
//! struct MyConfig { name: &'static str }
//!
//! // Domain-specific validator (could be generic over trait bounds)
//! struct NameNotEmpty;
//! impl Validator<MyConfig> for NameNotEmpty {
//!     type Error = String;
//!     fn validate(cfg: &MyConfig) -> Result<(), Self::Error> {
//!         if cfg.name.is_empty() { Err("name must not be empty".into()) } else { Ok(()) }
//!     }
//! }
//!
//! let cfg = MyConfig { name: "demo" };
//! // Call-site pattern: make cfg the receiver and choose the validator type.
//! cfg.validate_with::<NameNotEmpty>().expect("ok");
//! ```
//!
//! Composing multiple validators
//! - Define a tiny composite type that calls multiple `Validator<C>` impls in sequence.
//! - This keeps composition flexible without introducing complex macros here.
//! ```ignore
//! struct All<A, B>(core::marker::PhantomData<(A, B)>);
//! impl<C, A, B> Validator<C> for All<A, B>
//! where A: Validator<C, Error = String>, B: Validator<C, Error = String>
//! {
//!     type Error = String;
//!     fn validate(cfg: &C) -> Result<(), Self::Error> {
//!         A::validate(cfg)?;
//!         B::validate(cfg)?;
//!         Ok(())
//!     }
//! }
//! ```

/// Combinator that requires both A and B validators to pass.
/// 
/// This is a zero-sized type that combines two validators for the same config type.
/// Both validators must succeed, and they must use the same error type.
/// 
/// # Example
/// ```ignore
/// cfg.validate_with::<And<ValidatorA, ValidatorB>>()?;
/// ```
pub struct And<A, B>(core::marker::PhantomData<(A, B)>);

impl<Cfg, A, B, E> Validator<Cfg> for And<A, B>
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
{
    type Error = E;
    
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        // Run A first, then B - short-circuits on first error
        A::validate(cfg)?;
        B::validate(cfg)?;
        Ok(())
    }
}

/// Alternative tuple-based combinator for even more terse usage.
/// Allows: `cfg.validate_with::<(ValidatorA, ValidatorB)>()?;`
impl<Cfg, A, B, E> Validator<Cfg> for (A, B)
where
    A: Validator<Cfg, Error = E>,
    B: Validator<Cfg, Error = E>,
{
    type Error = E;
    
    fn validate(cfg: &Cfg) -> Result<(), Self::Error> {
        A::validate(cfg)?;
        B::validate(cfg)?;
        Ok(())
    }
}

/// Generic validator that checks `Cfg` and returns a domain-defined error.
pub trait Validator<Cfg> {
    type Error;
    fn validate(cfg: &Cfg) -> Result<(), Self::Error>;
}

/// Extension for ergonomic `cfg.validate_with::<V>()` calls.
pub trait ValidateExt {
    fn validate_with<V>(
        &self,
    ) -> Result<(), <V as Validator<Self>>::Error>
    where
        Self: Sized,
        V: Validator<Self>;
}

impl<T> ValidateExt for T {
    fn validate_with<V>(&self) -> Result<(), <V as Validator<Self>>::Error>
    where
        Self: Sized,
        V: Validator<Self>,
    {
        V::validate(self)
    }
}

/// Logical OR combinator: succeed if either `A` or `B` passes.
///
/// Tries `A` first; if it fails, tries `B`. Both must share the same error type.
pub struct Or<A, B>(core::marker::PhantomData<(A, B)>);

impl<C, A, B, E> Validator<C> for Or<A, B>
where
    A: Validator<C, Error = E>,
    B: Validator<C, Error = E>,
{
    type Error = E;
    fn validate(cfg: &C) -> Result<(), Self::Error> {
        match A::validate(cfg) {
            Ok(()) => Ok(()),
            Err(_e) => B::validate(cfg),
        }
    }
}

/// Optional combinator: never fails; logs when inner validator fails (behind `logging` feature).
pub struct Optional<A>(core::marker::PhantomData<A>);

impl<C, A, E> Validator<C> for Optional<A>
where
    A: Validator<C, Error = E>,
    E: core::fmt::Debug,
{
    type Error = ();
    fn validate(cfg: &C) -> Result<(), Self::Error> {
        match A::validate(cfg) {
            Ok(()) => {
                #[cfg(feature = "log")]
                log::debug!("optional validation passed");
                Ok(())
            }
            Err(_e) => {
                #[cfg(feature = "log")]
                log::warn!("optional validation failed");
                Ok(())
            }
        }
    }
}

// (additional tests appear later in this file)

#[cfg(test)]
mod tests {
    use super::*;

    // Test config struct
    #[derive(Debug)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    // Validator that checks name is not empty
    struct NameNotEmpty;
    impl Validator<TestConfig> for NameNotEmpty {
        type Error = String;
        fn validate(cfg: &TestConfig) -> Result<(), Self::Error> {
            if cfg.name.is_empty() {
                Err("name cannot be empty".to_string())
            } else {
                Ok(())
            }
        }
    }

    // Validator that checks value is positive
    struct ValuePositive;
    impl Validator<TestConfig> for ValuePositive {
        type Error = String;
        fn validate(cfg: &TestConfig) -> Result<(), Self::Error> {
            if cfg.value <= 0 {
                Err("value must be positive".to_string())
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_single_validator_success() {
        let cfg = TestConfig { name: "test".to_string(), value: 42 };
        assert!(cfg.validate_with::<NameNotEmpty>().is_ok());
    }

    #[test]
    fn test_single_validator_failure() {
        let cfg = TestConfig { name: "".to_string(), value: 42 };
        let result = cfg.validate_with::<NameNotEmpty>();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name cannot be empty");
    }

    #[test]
    fn test_and_combinator_both_pass() {
        let cfg = TestConfig { name: "test".to_string(), value: 42 };
        let result = cfg.validate_with::<And<NameNotEmpty, ValuePositive>>();
        assert!(result.is_ok());
    }

    #[test]
    fn test_and_combinator_first_fails() {
        let cfg = TestConfig { name: "".to_string(), value: 42 };
        let result = cfg.validate_with::<And<NameNotEmpty, ValuePositive>>();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name cannot be empty");
    }

    #[test]
    fn test_and_combinator_second_fails() {
        let cfg = TestConfig { name: "test".to_string(), value: -5 };
        let result = cfg.validate_with::<And<NameNotEmpty, ValuePositive>>();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "value must be positive");
    }

    #[test]
    fn test_and_combinator_both_fail_returns_first() {
        let cfg = TestConfig { name: "".to_string(), value: -5 };
        let result = cfg.validate_with::<And<NameNotEmpty, ValuePositive>>();
        assert!(result.is_err());
        // Should return the first error (short-circuit)
        assert_eq!(result.unwrap_err(), "name cannot be empty");
    }

    #[test]
    fn test_tuple_combinator_success() {
        let cfg = TestConfig { name: "test".to_string(), value: 42 };
        let result = cfg.validate_with::<(NameNotEmpty, ValuePositive)>();
        assert!(result.is_ok());
    }

    #[test]
    fn test_tuple_combinator_failure() {
        let cfg = TestConfig { name: "".to_string(), value: 42 };
        let result = cfg.validate_with::<(NameNotEmpty, ValuePositive)>();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "name cannot be empty");
    }

    // OR combinator: Name must be non-empty OR value must be positive.
    #[test]
    fn test_or_combinator() {
        let ok1 = TestConfig { name: "x".to_string(), value: -1 };
        let ok2 = TestConfig { name: "".to_string(), value: 1 };
        let bad = TestConfig { name: "".to_string(), value: 0 };
        assert!(ok1.validate_with::<Or<NameNotEmpty, ValuePositive>>().is_ok());
        assert!(ok2.validate_with::<Or<NameNotEmpty, ValuePositive>>().is_ok());
        assert!(bad.validate_with::<Or<NameNotEmpty, ValuePositive>>().is_err());
    }

    // Optional combinator never fails
    #[test]
    fn test_optional_combinator_never_fails() {
        let bad = TestConfig { name: "".to_string(), value: 0 };
        let res: Result<(), ()> = bad.validate_with::<Optional<NameNotEmpty>>();
        assert!(res.is_ok());
    }
}
