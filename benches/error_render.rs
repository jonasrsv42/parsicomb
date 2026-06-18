//! Benchmarks for the cost of *rendering* a `ParsicombError`.
//!
//! Constructing a `CodeLoc`/`ParsicombError` is cheap — it just stores a slice
//! reference and a position. But the `Display` impl computes a human-readable
//! line/column (`readable_position`) and a context snippet (`context_lines`),
//! and the latter calls `Atomic::format_slice(&self.code)` over the *entire*
//! input buffer (`String::from_utf8_lossy(whole_buffer).to_string()` for `u8`).
//!
//! For large inputs (e.g. a binary blob with a big embedded payload) this makes
//! formatting a one-line error message O(input size) with a full-buffer
//! allocation, even though the error position is already known.
//!
//! This bench pins that cost so we can prove the fix bounds it.
//!
//! Run with: `cargo bench --bench error_render`

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use parsicomb::{CodeLoc, ParsicombError};
use std::borrow::Cow;
use std::hint::black_box;

/// Build a buffer of `size` bytes with no newlines (mimics a binary payload:
/// one giant "line" as far as the text-oriented error renderer is concerned).
fn binary_buffer(size: usize) -> Vec<u8> {
    vec![0xABu8; size]
}

/// Build the error blobfig actually produces on the failure path: a syntax
/// error whose location buffer spans the whole input, positioned *after* the
/// big payload.
fn error_at_end(buf: &[u8]) -> ParsicombError<'_> {
    ParsicombError::SyntaxError {
        message: Cow::Borrowed("invalid value tag"),
        loc: CodeLoc::new(buf, buf.len().saturating_sub(1)),
    }
}

fn bench(c: &mut Criterion) {
    let sizes = [1 << 20, 8 << 20, 64 << 20]; // 1 MiB, 8 MiB, 64 MiB

    // Constructing the error: should be flat (~ns) regardless of input size.
    let mut build = c.benchmark_group("error_construct");
    for &size in &sizes {
        let buf = binary_buffer(size);
        build.throughput(Throughput::Bytes(buf.len() as u64));
        build.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| {
                let e = error_at_end(black_box(buf));
                black_box(e.position());
            });
        });
    }
    build.finish();

    // Rendering the error to a String: currently O(input size) + full alloc.
    let mut render = c.benchmark_group("error_render");
    for &size in &sizes {
        let buf = binary_buffer(size);
        render.throughput(Throughput::Bytes(buf.len() as u64));
        render.bench_with_input(BenchmarkId::from_parameter(size), &buf, |b, buf| {
            b.iter(|| {
                let e = error_at_end(black_box(buf));
                black_box(e.to_string());
            });
        });
    }
    render.finish();

    // Reference: rendering an error on a small (normal source-file sized)
    // input. This is the cost we want large inputs to stay close to.
    let mut small = c.benchmark_group("error_render_small");
    let buf = b"let x = ;\nlet y = 2;\n".to_vec();
    small.bench_function("20_bytes", |b| {
        b.iter(|| {
            let e: ParsicombError<'_, u8> = ParsicombError::SyntaxError {
                message: Cow::Borrowed("expected expression"),
                loc: CodeLoc::new(black_box(&buf), 8),
            };
            black_box(e.to_string());
        });
    });
    small.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
