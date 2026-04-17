//! Trait + generic resolver for alias-aware lookup of canonical keys.
//!
//! The config has two entry types that accept lookup by either a
//! canonical `BTreeMap` key or by a user-declared short-name alias:
//! `RepoEntry` (jumped to by `rep jump alias` etc.) and
//! `WorkspaceEntry` (targeted by `rep workspace open alias`). Without
//! a shared resolver, each registry would reimplement the same
//! "try-key-then-walk-aliases" loop and the two copies would
//! eventually drift on edge cases. This module is the single chokepoint
//! so the resolution semantics stay identical for every aliased entry
//! today and for any future one.

use std::collections::BTreeMap;

/// Any config entry type that can be looked up by a canonical key
/// OR by one of its declared short-name aliases.
///
/// Implementing this trait opts an entry into the
/// `resolve_canonical_key` pipeline. The trait exists purely so the
/// resolver can be generic over entry type — it does not define
/// construction, validation, or mutation, all of which remain on the
/// concrete type.
pub trait HasAliases {
    /// Declared alternative names for this entry, in the order the
    /// user wrote them.
    ///
    /// Order is preserved because callers may rely on first-match
    /// semantics for display and because a stable declared order
    /// keeps TOML round-trips byte-identical. Returning `&[String]`
    /// rather than an iterator lets the resolver walk the list
    /// without forcing each implementor to return a boxed iterator.
    fn aliases(&self) -> &[String];
}

/// Resolves a user-supplied lookup key to the canonical key in the
/// given entry map, walking both the map's canonical keys and each
/// entry's alias list.
///
/// Primary-key match wins first, so the common case (user typed the
/// canonical name) costs one `BTreeMap::get_key_value` and is never
/// shadowed by a colliding alias declared elsewhere in the map. Falls
/// back to a linear scan of entries' alias lists and returns the
/// first canonical key whose entry declares the queried alias.
///
/// Returns `None` when nothing matches so callers can map absence to
/// their domain-specific not-found error variant (`RepoNotFound`,
/// `WorkspaceNotFound`, ...) without this helper needing to know
/// about the error enum.
pub fn resolve_canonical_key<'map, T>(
    entry_map: &'map BTreeMap<String, T>,
    query: &str,
) -> Option<&'map str>
where
    T: HasAliases,
{
    if let Some((canonical_key, _)) = entry_map.get_key_value(query) {
        return Some(canonical_key.as_str());
    }
    entry_map
        .iter()
        .find(|(_, entry)| entry.aliases().iter().any(|declared| declared == query))
        .map(|(canonical_key, _)| canonical_key.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEntry {
        declared_aliases: Vec<String>,
    }

    impl HasAliases for FakeEntry {
        fn aliases(&self) -> &[String] {
            &self.declared_aliases
        }
    }

    fn build_entry_map() -> BTreeMap<String, FakeEntry> {
        let mut entry_map = BTreeMap::new();
        entry_map.insert(
            "venoble".to_owned(),
            FakeEntry {
                declared_aliases: vec!["vn".to_owned(), "noble".to_owned()],
            },
        );
        entry_map.insert(
            "reportal".to_owned(),
            FakeEntry {
                declared_aliases: vec!["rep".to_owned()],
            },
        );
        entry_map
    }

    #[test]
    fn canonical_key_match_wins_first() {
        let entry_map = build_entry_map();
        assert_eq!(resolve_canonical_key(&entry_map, "venoble"), Some("venoble"));
    }

    #[test]
    fn alias_falls_back_to_canonical_key_lookup() {
        let entry_map = build_entry_map();
        assert_eq!(resolve_canonical_key(&entry_map, "vn"), Some("venoble"));
        assert_eq!(resolve_canonical_key(&entry_map, "noble"), Some("venoble"));
        assert_eq!(resolve_canonical_key(&entry_map, "rep"), Some("reportal"));
    }

    #[test]
    fn unknown_query_returns_none() {
        let entry_map = build_entry_map();
        assert_eq!(resolve_canonical_key(&entry_map, "ghost"), None);
    }

    #[test]
    fn empty_map_returns_none_for_any_query() {
        let empty_entry_map: BTreeMap<String, FakeEntry> = BTreeMap::new();
        assert_eq!(resolve_canonical_key(&empty_entry_map, "anything"), None);
    }
}
