use std::sync::OnceLock;

use bun_core::String as BunString;

use crate::whatever_policy::ModulePolicy;

// Process-wide and irreversible: a second realm must not widen the mod's policy.
static POLICY: OnceLock<ModulePolicy> = OnceLock::new();

pub fn is_locked() -> bool {
    POLICY.get().is_some()
}

pub fn allows_import(name: &[u8]) -> bool {
    let Some(policy) = POLICY.get() else {
        return true;
    };
    std::str::from_utf8(name).is_ok_and(|name| policy.allows_specifier(name))
}

#[unsafe(no_mangle)]
extern "C" fn Bun__whateverIsLocked() -> bool {
    is_locked()
}

#[unsafe(no_mangle)]
extern "C" fn Bun__whateverLock(root: &BunString) -> bool {
    let bytes = root.to_utf8();
    let Ok(root) = std::str::from_utf8(bytes.slice()) else {
        return false;
    };
    let Ok(policy) = ModulePolicy::new(std::path::Path::new(root)) else {
        return false;
    };
    POLICY.set(policy).is_ok()
}

#[unsafe(no_mangle)]
extern "C" fn Bun__whateverAllowsSpecifier(name: &BunString) -> bool {
    let Some(policy) = POLICY.get() else {
        return true;
    };
    let bytes = name.to_utf8();
    std::str::from_utf8(bytes.slice()).is_ok_and(|name| policy.allows_specifier(name))
}

#[unsafe(no_mangle)]
extern "C" fn Bun__whateverAllowsResolved(name: &BunString) -> bool {
    let Some(policy) = POLICY.get() else {
        return true;
    };
    let bytes = name.to_utf8();
    std::str::from_utf8(bytes.slice()).is_ok_and(|name| policy.allows_resolved(name))
}
