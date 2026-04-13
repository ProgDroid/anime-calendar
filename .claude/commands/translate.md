---
description: Add an i18n translation key to both en.json and pt.json locale files simultaneously
argument-hint: <dot.notation.key> "<english text>" "<portuguese text>"
allowed-tools: Read, Edit
---

Add the translation key from $ARGUMENTS to both locale files in `frontend/src/locales/`.

## Parse Arguments

The arguments are: `<key> "<english>" "<portuguese>"`

- **Key**: dot-notation path, e.g. `auth.login.newButton` or `calendar.header.title`
- **English**: the English string (may be quoted)
- **Portuguese**: the Portuguese string (may be quoted)

## Steps

1. Read `frontend/src/locales/en.json`
2. Read `frontend/src/locales/pt.json`
3. For each file:
   - Navigate the nested structure using the dot-notation key
   - If an intermediate object doesn't exist, create it
   - Insert the leaf key with the appropriate text value
   - Preserve existing alphabetical ordering within each object
4. Write both files using Edit (not Write — preserve existing content)
5. Confirm: output the key and both translations as a summary

## Key Format Examples

- `auth.login.newButton` → insert `"newButton": "..."` inside `en.json > auth > login`
- `errors.network` → insert `"network": "..."` inside `en.json > errors`
- `calendar.filter.label` → insert `"label": "..."` inside `en.json > calendar > filter`

If the parent namespace doesn't exist in a file, create it as an empty object and then add the key.

Never overwrite existing keys — if the key already exists in either file, stop and report the conflict.
