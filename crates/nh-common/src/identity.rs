//! Port of `identity/identity.go`.
//!
//! STUB: dependency tracking is not needed for a one-shot build
//!
//! Owner: Wave B task T01 (common-values).


//! STUB of neohugo `identity`: dependency tracking only matters for server/watch rebuilds.
//! Kept as unit types so ported signatures can mention them.

/// Go: `identity.Identity`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Identity(pub String);

/// Go: `identity.Manager` (no-op).
#[derive(Clone, Debug, Default)]
pub struct Manager;

/// Go: `identity.NopManager`.
pub const NOP_MANAGER: Manager = Manager;

/// Go: `identity.Incrementer` (`Incr() int`). The PostProcess placeholder ids
/// (`__h_pp_l1_<id>_`) come from ONE incrementer: the build's `deps.BuildState`, shared by all
/// sites (deps.go:255 passes `d.BuildState` to `resources.NewSpec`, which stores it in the shared
/// `SpecCommon`).
pub trait Incrementer: Send + Sync {
    fn incr(&self) -> i64;
}

/// Go: `identity.IncrementByOne` (the `NewSpec` fallback when no incrementer is given; tests).
#[derive(Debug, Default)]
pub struct IncrementByOne {
    counter: std::sync::atomic::AtomicU64,
}

impl Incrementer for IncrementByOne {
    // Go: identity/identity.go:Incr
    fn incr(&self) -> i64 {
        (self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1) as i64
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: identity/identity.go (521 lines; 2/39 funcs executed)
//   types: DependencyManagerProvider, DependencyManagerProviderFunc, DependencyManagerScopedProvider,
//          ForEeachIdentityProvider, ForEeachIdentityProviderFunc, ForEeachIdentityByNameProvider,
//          FindFirstManagerIdentityProvider, findFirstManagerIdentity, Identities, Identity, IdentityGroupProvider,
//          IdentityProvider, SignalRebuilder, IncrementByOne, Incrementer, IsProbablyDependentProvider,
//          IsProbablyDependencyProvider, Manager, ManagerOption, StringIdentity, identityManager, nopManager,
//          orIdentity
//    L44-56: NewManager(name string, opts ...ManagerOption) Manager
// EX L59-63: CleanString(s string) string
// EX L66-68: CleanStringIdentity(s string) StringIdentity
//    L71-81: GetDependencyManager(v any) Manager
//    L84-91: FirstIdentity(v any) Identity
//    L94-103: PrintIdentityInfo(v any)
//    L105-112: Unwrap(id Identity) Identity
//    L117-120: WalkIdentitiesDeep(v any, cb func(level int, id Identity) bool)
//    L125-127: WalkIdentitiesShallow(v any, cb func(level int, id Identity) bool)
//    L130-134: WithOnAddIdentity(f func(id Identity)) ManagerOption
//    L144-146: (d DependencyManagerProviderFunc) GetDependencyManager() Manager
//    L165-167: (f ForEeachIdentityProviderFunc) ForEeachIdentity(cb func(id Identity) bool) bool
//    L181-188: NewFindFirstManagerIdentityProvider(m Manager, id Identity) FindFirstManagerIdentityProvider
//    L195-197: (f findFirstManagerIdentity) FindFirstManagerIdentity() ManagerIdentity
//    L202-214: (ids Identities) AsSlice() []Identity
//    L216-227: (ids Identities) String() string
//    L257-259: (c *IncrementByOne) Incr() int
//    L292-294: (s StringIdentity) IdentifierBase() string
//    L312-328: (im *identityManager) AddIdentity(ids ...Identity)
//    L330-334: (im *identityManager) AddIdentityForEach(ids ...ForEeachIdentityProvider)
//    L336-345: (im *identityManager) ContainsIdentity(id Identity) FinderResult
//    L348-350: (im *identityManager) GetIdentity() Identity
//    L352-356: (im *identityManager) Reset()
//    L358-360: (im *identityManager) GetDependencyManagerForScope(int) Manager
//    L362-364: (im *identityManager) GetDependencyManagerForScopesAll() []Manager
//    L366-368: (im *identityManager) String() string
//    L370-384: (im *identityManager) forEeachIdentity(fn func(id Identity) bool) bool
//    L388-389: (m *nopManager) AddIdentity(ids ...Identity)
//    L391-392: (m *nopManager) AddIdentityForEach(ids ...ForEeachIdentityProvider)
//    L394-396: (m *nopManager) IdentifierBase() string
//    L398-400: (m *nopManager) GetIdentity() Identity
//    L402-403: (m *nopManager) Reset()
//    L405-407: (m *nopManager) forEeachIdentity(func(id Identity) bool) bool
//    L410-437: walkIdentities(v any, level int, deep bool, seen map[Identity]bool, cb func(level int, id Identity) bool)
//    L441-471: walkIdentitiesShallow(v any, level int, cb func(level int, id Identity) bool) bool
//    L478-480: Or(a, b Identity) Identity
//    L486-488: (o orIdentity) IdentifierBase() string
//    L490-497: (o orIdentity) ProbablyEq(other any) bool
//    L499-521: probablyEq(a, b Identity) bool
// ---------------------------------------------------------------------------
