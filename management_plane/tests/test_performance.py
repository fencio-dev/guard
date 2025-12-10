"""
Performance benchmarking for Week 2 encoding pipeline.

Performance Targets (from CLAUDE.md):
- Rust sandbox comparison: < 1ms
- Python encoding pipeline: < 10ms
- Full comparison (100 candidates): < 100ms (P50 latency)
"""

import time
import numpy as np
from app.encoding import (
    encode_to_128d,
    encode_boundary_to_128d,
    encode_boundary_to_128d_cached,
    get_cache_stats,
    clear_cache,
)
from app.models import (
    IntentEvent,
    Actor,
    Resource,
    Data,
    Risk,
    DesignBoundary,
    BoundaryScope,
    BoundaryRules,
    SliceThresholds,
    SliceWeights,
)
from app.ffi_bridge import get_sandbox


def create_test_intent() -> IntentEvent:
    """Create a test IntentEvent."""
    return IntentEvent(
        id="perf_test_001",
        schemaVersion="v1",
        tenantId="tenant_test",
        timestamp=1700000000.0,
        action="read",
        actor=Actor(id="alice@example.com", type="user"),
        resource=Resource(type="database", name="users_db", location="cloud"),
        data=Data(categories=["pii", "financial"], pii=True, volume="row"),
        risk=Risk(authn="mfa", network="corp", timeOfDay=14),
    )


def create_test_boundary() -> DesignBoundary:
    """Create a test DesignBoundary."""
    return DesignBoundary(
        id="perf_boundary_001",
        name="Performance Test Boundary",
        status="active",
        type="mandatory",
        boundarySchemaVersion="v1",
        scope=BoundaryScope(
            tenantId="tenant_test",
            domains=["database", "api"],
        ),
        rules=BoundaryRules(
            thresholds=SliceThresholds(
                action=0.8,
                resource=0.75,
                data=0.7,
                risk=0.6,
            ),
            weights=SliceWeights(
                action=1.0,
                resource=1.0,
                data=1.0,
                risk=1.0,
            ),
            decision="min",
            globalThreshold=0.75,
        ),
        notes="Test boundary for performance benchmarking",
        createdAt=1700000000.0,
        updatedAt=1700000000.0,
    )


def benchmark_intent_encoding(iterations: int = 100) -> dict:
    """Benchmark intent encoding performance."""
    print("\n" + "=" * 60)
    print(f"Benchmarking Intent Encoding ({iterations} iterations)")
    print("=" * 60)

    event = create_test_intent()
    times = []

    # Warmup
    for _ in range(5):
        encode_to_128d(event)

    # Actual benchmark
    for i in range(iterations):
        start = time.perf_counter()
        vector = encode_to_128d(event)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    times = np.array(times)

    results = {
        "mean": np.mean(times),
        "median": np.median(times),
        "p50": np.percentile(times, 50),
        "p95": np.percentile(times, 95),
        "p99": np.percentile(times, 99),
        "min": np.min(times),
        "max": np.max(times),
    }

    print(f"Mean:   {results['mean']:.2f} ms")
    print(f"Median: {results['median']:.2f} ms")
    print(f"P50:    {results['p50']:.2f} ms")
    print(f"P95:    {results['p95']:.2f} ms")
    print(f"P99:    {results['p99']:.2f} ms")
    print(f"Min:    {results['min']:.2f} ms")
    print(f"Max:    {results['max']:.2f} ms")

    # Check target
    target = 10.0  # ms
    if results['p50'] < target:
        print(f"✅ PASS: P50 ({results['p50']:.2f} ms) < {target} ms target")
    else:
        print(f"❌ FAIL: P50 ({results['p50']:.2f} ms) >= {target} ms target")

    return results


def benchmark_boundary_encoding(iterations: int = 100) -> dict:
    """Benchmark boundary encoding performance (without cache)."""
    print("\n" + "=" * 60)
    print(f"Benchmarking Boundary Encoding - No Cache ({iterations} iterations)")
    print("=" * 60)

    boundary = create_test_boundary()
    times = []

    # Clear cache to test cold performance
    clear_cache()

    # Warmup
    for _ in range(5):
        encode_boundary_to_128d(boundary)

    # Actual benchmark
    for i in range(iterations):
        start = time.perf_counter()
        vector = encode_boundary_to_128d(boundary)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    times = np.array(times)

    results = {
        "mean": np.mean(times),
        "median": np.median(times),
        "p50": np.percentile(times, 50),
        "p95": np.percentile(times, 95),
        "p99": np.percentile(times, 99),
        "min": np.min(times),
        "max": np.max(times),
    }

    print(f"Mean:   {results['mean']:.2f} ms")
    print(f"Median: {results['median']:.2f} ms")
    print(f"P50:    {results['p50']:.2f} ms")
    print(f"P95:    {results['p95']:.2f} ms")
    print(f"P99:    {results['p99']:.2f} ms")
    print(f"Min:    {results['min']:.2f} ms")
    print(f"Max:    {results['max']:.2f} ms")

    return results


def benchmark_boundary_encoding_cached(iterations: int = 100) -> dict:
    """Benchmark boundary encoding performance (with cache)."""
    print("\n" + "=" * 60)
    print(f"Benchmarking Boundary Encoding - With Cache ({iterations} iterations)")
    print("=" * 60)

    boundary = create_test_boundary()
    boundary_json = boundary.model_dump_json()
    times = []

    # Clear cache first
    clear_cache()

    # Prime the cache
    encode_boundary_to_128d_cached(boundary.id, boundary_json)

    # Actual benchmark (should hit cache)
    for i in range(iterations):
        start = time.perf_counter()
        vector = encode_boundary_to_128d_cached(boundary.id, boundary_json)
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    times = np.array(times)

    results = {
        "mean": np.mean(times),
        "median": np.median(times),
        "p50": np.percentile(times, 50),
        "p95": np.percentile(times, 95),
        "p99": np.percentile(times, 99),
        "min": np.min(times),
        "max": np.max(times),
    }

    print(f"Mean:   {results['mean']:.2f} ms")
    print(f"Median: {results['median']:.2f} ms")
    print(f"P50:    {results['p50']:.2f} ms")
    print(f"P95:    {results['p95']:.2f} ms")
    print(f"P99:    {results['p99']:.2f} ms")
    print(f"Min:    {results['min']:.2f} ms")
    print(f"Max:    {results['max']:.2f} ms")

    # Show cache stats
    stats = get_cache_stats()
    print(f"\nCache Stats: hits={stats['hits']}, misses={stats['misses']}, size={stats['size']}")

    return results


def benchmark_rust_comparison(iterations: int = 1000) -> dict:
    """Benchmark Rust sandbox comparison performance."""
    print("\n" + "=" * 60)
    print(f"Benchmarking Rust Comparison ({iterations} iterations)")
    print("=" * 60)

    # Pre-encode vectors
    event = create_test_intent()
    boundary = create_test_boundary()

    intent_vector = encode_to_128d(event)
    boundary_vector = encode_boundary_to_128d(boundary)

    thresholds = [0.8, 0.75, 0.7, 0.6]
    weights = [1.0, 1.0, 1.0, 1.0]
    decision_mode = 0
    global_threshold = 0.75

    sandbox = get_sandbox()
    times = []

    # Warmup
    for _ in range(10):
        sandbox.compare(
            intent_vector=intent_vector,
            boundary_vector=boundary_vector,
            thresholds=thresholds,
            weights=weights,
            decision_mode=decision_mode,
            global_threshold=global_threshold,
        )

    # Actual benchmark
    for i in range(iterations):
        start = time.perf_counter()
        decision, similarities = sandbox.compare(
            intent_vector=intent_vector,
            boundary_vector=boundary_vector,
            thresholds=thresholds,
            weights=weights,
            decision_mode=decision_mode,
            global_threshold=global_threshold,
        )
        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    times = np.array(times)

    results = {
        "mean": np.mean(times),
        "median": np.median(times),
        "p50": np.percentile(times, 50),
        "p95": np.percentile(times, 95),
        "p99": np.percentile(times, 99),
        "min": np.min(times),
        "max": np.max(times),
    }

    print(f"Mean:   {results['mean']:.4f} ms")
    print(f"Median: {results['median']:.4f} ms")
    print(f"P50:    {results['p50']:.4f} ms")
    print(f"P95:    {results['p95']:.4f} ms")
    print(f"P99:    {results['p99']:.4f} ms")
    print(f"Min:    {results['min']:.4f} ms")
    print(f"Max:    {results['max']:.4f} ms")

    # Check target
    target = 1.0  # ms
    if results['p50'] < target:
        print(f"✅ PASS: P50 ({results['p50']:.4f} ms) < {target} ms target")
    else:
        print(f"❌ FAIL: P50 ({results['p50']:.4f} ms) >= {target} ms target")

    return results


def benchmark_full_pipeline(num_boundaries: int = 100, iterations: int = 10) -> dict:
    """Benchmark full comparison pipeline with multiple boundaries."""
    print("\n" + "=" * 60)
    print(f"Benchmarking Full Pipeline ({num_boundaries} boundaries, {iterations} iterations)")
    print("=" * 60)

    # Create test data
    event = create_test_intent()
    boundaries = [create_test_boundary() for _ in range(num_boundaries)]

    times = []

    for i in range(iterations):
        start = time.perf_counter()

        # Encode intent
        intent_vector = encode_to_128d(event)

        # Compare against all boundaries
        sandbox = get_sandbox()
        for boundary in boundaries:
            boundary_json = boundary.model_dump_json()
            boundary_vector = encode_boundary_to_128d_cached(boundary.id, boundary_json)

            thresholds = [
                boundary.rules.thresholds.action,
                boundary.rules.thresholds.resource,
                boundary.rules.thresholds.data,
                boundary.rules.thresholds.risk,
            ]
            weights = [1.0, 1.0, 1.0, 1.0]
            decision_mode = 0 if boundary.rules.decision == "min" else 1
            global_threshold = boundary.rules.globalThreshold or 0.75

            decision, similarities = sandbox.compare(
                intent_vector=intent_vector,
                boundary_vector=boundary_vector,
                thresholds=thresholds,
                weights=weights,
                decision_mode=decision_mode,
                global_threshold=global_threshold,
            )

        end = time.perf_counter()
        times.append((end - start) * 1000)  # Convert to ms

    times = np.array(times)

    results = {
        "mean": np.mean(times),
        "median": np.median(times),
        "p50": np.percentile(times, 50),
        "p95": np.percentile(times, 95),
        "p99": np.percentile(times, 99),
        "min": np.min(times),
        "max": np.max(times),
    }

    print(f"Mean:   {results['mean']:.2f} ms")
    print(f"Median: {results['median']:.2f} ms")
    print(f"P50:    {results['p50']:.2f} ms")
    print(f"P95:    {results['p95']:.2f} ms")
    print(f"P99:    {results['p99']:.2f} ms")
    print(f"Min:    {results['min']:.2f} ms")
    print(f"Max:    {results['max']:.2f} ms")

    # Check target
    target = 100.0  # ms
    if results['p50'] < target:
        print(f"✅ PASS: P50 ({results['p50']:.2f} ms) < {target} ms target")
    else:
        print(f"❌ FAIL: P50 ({results['p50']:.2f} ms) >= {target} ms target")

    return results


if __name__ == "__main__":
    print("\n" + "=" * 60)
    print("Performance Benchmarking - Week 2 Encoding Pipeline")
    print("=" * 60)

    # Run all benchmarks
    intent_results = benchmark_intent_encoding(iterations=100)
    boundary_results = benchmark_boundary_encoding(iterations=100)
    boundary_cached_results = benchmark_boundary_encoding_cached(iterations=100)
    rust_results = benchmark_rust_comparison(iterations=1000)
    full_results = benchmark_full_pipeline(num_boundaries=100, iterations=10)

    # Summary
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    print(f"Intent Encoding (P50):         {intent_results['p50']:>8.2f} ms  (target: < 10 ms)")
    print(f"Boundary Encoding (P50):       {boundary_results['p50']:>8.2f} ms  (target: < 10 ms)")
    print(f"Boundary Cached (P50):         {boundary_cached_results['p50']:>8.4f} ms  (cache speedup)")
    print(f"Rust Comparison (P50):         {rust_results['p50']:>8.4f} ms  (target: < 1 ms)")
    print(f"Full Pipeline 100x (P50):      {full_results['p50']:>8.2f} ms  (target: < 100 ms)")

    print("\n" + "=" * 60)
    print("Performance Targets:")
    print("=" * 60)

    all_pass = True

    if intent_results['p50'] < 10.0:
        print("✅ Intent encoding: PASS")
    else:
        print("❌ Intent encoding: FAIL")
        all_pass = False

    if rust_results['p50'] < 1.0:
        print("✅ Rust comparison: PASS")
    else:
        print("❌ Rust comparison: FAIL")
        all_pass = False

    if full_results['p50'] < 100.0:
        print("✅ Full pipeline (100 boundaries): PASS")
    else:
        print("❌ Full pipeline (100 boundaries): FAIL")
        all_pass = False

    if all_pass:
        print("\n🎉 All performance targets met!")
    else:
        print("\n⚠️  Some performance targets not met")

    print("=" * 60)
