//! I/O 低水準基盤 — アトミック書込・プロセス spawn (A4)・テレメトリ配線 (A10)。依存できるのはアダプタ層と composition root のみ (D4)。I/O の責務は Gateways にある。

#![forbid(unsafe_code)]
