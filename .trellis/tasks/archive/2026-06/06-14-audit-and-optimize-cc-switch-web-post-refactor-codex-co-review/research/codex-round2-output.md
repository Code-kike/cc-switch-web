Signed off. No final dissent. F11 is agreed as: blocking std DNS in `validate_outbound_url` plus unbounded speed-test fan-out.

My fix-first ranking for a personal Tailscale deployment:

1. **C1 path traversal**
   Proven unauth arbitrary file read, likely tiny fix, highest risk-to-effort ratio.

2. **C2 API auth/CSRF/rate-limit**
   Even on Tailscale, this API can export secrets, import SQL, and rewrite proxy takeover state. Minimal auth buys down the whole surface.

3. **F3 plus F4 SSRF guard coverage**
   The guard already exists; apply it consistently to WebDAV/S3 and ZenMux. This prevents the server becoming an internal-network pivot.

4. **F7 circuit-breaker half-open bypass**
   This affects the core proxy reliability path. If failover is not actively used, I’d swap this with **F10 auto-sync no-op UI**, because silent “backup enabled but nothing happens” is operationally nasty.