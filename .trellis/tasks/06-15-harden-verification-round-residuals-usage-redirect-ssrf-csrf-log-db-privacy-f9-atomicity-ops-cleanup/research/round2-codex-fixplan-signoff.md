A: AGREE. Real for token_plan/ZenMux; balance is mostly hardcoded today, but guarding the native-template arm is still correct policy and avoids signature churn.

B: AGREE. `split(':')` breaks bracketed IPv6; `Url::parse("http://{Host}")` is the right symmetric host extraction.

C: AGREE. The four `forwarder.rs` `record_result(... Some(e.to_string()))` paths can persist full upstream bodies into `provider_health.last_error`; `summarize_proxy_error(&e)` is the right fix.

D: AGREE. The queue write really happens before `switch_proxy_target`; `switch_proxy_target` uses the explicit provider id and does not read the queue. Defer the empty-queue add until after successful switch.

E: AGREE. Stale `RuntimeMode +` comment and F9/FIX naming should be cleaned up.

F: AGREE. Do not feature-gate. Desktop initial dials stay unchanged; blocking internal-IP literal redirect hops is acceptable and desirable here.

I found no additional material residual in the code delta. The proposed fixes do not introduce a new regression beyond intended web-side rejection/truncation behavior. converged