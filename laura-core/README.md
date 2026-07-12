# call-laura-core

(Package name: `call-laura-core` — the crates.io name; source lives in the `laura-core/` directory, unchanged, to avoid a needless folder rename.)

Pure review logic behind [`call-laura`](https://github.com/rfi-irfos/call-laura) —
structured document review grounded in Laura Serna Gaviria's Human–AI
Co-Evolution research framework, plus two RFI-IRFOS-original lenses.

Fully deterministic and local: no network call, no API key, no async runtime
needed. Same input always produces the same output.

**Read the workspace root README first**, especially "Attribution & Sourcing" —
it explains exactly which parts of this crate's output are genuinely Laura Serna
Gaviria's own published work versus this project's own operationalizations or
additions. That distinction is load-bearing for how you should read any output
this crate produces.

## Usage

```rust
use call_laura_core::schema::{Lens, ReviewRequest};

let req = ReviewRequest {
    text: "your document here".into(),
    lenses: Lens::ALL.to_vec(),
    metadata: None,
};
let response = call_laura_core::review(&req);
```

License: LGPL-3.0-or-later. See the workspace root `LICENSE-LGPL`.
