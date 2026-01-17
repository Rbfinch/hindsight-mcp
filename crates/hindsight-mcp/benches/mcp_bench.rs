use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use hindsight_mcp::db::Database;
use hindsight_mcp::queries;

/// Create an in-memory database with sample data for benchmarking
fn setup_benchmark_db() -> Database {
    let db = Database::in_memory().expect("Failed to create in-memory database");
    db.initialize().expect("Failed to initialize database");

    // Insert sample workspace
    let workspace = hindsight_mcp::db::WorkspaceRecord::new(
        "bench-workspace".to_string(),
        "/bench/workspace".to_string(),
    );
    db.insert_workspace(&workspace)
        .expect("Failed to insert workspace");

    // Insert sample commits
    for i in 0..100 {
        let commit = hindsight_mcp::db::CommitRecord::new(
            workspace.id.clone(),
            format!("{:040x}", i),
            format!("Author {}", i % 10),
            Some(format!("author{}@example.com", i % 10)),
            format!(
                "Commit message {} with searchable content about refactoring",
                i
            ),
            chrono::Utc::now(),
        );
        db.insert_commit(&commit).expect("Failed to insert commit");
    }

    db
}

fn query_benchmarks(c: &mut Criterion) {
    let db = setup_benchmark_db();

    let mut group = c.benchmark_group("queries");

    // Timeline query benchmark
    group.bench_function("get_timeline_50", |b| {
        b.iter(|| queries::get_timeline(db.connection(), 50, None).expect("timeline query failed"))
    });

    // Search benchmark
    group.bench_function("search_commits", |b| {
        b.iter(|| {
            queries::search_commits(db.connection(), "refactoring", 20).expect("search failed")
        })
    });

    // Activity summary benchmark
    group.bench_function("activity_summary_7_days", |b| {
        b.iter(|| queries::get_activity_summary(db.connection(), 7).expect("summary failed"))
    });

    // Failing tests benchmark (empty result is still valid)
    group.bench_function("failing_tests", |b| {
        b.iter(|| {
            queries::get_failing_tests(db.connection(), 50, None, None)
                .expect("failing tests failed")
        })
    });

    group.finish();
}

fn database_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("database");

    // Database open benchmark
    group.bench_function("open_in_memory", |b| {
        b.iter(|| Database::in_memory().expect("Failed to create database"))
    });

    // Database initialize benchmark
    group.bench_function("initialize", |b| {
        b.iter(|| {
            let db = Database::in_memory().expect("Failed to create database");
            db.initialize().expect("Failed to initialize");
        })
    });

    group.finish();
}

fn scaling_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("scaling");

    for size in [10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("timeline_limit", size),
            size,
            |b, &limit| {
                let db = setup_benchmark_db();
                b.iter(|| {
                    queries::get_timeline(db.connection(), limit, None).expect("timeline failed")
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    query_benchmarks,
    database_benchmarks,
    scaling_benchmarks
);
criterion_main!(benches);
