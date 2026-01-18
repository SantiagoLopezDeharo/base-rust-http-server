/*
 * Concurrency and sequential test for http://localhost:8080/test
 *
 * Run with:
 *   node concurrency_test.js
 */

const TARGET_URL = "http://localhost:8080/test";
const CONCURRENCY = 1000;

async function concurrentTest() {
  const start = Date.now();
  const requests = Array.from({ length: CONCURRENCY }, async () => {
    const res = await fetch(TARGET_URL);
    const text = await res.text();
    return { status: res.status, bytes: text.length };
  });
  const results = await Promise.allSettled(requests);
  const totalMs = Date.now() - start;
  return { totalMs, results };
}

async function sequentialTest() {
  const start = Date.now();
  const results = [];
  for (let i = 0; i < CONCURRENCY; i++) {
    try {
      const res = await fetch(TARGET_URL);
      const text = await res.text();
      results.push({
        status: "fulfilled",
        value: { status: res.status, bytes: text.length },
      });
    } catch (err) {
      results.push({ status: "rejected", reason: err });
    }
  }
  const totalMs = Date.now() - start;
  return { totalMs, results };
}

async function run() {
  console.log(`Running ${CONCURRENCY} requests concurrently...`);
  const concurrent = await concurrentTest();
  const fulfilledC = concurrent.results.filter(
    (r) => r.status === "fulfilled",
  ).length;
  const rejectedC = concurrent.results.length - fulfilledC;
  console.log(
    `Concurrent: ${concurrent.totalMs} ms | Fulfilled: ${fulfilledC}, Rejected: ${rejectedC}`,
  );

  console.log(`\nRunning ${CONCURRENCY} requests sequentially...`);
  const sequential = await sequentialTest();
  const fulfilledS = sequential.results.filter(
    (r) => r.status === "fulfilled",
  ).length;
  const rejectedS = sequential.results.length - fulfilledS;
  console.log(
    `Sequential: ${sequential.totalMs} ms | Fulfilled: ${fulfilledS}, Rejected: ${rejectedS}`,
  );

  console.log("\n--- Comparison ---");
  if (concurrent.totalMs < sequential.totalMs) {
    console.log(
      `Concurrent requests were faster by ${sequential.totalMs - concurrent.totalMs} ms.`,
    );
  } else if (concurrent.totalMs > sequential.totalMs) {
    console.log(
      `Sequential requests were faster by ${concurrent.totalMs - sequential.totalMs} ms (unexpected).`,
    );
  } else {
    console.log("Both methods took the same time.");
  }
}

run().catch((err) => {
  console.error("Unexpected error:", err);
  process.exit(1);
});
