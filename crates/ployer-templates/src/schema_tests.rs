use super::*;

// ---------------------------------------------------------------------------
// Cross-repo compatibility: the removed `icon` key
//
// `icon` was dropped from `Template` and `IndexEntry` in ac178ed, but
// `ployer` (this binary) and `ployer-website` (which generates and serves the
// registry) deploy independently. The published catalog still ships the key on
// every entry, so the parser has to keep ignoring it.
//
// Verified against the live registry on 2026-10-29:
//
//   $ curl -s https://ployer.nusendra.com/templates/index.json | grep -c '"icon"'
//   4                                   # jellyfin, mariadb, mongodb, postgresql
//   $ curl -s https://ployer.nusendra.com/templates/postgresql.yaml | grep icon
//   icon: postgresql.svg
//
// These two tests exist to fail loudly if anyone adds
// `#[serde(deny_unknown_fields)]` to either struct. That attribute looks like
// tightening; here it would make every install fail on a field nobody reads.
// ---------------------------------------------------------------------------

/// The index is fetched before anything else, so a strict parser here breaks
/// the catalog listing outright — no template is installable.
#[test]
fn an_index_entry_carrying_the_removed_icon_key_still_parses() {
    // Shape copied from the live index.json, trimmed to one entry.
    let json = r#"{
      "schema_version": 1,
      "generated_at": "2026-10-29T04:56:52.749Z",
      "templates": [{
        "slug": "postgresql",
        "name": "PostgreSQL",
        "description": "Object-relational database with strong SQL standards compliance.",
        "category": "database",
        "icon": "postgresql.svg",
        "tags": ["database", "sql", "postgres"],
        "version": 1,
        "sha256": "00",
        "url": "https://ployer.nusendra.com/templates/postgresql.yaml"
      }]
    }"#;

    let index: RegistryIndex =
        serde_json::from_str(json).expect("a stale `icon` key must not break index parsing");

    assert_eq!(index.templates[0].slug, "postgresql");
    assert_eq!(index.templates[0].tags, ["database", "sql", "postgres"]);
}

/// Same guarantee one level down: the template YAML served from the registry
/// still declares `icon:`, so `Template` must ignore it too.
#[test]
fn a_template_carrying_the_removed_icon_key_still_parses() {
    // Shape copied from the live postgresql.yaml, trimmed to the required keys.
    let yaml = r#"
slug: postgresql
name: PostgreSQL
description: Object-relational database with strong SQL standards compliance.
category: database
icon: postgresql.svg
tags: [database, sql, postgres]
version: 1
compose: |
  services:
    postgresql:
      image: postgres:17-alpine
"#;

    let t: Template =
        serde_yaml::from_str(yaml).expect("a stale `icon:` must not break template parsing");

    assert_eq!(t.slug, "postgresql");
    assert!(t.compose.contains("image: postgres:17-alpine"));
}
