# Translation Key Reminder

## Important Note for Future Development

When adding new components or functionality that includes user-facing text, always remember to:

1. **Add corresponding translation keys** to both `frontend/src/locales/en.json` and `frontend/src/locales/pt.json` files
2. **Use the `$t()` function** for all translatable strings in Vue templates and components
3. **Maintain consistency** in translation key naming conventions
4. **Test translations** in both English and Portuguese to ensure proper display

## Best Practices

- Translation keys should be descriptive and follow a logical hierarchy (e.g., `components.login.title`, `userSettings.language`)
- Keep translation files organized by category for easy maintenance
- Avoid hardcoding strings directly in components - always use the i18n system
- When adding new languages in the future, ensure all translation keys are present in the new language files

This reminder ensures consistent internationalization across the application and prevents missing translations that could break the user experience.