use std::path::PathBuf;

use qimeng_scut::manual::ManualRepo;

fn fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn indexes_entries_with_relative_paths_inside_sections() {
    let repo = ManualRepo::load(fixture_root("manual_repo")).expect("load repo");

    let health_section = repo
        .sections
        .iter()
        .find(|section| section.title == "health")
        .expect("health section");

    let entries = health_section
        .entries
        .iter()
        .map(|entry| {
            (
                entry.title.as_str(),
                entry.relative_path.to_string_lossy().to_string(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        entries,
        vec![
            ("就医", "medical.md".to_string()),
            ("先活下去", "nested/alive.md".to_string()),
        ]
    );
}
