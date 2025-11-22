use codeinput::core::resolver::find_owners_and_tags_for_file;
use codeinput::core::types::{
    codeowners_entry_to_matcher, CodeownersEntry, CodeownersEntryMatcher, Owner, OwnerType, Tag,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::{Path, PathBuf};

fn create_test_tag(name: &str) -> Tag {
    Tag(name.to_string())
}

fn create_test_owner(identifier: &str, owner_type: OwnerType) -> Owner {
    Owner {
        identifier: identifier.to_string(),
        owner_type,
    }
}

fn create_test_codeowners_entry_matcher(
    source_file: &str, line_number: usize, pattern: &str, owners: Vec<Owner>, tags: Vec<Tag>,
) -> CodeownersEntryMatcher {
    let entry = CodeownersEntry {
        source_file: PathBuf::from(source_file),
        line_number,
        pattern: pattern.to_string(),
        owners,
        tags,
    };
    codeowners_entry_to_matcher(&entry).expect("Failed to create matcher in benchmark")
}

fn bench_find_owners_and_tags_simple_pattern(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
            vec![create_test_tag("rust")],
        ),
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            2,
            "*.js",
            vec![create_test_owner("@js-team", OwnerType::Team)],
            vec![create_test_tag("javascript")],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_and_tags_simple", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_and_tags_complex_patterns(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@global-team", OwnerType::Team)],
            vec![create_test_tag("global")],
        ),
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            5,
            "src/**/*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
            vec![create_test_tag("rust-source")],
        ),
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            10,
            "src/frontend/**/*",
            vec![create_test_owner("@frontend-team", OwnerType::Team)],
            vec![create_test_tag("frontend")],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/main.rs");

    c.bench_function("find_owners_and_tags_complex", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_and_tags_many_entries(c: &mut Criterion) {
    let mut entries = Vec::new();

    // Create many entries with different patterns
    for i in 0..100 {
        entries.push(create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            i + 1,
            &format!("src/module_{}/**/*", i),
            vec![create_test_owner(&format!("@team-{}", i), OwnerType::Team)],
            vec![create_test_tag(&format!("module-{}", i))],
        ));
    }

    let file_path = Path::new("/project/src/module_50/file.rs");

    c.bench_function("find_owners_and_tags_many_entries", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_and_tags_nested_codeowners(c: &mut Criterion) {
    let entries = vec![
        // Root CODEOWNERS
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@root-team", OwnerType::Team)],
            vec![create_test_tag("root")],
        ),
        // Nested CODEOWNERS in src/
        create_test_codeowners_entry_matcher(
            "/project/src/CODEOWNERS",
            1,
            "*.rs",
            vec![create_test_owner("@rust-team", OwnerType::Team)],
            vec![create_test_tag("rust")],
        ),
        // Nested CODEOWNERS in src/frontend/
        create_test_codeowners_entry_matcher(
            "/project/src/frontend/CODEOWNERS",
            1,
            "*.tsx",
            vec![create_test_owner("@frontend-team", OwnerType::Team)],
            vec![create_test_tag("frontend")],
        ),
    ];

    let file_path = Path::new("/project/src/frontend/component.tsx");

    c.bench_function("find_owners_and_tags_nested", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_and_tags_no_matches(c: &mut Criterion) {
    let entries = vec![create_test_codeowners_entry_matcher(
        "/project/CODEOWNERS",
        1,
        "*.js",
        vec![create_test_owner("@js-team", OwnerType::Team)],
        vec![create_test_tag("javascript")],
    )];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_and_tags_no_matches", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

fn bench_find_owners_and_tags_multiple_matches(c: &mut Criterion) {
    let entries = vec![
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            1,
            "*",
            vec![create_test_owner("@global-team", OwnerType::Team)],
            vec![create_test_tag("global")],
        ),
        create_test_codeowners_entry_matcher(
            "/project/CODEOWNERS",
            2,
            "*.rs",
            vec![
                create_test_owner("@rust-team", OwnerType::Team),
                create_test_owner("@reviewer", OwnerType::User),
            ],
            vec![create_test_tag("rust"), create_test_tag("code")],
        ),
    ];

    let file_path = Path::new("/project/src/main.rs");

    c.bench_function("find_owners_and_tags_multiple_matches", |b| {
        b.iter(|| find_owners_and_tags_for_file(black_box(file_path), black_box(&entries)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_find_owners_and_tags_simple_pattern,
    bench_find_owners_and_tags_complex_patterns,
    bench_find_owners_and_tags_many_entries,
    bench_find_owners_and_tags_nested_codeowners,
    bench_find_owners_and_tags_no_matches,
    bench_find_owners_and_tags_multiple_matches,
);
criterion_main!(benches);
