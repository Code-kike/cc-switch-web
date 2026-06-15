A — AGREE.  
B — AGREE.  
C — AGREE.  
D — AGREE.  
E — AGREE.  
F — AGREE.

No desktop SSRF enforcement added, no Tauri dependency introduced into web-reachable modules, and web SSRF remains web-only. No incomplete round-2 change found. `git diff HEAD` is limited to the intended files; one untracked Trellis research note exists outside the diff.

Verified with `cargo check --example server --no-default-features --features web-server`, `cargo check --features desktop`, and focused web/desktop unit tests for A-D. Warnings only.

verified