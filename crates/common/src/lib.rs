// =============================================================================
// PHASE 0: RUST BOOTCAMP
// =============================================================================
// This file serves as both a working library AND a tutorial.
// Each section explains a Rust concept with C++/Go/JS analogies.
//
// HOW TO USE THIS FILE:
//   1. Read the comments top-to-bottom
//   2. Run `cargo test -- -nocapture` to see the examples execute
//   3. Experiment by modifying the test cases
// =============================================================================

// ─── MODULE DECLARATIONS ────────────────────────────────────────────────────
// KEY LESSON: Rust modules
// =========================
// `mod foo;` tells Rust: "look for foo.rs or foo/mod.rs in the same directory"
// Unlike C++ `#include` (textual inclusion) or Go's package-by-directory,
// Rust modules are explicit: you MUST declare every module in the parent.
//
// Public re-exports with `pub use` create a clean public API surface.
// This is like C++'s public headers or Go's exported identifiers (capitalized).
// The key difference: in Rust, `pub` is opt-in. Everything is private by default.
// In Go, capitalization controls visibility. In C++, `public:` is per-class.
// In Rust, `pub` is per-item (struct, fn, module, field, etc.).

pub mod error;

// ─── SECTION 0.2: OWNERSHIP, BORROWING, REFERENCES ──────────────────────────
// KEY LESSON: Ownership
// ======================
// Rust's ownership system is the core innovation. It eliminates:
//   - Use-after-free (C/C++: dangling pointer)
//   - Double-free (C/C++: same memory freed twice)
//   - Data races (C/C++/Go: concurrent read/write without synchronization)
//   - Null pointer dereference (C/C++: segmentation fault)
//
// The THREE RULES of ownership:
//   1. Each value in Rust has exactly ONE owner at any given time.
//   2. When the owner goes out of scope, the value is dropped (freed).
//   3. Ownership can be transferred (moved) or temporarily shared (borrowed).
//
// COMPARISON:
//   C++:  std::unique_ptr<T> ≈ owned value (move semantics)
//         std::shared_ptr<T> ≈ Arc<T> (reference counted)
//         const T&           ≈ &T (immutable borrow)
//         T&                 ≈ &mut T (mutable borrow)
//   Go:   All values are owned, but GC handles cleanup.
//         Go doesn't have move semantics — everything is copied or referenced.
//   JS:   GC handles everything. No concept of ownership at all.

/// Demonstrates ownership transfer (move semantics).
///
/// In Rust, assigning a `String` to another variable MOVES it.
/// The original variable can no longer be used.
/// This is like C++ move semantics (`std::move`), but it's the DEFAULT in Rust.
///
/// ```rust
/// let s1 = String::from("hello");
/// let s2 = s1;              // s1 is MOVED to s2
/// // println!("{s1}");     // COMPILE ERROR: s1 was moved!
/// println!("{s2}");         // OK: s2 now owns the string
/// ```
///
/// For types that implement `Copy` (integers, bools, etc.), assignment copies
/// instead of moving. `Copy` means "bitwise copy is safe" — no heap data involved.
///
/// ```rust
/// let x = 42;
/// let y = x;  // x is COPIED (not moved) because i32 implements Copy
/// println!("{x}");  // OK: x is still valid
/// ```
pub fn ownership_demo() -> String {
    let s1 = String::from("hello");
    let s2 = s1; // s1 is MOVED to s2
                 // s1 is no longer valid here
    s2 // return ownership to caller
}

/// Demonstrates borrowing (references).
///
/// Instead of moving a value, you can BORROW it:
///   - `&T`    = immutable reference (shared borrow). You can have MANY.
///   - `&mut T` = mutable reference (exclusive borrow). You can have ONLY ONE.
///
/// This is Rust's "Aliasing XOR Mutability" rule:
///   At any given time, you can have EITHER:
///     - One mutable reference (&mut T), OR
///     - Any number of immutable references (&T)
///   But NEVER both simultaneously.
///
/// This rule prevents data races at COMPILE TIME.
/// C++: const references can coexist with mutable references → data races possible.
/// Go: goroutines share memory freely → need mutexes for safety.
/// Rust: the compiler guarantees no data races without any runtime cost.
///
/// ```rust
/// let mut s = String::from("hello");
/// let r1 = &s;      // immutable borrow
/// let r2 = &s;      // another immutable borrow — OK
/// // let r3 = &mut s; // COMPILE ERROR: can't borrow mutably while immutably borrowed
/// println!("{r1} {r2}");
/// // r1 and r2 are no longer used after this point (non-lexical lifetimes)
/// let r3 = &mut s;  // NOW this is OK: immutable borrows have ended
/// r3.push_str(" world");
/// ```
pub fn borrowing_demo() -> String {
    let mut s = String::from("hello");
    {
        let r1 = &s;
        let r2 = &s;
        println!("Immutable references: {r1}, {r2}");
    } // r1 and r2 go out of scope here — immutable borrow ends

    let r3 = &mut s; // now mutable borrow is OK
    r3.push_str(" world");
    s // return the modified string
}

// ─── SECTION 0.3: STRUCTS, ENUMS, PATTERN MATCHING ─────────────────────────

/// KEY LESSON: Structs
/// ====================
/// Rust structs are like C structs with methods, or Go structs.
/// Unlike C++, Rust has no class/inheritance. Instead:
///   - Data: struct (or enum)
///   - Behavior: impl blocks (with traits for polymorphism)
///
/// Field visibility: `pub` makes a field accessible outside the module.
/// Private fields (no `pub`) are only accessible within the same module.
/// This is more like C++ `private` by default, but at MODULE level, not class.
///
/// Derive macros: `#[derive(Debug)]` auto-generates the `Debug` trait
/// (like C++ `operator<<` for ostream, or Go's `fmt.Stringer`).
/// This is compile-time code generation — no reflection, no runtime cost.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Product {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub price: rust_decimal::Decimal, // NEVER use f64 for money! (floating point rounding)
    pub stock: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// KEY LESSON: impl blocks
/// =========================
/// `impl Product { ... }` adds methods to the Product struct.
/// Unlike C++ where methods are defined inside the class body,
/// Rust separates data (struct) from behavior (impl).
/// This is intentional: you can have MULTIPLE `impl Product` blocks,
/// even in different files, as long as they're in the same crate.
/// This is great for organizing code by feature/trait.
impl Product {
    /// Creates a new Product with defaults.
    /// `new()` is a CONVENTION, not a language feature.
    /// There's no constructor in Rust — just a static method.
    /// Compare: C++ constructor, Go `NewProduct()` function.
    pub fn new(
        name: String,
        description: String,
        price: rust_decimal::Decimal,
        stock: i32,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            description,
            price,
            stock,
            created_at: now,
            updated_at: now,
        }
    }

    /// Validates the product fields.
    /// Returns `Result<(), String>` — Ok(()) if valid, Err(msg) if not.
    ///
    /// KEY LESSON: Result<T, E>
    /// =========================
    /// `Result<T, E>` is Rust's way of handling fallible operations.
    /// It's an enum with two variants:
    ///   - `Ok(T)`  — success, contains the value
    ///   - `Err(E)` — failure, contains the error
    ///
    /// COMPARISON:
    ///   Go:   `(T, error)` tuple — but you can FORGET to check the error.
    ///   C++:  exceptions — invisible control flow, can be thrown from anywhere.
    ///   Rust: Result<T,E> — the TYPE SYSTEM forces you to handle both cases.
    ///         The `?` operator propagates errors: `let x = fallible()?;`
    ///         If `fallible()` returns Err, the function returns immediately
    ///         with that error. This is like Go's `if err != nil { return err }`
    ///         but it's a single character: `?`.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Product name cannot be empty".to_string());
        }
        if self.price <= rust_decimal::Decimal::ZERO {
            return Err("Price must be greater than zero".to_string());
        }
        if self.stock < 0 {
            return Err("Stock cannot be negative".to_string());
        }
        Ok(())
    }
}

/// KEY LESSON: Enums (Sum Types / Tagged Unions)
/// ==============================================
/// Rust enums are NOT like C/C++ enums (which are just named integers).
/// Rust enums are "algebraic data types" — each variant can carry data.
///
/// This is the single most powerful feature coming from C/Go:
///   - C:      enum + union + tag field (manual, error-prone)
///   - C++:    std::variant<Types...> (clunky, requires std::visit)
///   - Go:     interface{} or const iota enums (no data attached!)
///   - Rust:   first-class, compiler-checked, pattern-matched
///
/// Rust enums eliminate entire classes of bugs:
///   - No invalid states: the type system encodes valid states
///   - No forgotten cases: `match` enforces exhaustiveness
///   - No null: use Option<T> instead of null/nil/None
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ProductStatus {
    Active,
    Draft,
    Discontinued,
    OutOfStock,
}

impl std::fmt::Display for ProductStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProductStatus::Active => write!(f, "active"),
            ProductStatus::Draft => write!(f, "draft"),
            ProductStatus::Discontinued => write!(f, "discontinued"),
            ProductStatus::OutOfStock => write!(f, "out_of_stock"),
        }
    }
}

/// KEY LESSON: Option<T> — Rust's replacement for null
/// =====================================================
/// Option<T> is an enum with two variants:
///   - `Some(T)` — a value is present
///   - `None`     — no value
///
/// Why no null? Tony Hoare called null references his "billion-dollar mistake."
/// In C/C++: dereferencing NULL causes undefined behavior (segfault).
/// In Go: nil pointer dereference causes panic.
/// In JS: `undefined` and `null` cause TypeErrors at runtime.
/// In Rust: you MUST handle the None case. The compiler won't let you ignore it.
///
/// How to work with Option:
///   - `match` for exhaustive handling
///   - `.unwrap()` — panic if None (use sparingly, like assertions)
///   - `.unwrap_or(default)` — provide fallback
///   - `.map(|v| ...)` — transform if Some
///   - `?` — return None if None (in functions returning Option)
pub fn find_product_by_id(products: &[Product], id: uuid::Uuid) -> Option<&Product> {
    products.iter().find(|p| p.id == id)
}

// ─── SECTION 0.4: TRAITS ────────────────────────────────────────────────────

/// KEY LESSON: Traits — Defining Shared Behavior
/// ==============================================
/// Traits define a set of methods that types can implement.
/// This is the PRIMARY mechanism for polymorphism in Rust.
///
/// COMPARISON:
///   Go:   interfaces — duck typing, implicit implementation
///   C++:  abstract base classes with virtual methods — vtable dispatch
///   JS:   no formal interface — duck typing at runtime
///
/// Rust traits are like Go interfaces BUT:
///   - Implementation is explicit (`impl Trait for Type { ... }`)
///   - Can be implemented for types you don't own (orphan rule permitting)
///   - Support associated types, constants, and default method implementations
///   - Monomorphized for concrete types (zero-cost), but can also use dynamic dispatch
///
/// TWO KINDS of trait dispatch:
///   1. Static dispatch (monomorphization): `fn foo<T: MyTrait>(t: T)`
///      - Compiler generates a separate copy of the function for each concrete type
///      - Like C++ templates — zero runtime overhead
///      - This is the DEFAULT in Rust
///   2. Dynamic dispatch: `fn foo(t: &dyn MyTrait)`
///      - Uses a vtable — like C++ virtual methods or Go interfaces
///      - Slight runtime overhead, but enables heterogeneous collections
///      - Use `&dyn Trait` or `Box<dyn Trait>`
pub trait Repository<T> {
    /// Find an entity by its ID.
    /// Returns `Option<T>` — None if not found, Some(T) if found.
    /// This will be async later (using `async_trait` macro).
    fn find_by_id(&self, id: uuid::Uuid) -> Option<&T>;

    /// Find all entities.
    fn find_all(&self) -> Vec<&T>;

    /// Save an entity (create or update).
    fn save(&mut self, entity: T) -> Result<(), String>;

    /// Delete an entity by ID.
    fn delete(&mut self, id: uuid::Uuid) -> Result<(), String>;
}

// ─── SECTION 0.5: GENERICS & TRAIT BOUNDS ───────────────────────────────────

/// KEY LESSON: Generics with Trait Bounds
/// =======================================
/// Like C++ templates, but with constraints (trait bounds).
///
/// ```ignore
/// // Rust generics example (not run as doc-test because it's pseudo-code):
/// fn sort<T: Ord>(list: &mut [T]) {
///     list.sort(); // T must implement Ord (total ordering)
/// }
/// // C++ equivalent: template<typename T> requires Sortable<T> void sort(vector<T>& list)
/// // But C++ templates don't constrain T — you get cryptic errors if T lacks < operator.
/// // Rust's trait bounds give clear, early error messages.
/// ```
///
/// Monomorphization: Like C++ templates, Rust generates separate code for each
/// concrete type used. `sort::<i32>()` and `sort::<String>()` are different
/// functions. This is zero-cost: no runtime dispatch, and the compiler can
/// inline and optimize for each type.
///
/// Compared to Go generics (Go 1.18+): Go also uses monomorphization in some
/// cases, but the type system is less expressive (no trait bounds, just interfaces).
pub struct InMemoryRepository<T> {
    items: Vec<T>,
}

// KEY LESSON: The `impl<T> InMemoryRepository<T>` syntax
// =======================================================
// `impl<T>` declares a generic type parameter T for the impl block.
// This is like C++ `template<typename T>` before a class definition.
// Everything inside this impl block can use T.
impl<T> InMemoryRepository<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
}

// KEY LESSON: Conditional trait implementation
// =============================================
// `where T: Clone` means "this impl block only applies when T implements Clone."
// This is called a "trait bound" or "bound."
// The compiler will NOT let you use `InMemoryRepository<NonCloneType>` as a Repository.
// This is like C++ concepts (C++20) or SFINAE, but more readable.
impl<T> Repository<T> for InMemoryRepository<T>
where
    T: Clone,
{
    fn find_by_id(&self, id: uuid::Uuid) -> Option<&T> {
        // This is a placeholder — real implementation would compare IDs
        // For now, just return the first item if any
        let _ = id;
        self.items.first()
    }

    fn find_all(&self) -> Vec<&T> {
        self.items.iter().collect()
    }

    fn save(&mut self, entity: T) -> Result<(), String> {
        self.items.push(entity);
        Ok(())
    }

    fn delete(&mut self, _id: uuid::Uuid) -> Result<(), String> {
        // Placeholder
        Ok(())
    }
}

// ─── SECTION 0.7: ITERATORS & CLOSURES ──────────────────────────────────────

/// KEY LESSON: Iterators
/// ======================
/// Rust iterators are lazy, zero-cost abstractions.
/// `.iter()` returns an iterator that borrows each element (&T).
/// `.into_iter()` consumes the collection, returning owned values (T).
/// `.iter_mut()` returns mutable references (&mut T).
///
/// Iterator combinators (like JS array methods):
///   - `.map(|x| ...)`    — transform each element
///   - `.filter(|x| ...)` — keep elements matching predicate
///   - `.find(|x| ...)`   — return first match
///   - `.collect()`       — gather into a collection (Vec, HashMap, etc.)
///   - `.fold(init, |acc, x| ...)` — reduce (like JS reduce, Go accumulate)
///
/// KEY INSIGHT: Iterator chains compile to the SAME assembly as hand-written loops.
/// This is "zero-cost abstraction" — the high-level code is just as fast as low-level.
/// C++ ranges (C++20) offer similar guarantees. Go doesn't have functional iterators
/// in the standard library (you write for loops).
pub fn product_search_summary(products: &[Product], query: &str) -> Vec<String> {
    products
        .iter() // borrow each product
        .filter(|p| p.name.to_lowercase().contains(&query.to_lowercase()))
        .map(|p| format!("{} - ${}", p.name, p.price))
        .collect() // gather into Vec<String>
}

// ─── SECTION 0.8: MODULES, VISIBILITY, USE ──────────────────────────────────

/// KEY LESSON: Module system recap
/// =================================
/// ```
/// crate (root)
/// ├── lib.rs          (this file — the crate root for the `common` library)
/// ├── error.rs        (declared as `pub mod error;`)
/// └── (future modules)
/// ```
///
/// Visibility rules:
///   - `pub`     — visible to everything (including external crates)
///   - `pub(crate)` — visible within this crate only
///   - `pub(super)` — visible to the parent module
///   - (no pub)  — private to the current module (default)
///
/// This is more granular than Go (package-level only) or C++ (class-level only).
/// Rust's privacy is MODULE-level, which encourages organizing code into small,
/// cohesive modules without exposing internals.

// =============================================================================
// TESTS (Run with: `cargo test -p common -- --nocapture`)
// =============================================================================
// KEY LESSON: Built-in testing
// =============================
// `cargo test` runs all `#[test]` functions.
// Tests can be in the same file (unit tests) or in tests/ directory (integration tests).
// This is like Go's `go test`, but tests live alongside code (or in tests/).
// C++: separate test frameworks (GTest, Catch2). Rust: built-in.
//
// `#[cfg(test)]` means "only compile this module when testing."
// This prevents test helpers from bloating the release binary.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_move() {
        let result = ownership_demo();
        assert_eq!(result, "hello");
        // `result` is now owned by this function.
        // When this function ends, `result` is dropped (freed).
        // No GC needed. No manual free needed. The compiler knows exactly when to free.
    }

    #[test]
    fn test_borrowing() {
        let result = borrowing_demo();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_product_creation() {
        let price = rust_decimal::Decimal::new(2999, 2); // 29.99
        let product = Product::new("Test Product".into(), "A test product".into(), price, 100);
        assert!(product.validate().is_ok());
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.stock, 100);
    }

    #[test]
    fn test_product_validation_fails_on_empty_name() {
        let product = Product::new(
            "".into(),
            "desc".into(),
            rust_decimal::Decimal::new(100, 0),
            10,
        );
        assert!(product.validate().is_err());
    }

    #[test]
    fn test_product_validation_fails_on_zero_price() {
        let product = Product::new(
            "Name".into(),
            "desc".into(),
            rust_decimal::Decimal::ZERO,
            10,
        );
        assert!(product.validate().is_err());
    }

    #[test]
    fn test_product_validation_fails_on_negative_stock() {
        let product = Product::new(
            "Name".into(),
            "desc".into(),
            rust_decimal::Decimal::new(100, 0),
            -1,
        );
        assert!(product.validate().is_err());
    }

    #[test]
    fn test_option_some() {
        let products = vec![Product::new(
            "A".into(),
            "desc".into(),
            rust_decimal::Decimal::new(100, 0),
            10,
        )];
        let id = products[0].id;
        let found = find_product_by_id(&products, id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "A");
    }

    #[test]
    fn test_option_none() {
        let products = vec![Product::new(
            "A".into(),
            "desc".into(),
            rust_decimal::Decimal::new(100, 0),
            10,
        )];
        let found = find_product_by_id(&products, uuid::Uuid::new_v4());
        assert!(found.is_none());
    }

    #[test]
    fn test_iterator_search() {
        let products = vec![
            Product::new(
                "Laptop".into(),
                "desc".into(),
                rust_decimal::Decimal::new(99900, 2),
                10,
            ),
            Product::new(
                "Mouse".into(),
                "desc".into(),
                rust_decimal::Decimal::new(2500, 2),
                50,
            ),
            Product::new(
                "Keyboard".into(),
                "desc".into(),
                rust_decimal::Decimal::new(7500, 2),
                30,
            ),
        ];
        let results = product_search_summary(&products, "mo");
        assert_eq!(results.len(), 1);
        assert!(results[0].contains("Mouse"));
    }

    #[test]
    fn test_in_memory_repository() {
        let mut repo = InMemoryRepository::new();
        let product = Product::new(
            "Test".into(),
            "desc".into(),
            rust_decimal::Decimal::new(100, 0),
            10,
        );
        assert!(repo.save(product).is_ok());
        assert_eq!(repo.find_all().len(), 1);
    }

    #[test]
    fn test_enum_display() {
        assert_eq!(ProductStatus::Active.to_string(), "active");
        assert_eq!(ProductStatus::OutOfStock.to_string(), "out_of_stock");
    }
}
