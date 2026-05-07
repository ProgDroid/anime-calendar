# Legal Pre-Release Checklist

Placeholder pages are live at `/privacy` and `/terms`. Replace the stub content once
the sections below are complete.

---

## Privacy Policy (`/privacy`)

- [ ] **What data you collect** — email address, display name, Google OAuth identity (if used), IP address (rate limiting), payment method (Stripe — store only subscription status, not card details)
- [ ] **Why you collect it** — account creation, authentication, billing, calendar export delivery
- [ ] **How long you keep it** — specify retention period (e.g. account data deleted on request within 30 days)
- [ ] **Third-party processors** — list: Stripe (payments), Google (OAuth), hosting provider, AniList (data source — outbound only, no user data sent to AniList)
- [ ] **Cookie / session disclosure** — the app uses an httpOnly session cookie; describe its purpose and lifetime
- [ ] **User rights** — right to request data export or account deletion (reference the Danger Zone tab in account settings)
- [ ] **Contact address** — an email users can write to for data requests
- [ ] **GDPR coverage** — if any EU users: legal basis for processing (contract / legitimate interest), Data Controller identity
- [ ] **Effective date** — include the date the policy was last updated

---

## Terms of Service (`/terms`)

- [ ] **Acceptable use** — no scraping, no automated bulk exports, no sharing credentials
- [ ] **Paid tier / billing** — Stripe is the payment processor; describe the 14-day free trial, renewal behaviour, and how to cancel
- [ ] **Refund policy** — decide and state it (e.g. no refunds after 48 hours, or defer to Stripe's standard policy)
- [ ] **Account termination** — your right to suspend / terminate for abuse; user's right to delete their account at any time
- [ ] **Disclaimer / liability cap** — calendar data comes from AniList (third party); accuracy of airing schedules is not guaranteed
- [ ] **AniList data credit** — already in the footer; also mention it here ("anime data sourced from AniList — anilist.co")
- [ ] **Governing law** — specify jurisdiction (your country / state)
- [ ] **Contact address** — same as privacy policy
- [ ] **Effective date** — include the date the terms were last updated

---

## Tooling suggestions

Writing from scratch is slow and easy to get wrong. Faster options:

- **Termly** (`termly.io`) — generates GDPR-compliant Privacy Policy + ToS for small SaaS apps; free tier covers indie scale
- **iubenda** (`iubenda.com`) — similar generator, strong on EU/GDPR language; embeddable widget option if you later want to host content on their servers instead of the Vue pages
- **Bonterms** (`bonterms.com`) — open-source SaaS agreement templates if you prefer editing a plain document rather than using a generator

Once you have real text, replace the `legal.privacy.placeholder` and `legal.terms.placeholder` i18n keys in `frontend/src/locales/en.json` (and `pt.json`) with the actual content. For multi-section document content, replace the `<p>` tags in `PrivacyPolicyPage.vue` / `TermsOfServicePage.vue` with proper HTML prose sections or a rendered Markdown block.
