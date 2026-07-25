//! Security strategy, mirroring Java `QLSecurityStrategy`.
//!
//! Java checks `java.lang.reflect.Member` objects; Rust native members are
//! registered explicitly (SPEC §4/§6), so members are identified by
//! `NativeMember` descriptors (`type_name.member_name`).

use std::collections::HashSet;

/// Identifies a native member (method/field) for security checks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NativeMember {
    /// Native type name (as registered in the native registry).
    pub type_name: String,
    /// Member (method or field) name.
    pub member_name: String,
}

impl NativeMember {
    pub fn new(type_name: impl Into<String>, member_name: impl Into<String>) -> Self {
        NativeMember {
            type_name: type_name.into(),
            member_name: member_name.into(),
        }
    }
}

/// Mirroring Java `QLSecurityStrategy`: decides whether a native member may
/// be accessed from a script.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum QLSecurityStrategy {
    /// Java `QLSecurityStrategy.open()`: allow everything.
    Open,
    /// Java `QLSecurityStrategy.isolation()`: allow nothing. (Java default.)
    #[default]
    Isolation,
    /// Java `QLSecurityStrategy.blackList(Set<Member>)`.
    BlackList(HashSet<NativeMember>),
    /// Java `QLSecurityStrategy.whiteList(Set<Member>)`.
    WhiteList(HashSet<NativeMember>),
}

impl QLSecurityStrategy {
    /// Java `QLSecurityStrategy.open()`.
    pub fn open() -> Self {
        QLSecurityStrategy::Open
    }

    /// Java `QLSecurityStrategy.isolation()`.
    pub fn isolation() -> Self {
        QLSecurityStrategy::Isolation
    }

    /// Java `QLSecurityStrategy.blackList(...)`.
    pub fn black_list(black_list: HashSet<NativeMember>) -> Self {
        QLSecurityStrategy::BlackList(black_list)
    }

    /// Java `QLSecurityStrategy.whiteList(...)`.
    pub fn white_list(white_list: HashSet<NativeMember>) -> Self {
        QLSecurityStrategy::WhiteList(white_list)
    }

    /// Java `check(Member)`: true when the member is secure to access.
    pub fn check(&self, member: &NativeMember) -> bool {
        match self {
            QLSecurityStrategy::Open => true,
            QLSecurityStrategy::Isolation => false,
            QLSecurityStrategy::BlackList(black_list) => !black_list.contains(member),
            QLSecurityStrategy::WhiteList(white_list) => white_list.contains(member),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_semantics() {
        let member = NativeMember::new("java.lang.Runtime", "exec");
        assert!(QLSecurityStrategy::open().check(&member));
        assert!(!QLSecurityStrategy::isolation().check(&member));

        let black: HashSet<_> = [member.clone()].into_iter().collect();
        assert!(!QLSecurityStrategy::black_list(black).check(&member));
        assert!(!QLSecurityStrategy::white_list(HashSet::new()).check(&member));

        let white: HashSet<_> = [member.clone()].into_iter().collect();
        assert!(QLSecurityStrategy::white_list(white).check(&member));
    }
}
