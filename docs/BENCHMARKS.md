# docs/BENCHMARKS.md

Benchmarks to run later (do not run now).

1. Cold start
   - Command: `time harness -p "hi" --model gpt-4o-mini`
   - Record: wall time to first token, binary size.
2. Idle memory
   - Command: run harnessd, measure RSS after 60s idle.
   - Record: RSS MB.
3. 100 concurrent sessions
   - Command: script opening 100 WebSocket sessions sending a message each.
   - Record: p50/p95/p99 latency, peak RSS, CPU.
4. Tool-call overhead per turn
   - Command: eval run with a case requiring 5 tool calls.
   - Record: ms per turn, tokens.

Record numbers in docs/PERF.md.
EOF