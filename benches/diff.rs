use criterion::{Criterion, criterion_group, criterion_main};
use ratatui_code_editor::{bench_build_diff_rows, bench_build_diff_rows_fast, code::Code};

fn bench_diff_rows(c: &mut Criterion) {
    let cases = [
        ("insert-middle", make_insert_middle(2_000, 250)),
        ("delete-middle", make_delete_middle(2_000, 250)),
        ("replace-block", make_replace_block(2_000, 250)),
        ("separate-hunks", make_separate_hunks(2_000, 40)),
    ];

    for (name, (current, original)) in cases {
        let code = Code::new(&current, "unknown", None).unwrap();
        let original = Code::new(&original, "unknown", None).unwrap();

        let mut group = c.benchmark_group(format!("diff_rows/{name}"));
        group.bench_function("similar", |b| {
            b.iter(|| bench_build_diff_rows(&code, &original))
        });
        group.bench_function("fast", |b| {
            b.iter(|| bench_build_diff_rows_fast(&code, &original))
        });
        group.finish();
    }
}

fn base_lines(len: usize) -> Vec<String> {
    (0..len)
        .map(|idx| format!("let value_{idx} = {idx};"))
        .collect()
}

fn make_insert_middle(len: usize, inserted: usize) -> (String, String) {
    let original = base_lines(len);
    let mut current = original.clone();
    current.splice(
        len / 2..len / 2,
        (0..inserted).map(|idx| format!("let inserted_{idx} = {idx};")),
    );
    (current.join("\n"), original.join("\n"))
}

fn make_delete_middle(len: usize, deleted: usize) -> (String, String) {
    let original = base_lines(len);
    let mut current = original.clone();
    current.drain(len / 2..len / 2 + deleted);
    (current.join("\n"), original.join("\n"))
}

fn make_replace_block(len: usize, replaced: usize) -> (String, String) {
    let original = base_lines(len);
    let mut current = original.clone();
    current.splice(
        len / 2..len / 2 + replaced,
        (0..replaced).map(|idx| format!("let replacement_{idx} = {idx};")),
    );
    (current.join("\n"), original.join("\n"))
}

fn make_separate_hunks(len: usize, hunks: usize) -> (String, String) {
    let original = base_lines(len);
    let mut current = original.clone();
    let step = len / hunks;

    for idx in 0..hunks {
        let line_idx = idx * step;
        current[line_idx] = format!("let touched_{idx} = {idx};");
    }

    (current.join("\n"), original.join("\n"))
}

criterion_group!(benches, bench_diff_rows);
criterion_main!(benches);
