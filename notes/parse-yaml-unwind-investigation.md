# parse_yaml unwind safety investigation

## Issue summary
The current error-handling path in `parse_yaml()` delegates directly to `handle_eval_error()`, which resumes R's longjmp before Rust stack frames finish unwinding. When `parse_yaml()` allocates locals (e.g., buffers or helper structs) those destructors may be skipped on error paths, leading to leaked memory and other cleanup not running.

## Analysis
- `parse_yaml()` previously invoked `yaml_to_r::parse_yaml_impl()` and immediately routed failures to `handle_eval_error()`. Because `handle_eval_error()` resumes the R continuation for `EvalError::Jump`, the Rust frame containing `parse_yaml()` never unwound, so `Drop` implementations in that frame were bypassed.
- Instrumentation via a `DropProbe` counter (constructed at the top of `parse_yaml()`) showed destructor invocations on successful parses but no drop on jump-triggered errors (e.g., NA inputs that raise an R error). This confirmed the unwinding gap.

## Experiments
- Added a `DropProbe` in `parse_yaml()` and enabled logging with `YAML12_DROP_PROBE_LOG=1`. Calling `yaml12::parse_yaml("foo: bar")` printed the drop log, while `yaml12::parse_yaml(NA_character_)` raised an error without emitting the drop log, demonstrating the frame was not unwound on the error path.
- Keeping the parser work inside a scoped block ensured the probe dropped before mapping errors back into R, so counters incremented for ok, API error, and jump-token scenarios.

## Proposed approach
- Keep entrypoints free of owned locals; do work inside the corresponding `*_impl` function. If per-call state is needed in an entrypoint, wrap it in a block that produces the `Fallible` result so locals drop before calling `handle_eval_error()`.
- Lightweight drop flags/probes are helpful during development to detect skipped drops on error paths; they should live in scoped blocks so they always drop before propagating errors.
- Validate in CI with unit tests that confirm scoped locals drop on success, API error, and jump-token cases, guarding against future changes that bypass unwinding.

## Notes for larger codebases
- Any Rust function that may resume an R continuation (or other foreign longjmp) should execute its work inside a scope that returns a `Result`, ensuring Rust owns the stack frame until after error mapping occurs.
- Longjmp resumptions (`resume()`/`R_ContinueUnwind`) must only be invoked after Rust scopes finish; otherwise, destructors are skipped and resources leak. Lightweight probes and counters can quickly surface when unwinding is bypassed.
