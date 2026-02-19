# Fuzzing Test Suite

This directory contains fuzzing targets for security-critical input validation.

## Structure

```
fuzz_targets/
├── authentication/          # User authentication domain
│   ├── fuzz_user_email.rs
│   ├── fuzz_user_password.rs
│   ├── fuzz_user_email_unicode.rs
│   ├── fuzz_email_at_symbol_variations.rs
│   └── fuzz_password_grapheme_counting.rs
└── (future domains: newsletter/, posts/, comments/)
```

## Running Fuzzers

### 🚨 Important: Use `make` commands for fuzzing

Fuzzing requires **Rust nightly**, but the rest of the project uses **stable Rust**.  
The Makefile automatically switches to nightly **only for fuzzing**, so you should **not** run `cargo fuzz` directly.

---

### ✅ Run an individual fuzzer

```bash
make fuzz-single name=fuzz_user_email duration=60
```

### 🔥 Run with more intensity

```bash
make fuzz-single name=fuzz_user_email duration=300
```

### 🧩 Run all fuzzers

```bash
make fuzz
```

### 🚀 Run all fuzzers intensive (300s per fuzzer)

```bash
make fuzz-intensive
```

### Run fuzzers by domain

```bash
make fuzz-domain name=authentication
```

### 📊 Generate fuzz coverage report

```bash
make fuzz-coverage name=fuzz_user_email
```

### 🧹 Clean fuzz artifacts (corpus + crash cases)

```bash
make fuzz-clean
```

## What Each Fuzzer Tests

### Authentication Domain

| Fuzzer                            | Purpose                    | Security Focus                    |
|-----------------------------------|----------------------------|-----------------------------------|
| `fuzz_user_email`                 | Basic email validation     | Injection, bypass attempts        |
| `fuzz_user_password`              | Password parsing           | Auth-critical, no crashes allowed |
| `fuzz_user_email_unicode`         | Unicode edge cases         | IDN homograph attacks             |
| `fuzz_email_at_symbol_variations` | @ symbol detection         | Most common validation bypass     |
| `fuzz_password_grapheme_counting` | Complex character handling | Emoji, combining characters       |

## When Crashes Are Found

1. Crash artifacts saved to: `fuzz/artifacts/<fuzzer_name>/crash-...`
2. Reproduce: `cargo fuzz run <fuzzer_name> fuzz/artifacts/.../crash-...`
3. Fix the bug in domain validation
4. Add test case to unit tests
5. Verify fix: Re-run fuzzer

## CI Integration

Fuzzers run nightly via `.github/workflows/fuzzing.yml` (5 minutes per domain)

## IDE Warnings

RustRover may show "module declaration missing" warnings - this is expected.
Fuzz targets use `#![no_main]` and are not normal Rust modules.

**Fix**: Right-click `fuzz/fuzz_targets/` → Mark Directory as → Excluded

## 📝 Adding New Fuzzers - Complete Checklist

Follow these steps when adding fuzzing for a new domain (e.g., posts, comments, newsletter):

### Step 1: Decide What to Fuzz

**✅ Fuzz if:**

- Security-critical (authentication, payment, file upload)
- Parsing untrusted input (HTML, markdown, user files)
- History of production bugs from edge cases
- Complex validation logic (email, URLs, custom formats)

**❌ Skip if:**

- Already covered by property-based tests (proptest)
- Simple validation (length checks, basic regex)
- Well-tested library doing the parsing (e.g., html5ever)

**Example Decision:**

```
Posts Domain:
  ✅ fuzz_post_content_markdown    (parses user input)
  ✅ fuzz_post_mentions            (@ handling, injection risk)
  ❌ fuzz_post_title               (simple length check, proptest sufficient)
```

### Step 2: Create Domain Directory

```bash
# Create directory for new domain
mkdir -p fuzz/fuzz_targets/<domain_name>

# Examples:
mkdir -p fuzz/fuzz_targets/posts
mkdir -p fuzz/fuzz_targets/comments
mkdir -p fuzz/fuzz_targets/newsletter
```

### Step 3: Write Fuzz Targets

Create fuzzer files in your new domain directory:

```rust
// Example: fuzz/fuzz_targets/posts/fuzz_post_content.rs
#![no_main]

use libfuzzer_sys::fuzz_target;
use techhub::domain::PostContent;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = PostContent::parse(s.to_string());
    }
});
```

**Naming Convention:**

- `fuzz_<type>_<what>.rs`
- Examples: `fuzz_post_content.rs`, `fuzz_comment_mentions.rs`, `fuzz_newsletter_html.rs`

### Step 4: Update `fuzz/Cargo.toml`

Add binary declarations for each new fuzzer:

```toml
# Posts domain fuzzers
[[bin]]
name = "fuzz_post_content"
path = "fuzz_targets/posts/fuzz_post_content.rs"
test = false
doc = false

[[bin]]
name = "fuzz_post_mentions"
path = "fuzz_targets/posts/fuzz_post_mentions.rs"
test = false
doc = false
```

**Tip:** Group by domain with comments for easy navigation.

### Step 5: Create Domain Helper Script

Create `scripts/fuzz/fuzz_<domain>.sh`:

```bash
#!/bin/bash
set -e

echo "=========================================="
echo "Fuzzing Posts Domain"
echo "=========================================="

FUZZERS=(
    "fuzz_post_content"
    "fuzz_post_mentions"
)

DURATION=60
MAX_LEN=10000  # Adjust based on expected input size

for fuzzer in "${FUZZERS[@]}"; do
    echo ""
    echo "Running $fuzzer (${DURATION}s)..."
    cargo fuzz run "$fuzzer" -- \
        -max_len="$MAX_LEN" \
        -max_total_time="$DURATION" \
        || echo "$fuzzer found issues"
done

echo ""
echo "Posts fuzzing complete!"
```

**Make it executable:**

```bash
chmod +x scripts/fuzz/fuzz_posts.sh
```

### Step 6: Update CI Workflow

Add new job to `.github/workflows/fuzzing.yml`:

```yaml
  fuzz-posts:
    name: Fuzz Posts Domain
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust nightly
        uses: actions-rs/toolchain@v1
        with:
          toolchain: nightly
          override: true

      - name: Install cargo-fuzz
        run: cargo install cargo-fuzz

      - name: Cache fuzz corpus
        uses: actions/cache@v3
        with:
          path: fuzz/corpus
          key: fuzz-corpus-posts-${{ github.sha }}
          restore-keys: fuzz-corpus-posts-

      - name: Run posts fuzzers
        run: |
          FUZZERS=("fuzz_post_content" "fuzz_post_mentions")

          for fuzzer in "${FUZZERS[@]}"; do
            echo "::group::Fuzzing $fuzzer"
            timeout 300 cargo fuzz run "$fuzzer" -- \
              -max_len=10000 \
              -max_total_time=60 \
              || true
            echo "::endgroup::"
          done

      - name: Upload crash artifacts
        if: failure()
        uses: actions/upload-artifact@v3
        with:
          name: fuzz-crashes-posts-${{ github.run_id }}
          path: fuzz/artifacts/**/*
          if-no-files-found: ignore
```

**Don't forget:** Update the `paths:` trigger to include your new domain:

```yaml
on:
  pull_request:
    paths:
      - 'src/domain/**'
      - 'fuzz/**'
```

### Step 7: Update THIS README

Add documentation for your new domain:

#### 7a. Update Structure Section

```
fuzz_targets/
├── authentication/
├── posts/                   # ← ADD THIS
│   ├── fuzz_post_content.rs
│   └── fuzz_post_mentions.rs
└── comments/
```

#### 7b. Add Domain Table

Add your new domain's fuzzer table below the existing ones:

**Example for Posts Domain:**

```
### Posts Domain

| Fuzzer | Purpose | Security Focus |
|--------|---------|----------------|
| fuzz_post_content | Post content validation | Markdown injection, XSS |
| fuzz_post_mentions | @mention parsing | User enumeration, injection |
```

**Note:** When editing the README, add this as actual markdown (not in a code block).

#### 7c. Update Running Fuzzers Section

```bash
### Run all posts fuzzers
./scripts/fuzz/fuzz_posts.sh
```

### Step 8: Test Your Fuzzers

```bash
# Test individual fuzzer (quick)
cargo fuzz run fuzz_post_content -- -max_total_time=10

# Test all domain fuzzers
./scripts/fuzz/fuzz_posts.sh

# Verify CI configuration
act -j fuzz-posts  # Using 'act' to test GitHub Actions locally
```

---

## 📋 Quick Checklist (Copy This When Adding New Domain)

```
New Domain: _________________

[ ] Step 1: Decided what to fuzz (list targets)
[ ] Step 2: Created directory: fuzz/fuzz_targets/<domain>/
[ ] Step 3: Wrote fuzzer files (*.rs)
[ ] Step 4: Updated fuzz/Cargo.toml with [[bin]] entries
[ ] Step 5: Created scripts/fuzz/fuzz_<domain>.sh
[ ] Step 6: Updated .github/workflows/fuzzing.yml (new job)
[ ] Step 7: Updated fuzz/README.md:
    [ ] Structure section
    [ ] Domain table
    [ ] Running commands
[ ] Step 8: Tested fuzzers locally
[ ] Step 9: Updated scripts/fuzz/fuzz_all.sh (if exists)
[ ] Step 10: Committed all changes with descriptive message

Fuzzers Added:
- fuzz_________________
- fuzz_________________
- fuzz_________________
```

---

## 🎯 Tips for Effective Fuzzing

### Choose the Right Input Size

```bash
# Emails: small
-max_len=512

# Posts/Comments: medium
-max_len=10000

# File uploads: large
-max_len=1000000
```

### Balance Speed vs Coverage

```bash
# Quick smoke test (CI, local dev)
-max_total_time=60

# Deeper testing (nightly CI)
-max_total_time=300

# Intensive (weekend runs)
-max_total_time=3600 -jobs=4
```

### Prioritize Security-Critical Code

Focus fuzzing on:

1. **Authentication** (highest priority)
2. **User input parsing** (markdown, HTML)
3. **File uploads**
4. **Payment/financial operations**
5. **API endpoints with complex validation**

Skip fuzzing on:

- Simple CRUD operations
- Already property-tested validation
- Code using battle-tested parsers (html5ever, serde)

### Monitor Fuzzing Effectiveness

```bash
# Check code coverage
cargo fuzz coverage fuzz_target_name

# Minimize corpus (remove redundant test cases)
cargo fuzz cmin fuzz_target_name
```

---

## 🆘 Troubleshooting

### "Module declaration missing" in IDE

- **Solution:** Mark `fuzz/fuzz_targets/` as Excluded in IDE settings
- **Why:** Fuzz targets use `#![no_main]` and aren't normal modules

### Fuzzer runs but finds nothing

- **Check 1:** Ensure your domain code has `pub` visibility for fuzzer
- **Check 2:** Verify fuzzer is actually calling the parse function
- **Check 3:** Increase `-max_total_time` or `-max_len`

### CI fuzzing job fails

- **Check 1:** Ensure all paths in `fuzz/Cargo.toml` are correct
- **Check 2:** Verify `cargo-fuzz` installation in CI workflow
- **Check 3:** Check if crash artifacts were uploaded (maybe expected!)

### Out of memory during fuzzing

- **Solution:** Reduce `-max_len` or add memory limits:
  ```bash
  cargo fuzz run fuzzer -- -rss_limit_mb=2048
  ```

---

## 📊 Understanding Fuzzer Output

When you run a fuzzer (via `make fuzz-*`), you’ll see a lot of fast-scrolling output like this:

```
#423086 REDUCE cov: 412 ft: 2147 corp: 568/32Kb lim: 469 exec/s: 7051 rss: 470Mb L: 14/450 MS: 4 ChangeByte-CMP-CopyPart-EraseBytes-
#424948 DONE   cov: 412 ft: 2147 corp: 568/32Kb lim: 469 exec/s: 6966 rss: 470Mb
Done 424948 runs in 61 second(s)
==========================================
Authentication fuzzing complete!
All fuzzing completed successfully!
==========================================
```

### ✅ What “successful” output looks like

If the fuzzer finishes normally, you will see:

```
Done <N> runs in <X> second(s)
<your fuzzer name> complete!
All fuzzing completed successfully!
```

✅ Means: No crash, panic, or undefined behavior was found  
✅ Code survived thousands of random inputs  
✅ Coverage was collected and fuzzing ended cleanly

If **any crash occurs** you will **not** see “completed successfully” — instead you’ll get an immediate error like:

```
ERROR: libFuzzer: deadly signal
==12345==ERROR: AddressSanitizer: heap-buffer-overflow
panic: index out of bounds
```

So **if you see the “All fuzzing completed successfully!” line, it passed.**

---

## 🧠 What the scrolling output means

A typical status line contains several fields:

| Field                | Meaning                                         |
|----------------------|-------------------------------------------------|
| `#423086`            | How many fuzzing iterations have run            |
| `cov: 412`           | Code coverage (unique blocks reached)           |
| `corp: 568/32Kb`     | Corpus size: 568 saved test inputs, total 32 KB |
| `exec/s: 7051`       | Execution speed (tests per second)              |
| `rss: 470Mb`         | Memory (RAM) being used                         |
| `L: 14/450`          | Length of current input / max fuzz input length |
| `MS: ChangeByte-...` | Mutation strategies being applied               |

These lines are **normal internal progress updates from libFuzzer**.

You can safely ignore them unless you are optimizing fuzzing speed or coverage.

---

## 🔎 Detecting hangs vs. crashes

### ❌ Crash (panic, illegal memory access, etc.)

- Fuzzer stops immediately
- Shows panic or sanitizer error
- Writes a **crash file** into:  
  `./fuzz/artifacts/<fuzzer-name>/`

Example:

```
artifact_path: fuzz/artifacts/fuzz_user_email/crash-abc123
```

### ❗ Hang (infinite loop, blocking network call, etc.)

- Fuzzer **does not stop**, but shows very slow progress
- Coverage and corpus stop increasing
- `exec/s` drops close to `0`

You can detect a hang by pressing:

```
CTRL + C
```

and seeing:

```
==ERROR: libFuzzer: timeout after 60 seconds
```

To **force hang detection**, you can set a timeout:

```bash
make fuzz-single name=fuzz_user_email duration=60 timeout=5
```

(Timeout support optional — depends on your Makefile)

---

## 📦 About the “Recommended dictionary” block

At the end you may see:

```
###### Recommended dictionary. ######
"\375\377" # Uses: 1238
"\001\001\012\014" # Uses: 1044
...
###### End of recommended dictionary. ######
```

This is libFuzzer suggesting byte patterns it found useful.  
You can **ignore this**, or use them to improve future fuzzing runs — it's optional.

---


