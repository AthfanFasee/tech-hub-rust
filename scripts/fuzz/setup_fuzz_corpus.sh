#!/usr/bin/env bash
# scripts/init_fuzz_corpus.sh
# Creates seed corpus files and a dictionary for cargo-fuzz targets.
# - Safe to re-run: will NOT overwrite existing files unless FORCE=1 is set.
set -euo pipefail

# If you export FORCE=1 it will overwrite existing files.
FORCE="${FORCE:-0}"

# Helper to create directory if missing
mkd() {
  local d="$1"
  if [ ! -d "$d" ]; then
    mkdir -p "$d"
    echo "Created directory: $d"
  fi
}

# Helper to write file only if missing (or if FORCE=1)
write_file() {
  local path="$1"
  local content="$2"

  if [ -f "$path" ] && [ "$FORCE" != "1" ]; then
    echo "Skipping existing file: $path"
    return 0
  fi

  # Ensure parent dir exists
  mkdir -p "$(dirname "$path")"
  printf '%s\n' "$content" > "$path"
  echo "Wrote: $path"
}

# Root check - ensure we're in project root or fuzz exists (best-effort)
if [ ! -d "fuzz" ]; then
  echo "Note: 'fuzz' directory not found at current location. Creating 'fuzz' anyway."
  mkdir -p fuzz
fi

# List of targets (create corpus dirs for each)
TARGET_DIRS=(
  "fuzz/corpus/fuzz_register_json"
  "fuzz/corpus/fuzz_login_json"
  "fuzz/corpus/fuzz_post_json"
  "fuzz/corpus/fuzz_comment_json"
  "fuzz/corpus/fuzz_newsletter_json"
  "fuzz/corpus/fuzz_email_at_symbol_variations"
  "fuzz/corpus/fuzz_password_grapheme_counting"
  "fuzz/corpus/fuzz_user_email"
  "fuzz/corpus/fuzz_user_email_unicode"
  "fuzz/corpus/fuzz_user_password"
)

echo "Creating corpus directories..."
for d in "${TARGET_DIRS[@]}"; do
  mkd "$d"
done

echo
echo "Writing seed files (won't overwrite existing files unless FORCE=1)..."
echo

#
# JSON fuzzers seeds
#
write_file "fuzz/corpus/fuzz_register_json/valid.json" '{
  "email": "alice@example.com",
  "user_name": "alice",
  "password": "P@ssw0rd123"
}'

write_file "fuzz/corpus/fuzz_register_json/missing_fields.json" '{
  "email": "no-username@example.com"
}'

write_file "fuzz/corpus/fuzz_register_json/empty.json" '{}'

write_file "fuzz/corpus/fuzz_register_json/unicode-weird.json" '{
  "email": "weird\u200B\u202E@example.com",
  "user_name": "an\u0301a",
  "password": "\u0000\u0001\u007F"
}'

write_file "fuzz/corpus/fuzz_login_json/valid.json" '{
  "user_name": "alice",
  "password": "P@ssw0rd123"
}'

write_file "fuzz/corpus/fuzz_login_json/missing_fields.json" '{
  "password": "only-password"
}'

write_file "fuzz/corpus/fuzz_login_json/empty.json" '{}'

write_file "fuzz/corpus/fuzz_login_json/unicode-weird.json" '{
  "user_name": "bob\u200B",
  "password": "\u202Ereverse\u202E"
}'

write_file "fuzz/corpus/fuzz_post_json/valid.json" '{
  "title": "Hello world",
  "text": "This is a short post body that is valid.",
  "img": "https://example.com/image.jpg"
}'

write_file "fuzz/corpus/fuzz_post_json/missing_fields.json" '{
  "title": "Missing text & img"
}'

write_file "fuzz/corpus/fuzz_post_json/empty.json" '{}'

write_file "fuzz/corpus/fuzz_post_json/unicode-weird.json" '{
  "title": "Zero\u200BWidth\u200BTest",
  "text": "Combining char: e\u0301 and RTL \u202Eabc\u202E",
  "img": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAUA"
}'

write_file "fuzz/corpus/fuzz_comment_json/valid.json" '{
  "text": "Nice post!",
  "post_id": "550e8400-e29b-41d4-a716-446655440000"
}'

write_file "fuzz/corpus/fuzz_comment_json/missing_fields.json" '{
  "text": "Missing post_id"
}'

write_file "fuzz/corpus/fuzz_comment_json/empty.json" '{}'

write_file "fuzz/corpus/fuzz_comment_json/unicode-weird.json" '{
  "text": "Emoji\uD83C\uDF89\u200B\u0301",
  "post_id": "not-a-uuid"
}'

write_file "fuzz/corpus/fuzz_comment_json/uuid-valid-sample.json" '{
  "text": "Example with a different valid UUID",
  "post_id": "123e4567-e89b-12d3-a456-426614174000"
}'

write_file "fuzz/corpus/fuzz_newsletter_json/valid.json" '{
  "title": "Monthly Update",
  "content": {
    "html": "<p>Hello <strong>Subscriber</strong></p>",
    "text": "Hello Subscriber\nVisit our site."
  }
}'

write_file "fuzz/corpus/fuzz_newsletter_json/missing_fields.json" '{
  "title": "No content object"
}'

write_file "fuzz/corpus/fuzz_newsletter_json/empty.json" '{}'

write_file "fuzz/corpus/fuzz_newsletter_json/unicode-weird.json" '{
  "title": "Weird \u200B\u202E Title",
  "content": {
    "html": "<div>\u200B\u202E</div>",
    "text": "Combining: a\u0301\u0300"
  }
}'

#
# Email & password style fuzzers: seeds that exercise unicode, multiple @, edge cases
#
write_file "fuzz/corpus/fuzz_email_at_symbol_variations/valid_at_simple.json" '{
  "email": "user@example.com"
}'

write_file "fuzz/corpus/fuzz_email_at_symbol_variations/multiple_at.json" '{
  "email": "a@@example.com"
}'

write_file "fuzz/corpus/fuzz_email_at_symbol_variations/unicode_at.json" '{
  "email": "user\uFF20example.com"
}'

write_file "fuzz/corpus/fuzz_email_at_symbol_variations/zero_width_at.json" '{
  "email": "user\u200B@\u200Bexample.com"
}'

write_file "fuzz/corpus/fuzz_password_grapheme_counting/valid_simple.json" '{
  "password": "SimplePass123!"
}'

write_file "fuzz/corpus/fuzz_password_grapheme_counting/emoji_cluster.json" '{
  "password": "👨‍👩‍👧‍👦"
}'

write_file "fuzz/corpus/fuzz_password_grapheme_counting/combining.json" '{
  "password": "e\u0301\u0323"
}'

write_file "fuzz/corpus/fuzz_user_email/valid_basic.json" '{
  "email": "basic@example.com"
}'

write_file "fuzz/corpus/fuzz_user_email/unicode_homograph.json" '{
  "email": "exampIe@exampIe.com"
}'

write_file "fuzz/corpus/fuzz_user_email_unicode/partial_multibyte.json" '{
  "raw": "\u00E9\uFFFD"
}'

write_file "fuzz/corpus/fuzz_user_email_unicode/rtl_and_bidi.json" '{
  "email": "user\u202Eexample@domain.com"
}'

# shellcheck disable=SC2016
write_file "fuzz/corpus/fuzz_user_password/valid_password.json" '{
  "password": "My$ecureP@ssw0rd"
}'

write_file "fuzz/corpus/fuzz_user_password/long_padding.json" '{
  "password": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
}'

#
# dictionary (global)
#
if [ -f "fuzz/dictionary.txt" ] && [ "$FORCE" = "1" ]; then
  echo "fuzz/dictionary.txt already exists, skipping (use FORCE=1 to overwrite)"
else
  cat > fuzz/dictionary.txt <<'EOF'
"user_name"
"username"
"password"
"email"
"title"
"text"
"img"
"post_id"
"content"
"html"
"CRASH"
"submit"
"token"
"activation"
"confirm"
"register"
"login"
"newsletter"
"@"
"＠"
"admin"
"signup"
"confirm"
EOF
  echo "Wrote: fuzz/dictionary.txt"
fi

echo
echo "Corpus setup finished."
echo
echo "Notes:"
echo "- If libFuzzer already created corpus files in these directories, this script *did not* overwrite them (unless you ran with FORCE=1)."
echo "- Keep the auto-generated corpus produced by libFuzzer: it contains inputs found by the fuzzer and is usually valuable to keep."
echo "- The seeds written here are supplemental — they help the fuzzer reach meaningful JSON quickly."
echo "- If you want to replace existing seeds, run with: FORCE=1 ./scripts/init_fuzz_corpus.sh"
exit 0
