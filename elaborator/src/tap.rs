/// TAP (Test Anything Protocol) v14 output.
/// Writes to stdout.
pub struct Tap {
    tests: Vec<TapTest>,
}

struct TapTest {
    ok: bool,
    desc: String,
    diagnostics: Option<String>,
}

impl Tap {
    pub fn new() -> Self {
        Tap { tests: Vec::new() }
    }

    pub fn ok(&mut self, desc: impl Into<String>) {
        self.tests.push(TapTest {
            ok: true,
            desc: desc.into(),
            diagnostics: None,
        });
    }

    pub fn not_ok(&mut self, desc: impl Into<String>, diagnostics: impl Into<String>) {
        self.tests.push(TapTest {
            ok: false,
            desc: desc.into(),
            diagnostics: Some(diagnostics.into()),
        });
    }

    pub fn finish(self) {
        println!("TAP version 14");
        println!("1..{}", self.tests.len());
        let mut pass = 0usize;
        let mut fail = 0usize;
        for (i, t) in self.tests.iter().enumerate() {
            let n = i + 1;
            if t.ok {
                println!("ok {} - {}", n, t.desc);
                pass += 1;
            } else {
                println!("not ok {} - {}", n, t.desc);
                if let Some(diag) = &t.diagnostics {
                    // TAP diagnostics are prefixed with "# "
                    for line in diag.lines() {
                        println!("  # {}", line);
                    }
                }
                fail += 1;
            }
        }
        // Summary
        println!("# tests {}", self.tests.len());
        println!("# pass  {}", pass);
        println!("# fail  {}", fail);
    }

    pub fn failure_count(&self) -> usize {
        self.tests.iter().filter(|t| !t.ok).count()
    }
}
