#!/bin/bash
# sqlx prepare reminder — reminds to regenerate .sqlx/ after editing sqlx query macros
# Fires after Edit/Write/MultiEdit; self-filters to *.rs files only

json_input=$(cat)

if command -v jq &>/dev/null; then
    file_path=$(echo "$json_input" | jq -r '.tool_input.file_path // empty')
else
    file_path=$(echo "$json_input" | grep -o '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | sed 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
fi

[[ -z "$file_path" || ! -f "$file_path" ]] && exit 0

# Only check Rust files
[[ "$file_path" != *.rs ]] && exit 0

# Check for any sqlx query macros
if grep -qE 'sqlx::(query|query_as|query_scalar|query_unchecked)|query(_as|_scalar|_unchecked)?!' "$file_path"; then
    echo "ℹ️  [sqlx] This file uses sqlx query macros."
    echo "   If you changed a query, regenerate the offline cache:"
    echo "   DATABASE_URL=postgresql://user:pass@host/db cargo sqlx prepare --workspace"
    echo "   Then commit the updated .sqlx/ directory."
fi

exit 0
