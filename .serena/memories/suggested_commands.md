# Suggested Commands

## Backend
```bash
# Build (Windows — nasm required for aws-lc-sys JWT backend)
AWS_LC_SYS_PREBUILT_NASM=1 cargo build

# Test all
cargo test

# Test specific package
cargo test --package server

# Check compilation without build artifacts
cargo check --package server

# Pedantic code checks
cargo clippy --package server --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core

# Automatically fix some pedantic code check issues
cargo clippy --fix --allow-dirty --workspace --all-targets -- -W clippy::all -W clippy::pedantic -W clippy::nursery -A clippy::missing_docs_in_private_items -A clippy::separated_literal_suffix -A clippy::implicit_return -A clippy::print_stderr -A clippy::exhaustive_enums -A clippy::exhaustive_structs -A clippy::single_char_lifetime_names -A clippy::missing_inline_in_public_items -A clippy::self_named_module_files -A clippy::wildcard_enum_match_arm -A clippy::pattern_type_mismatch -A clippy::std-instead-of-core

# Regenerate sqlx offline query cache (run after any sqlx query change)
DATABASE_URL=postgresql://user:pass@host:port/dbname cargo sqlx prepare --workspace
# Then commit the .sqlx/ directory

# Run with live DB
DATABASE_URL=postgresql://... cargo run
```

## Frontend
```bash
cd frontend
npm install
npm run dev        # Vite dev server
npm run build      # Type-check + build
npm run test:unit  # Vitest
npm run lint       # oxlint + eslint
```

## Git
```bash
git status
git log --oneline -10
```
