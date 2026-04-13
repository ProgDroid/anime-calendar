#!/bin/bash
# i18n guard — warns when a Vue file may contain hardcoded user-visible strings
# Fires after Edit/Write/MultiEdit; self-filters to *.vue files only

json_input=$(cat)

if command -v jq &>/dev/null; then
    file_path=$(echo "$json_input" | jq -r '.tool_input.file_path // empty')
else
    file_path=$(echo "$json_input" | grep -o '"file_path"[[:space:]]*:[[:space:]]*"[^"]*"' \
        | sed 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')
fi

[[ -z "$file_path" || ! -f "$file_path" ]] && exit 0

# Only check Vue files
[[ "$file_path" != *.vue ]] && exit 0

# Extract the <template> section only (skip <script> and <style>)
template=$(sed -n '/<template/,/<\/template>/p' "$file_path")

findings=()

while IFS= read -r line; do
    [[ -z "$line" ]] && continue

    # Skip comment lines
    echo "$line" | grep -qE '<!--' && continue

    # Skip lines that already use $t(), $tc(), v-t=
    echo "$line" | grep -qE '\$t[ce]?\(|v-t=' && continue

    # Skip lines that are pure interpolation {{ ... }} with no bare text
    trimmed=$(echo "$line" | sed 's/^[[:space:]]*//' | sed 's/[[:space:]]*$//')
    [[ "$trimmed" =~ ^\{\{.*\}\}$ ]] && continue

    # Pattern 1: inline text nodes — >Some Text< (uppercase start, 3+ chars, no {{ inside)
    if echo "$line" | grep -qE '>[[:space:]]*[A-Z][a-zA-Z][a-zA-Z ]{1,}[[:space:]]*<'; then
        if ! echo "$line" | grep -qE '\{\{|\$t[ce]?\('; then
            findings+=("  $(echo "$line" | sed 's/^[[:space:]]*//')")
        fi
    fi

    # Pattern 2: v-bind with bare string literals: :prop="'Some Text'"
    if echo "$line" | grep -qE ':[a-z-]+="'"'"'[A-Z][a-zA-Z ]{2,}'"'"'"'; then
        findings+=("  $(echo "$line" | sed 's/^[[:space:]]*//')")
    fi

done <<< "$template"

if [[ ${#findings[@]} -gt 0 ]]; then
    echo "⚠️  [i18n] Possible hardcoded strings in $(basename "$file_path") — wrap with \$t():"
    for f in "${findings[@]}"; do
        echo "$f"
    done
fi

exit 0
