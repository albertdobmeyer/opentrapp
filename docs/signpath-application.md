# SignPath Foundation application — values to submit

Copy-paste reference for the SignPath Foundation open-source code-signing
application. The supporting site is already live and verified
(`https://www.opentrapp.com/#download` mentions SignPath; `/privacy.html` is up).
**Submitting is a manual step on signpath.org — paste these in, tick the required
boxes, click submit.** See [`code-signing-policy.md`](code-signing-policy.md) for
the signing policy SignPath requires.

> Status: **SUBMITTED 2026-06-13 — pending SignPath review.** Next action is
> SignPath's; watch for their email. On approval, activate the `ci.yml` SignPath
> template (see "After approval" below).

---

## Fields

**Project Name**
```
opentrapp
```

**Repository URL**
```
https://github.com/albertdobmeyer/opentrapp
```

**Homepage URL**
```
https://www.opentrapp.com
```

**Download URL**  *(this page mentions SignPath — required)*
```
https://www.opentrapp.com/#download
```

**Privacy Policy URL**
```
https://www.opentrapp.com/privacy.html
```

**Wikipedia URL** — leave blank.

**Tagline**
```
An open-source desktop app that runs autonomous CLI agents inside a five-container security perimeter on your own computer.
```

**Description** *(≤300 characters — this version is 273)*
```text
OpenTrApp is an open-source desktop app that runs autonomous CLI agents inside a defense-in-depth security perimeter on your own computer — isolating the network, filtering egress, hiding credentials, and scanning skills — to contain a compromised or prompt-injected agent.
```

**Reputation**
```
OpenTrApp holds a passing OpenSSF Best Practices badge (project #12755) and publishes an OpenSSF Scorecard. Every release ships a CycloneDX SBOM, cosign keyless signatures, and SLSA build-provenance attestations, with CodeQL scanning on each commit. The project maintains a full public threat model, whitepaper, and reproducible-build recipe, and is developed entirely in the open. Links: https://github.com/albertdobmeyer/opentrapp and https://www.bestpractices.dev/projects/12755
```

**Maintainer Type**
```
Individual
```

**Build System**
```
GitHub Actions
```

**First Name**
```
Albert
```

**Last Name**
```
Dobmeyer
```

**Email** — the inbox SignPath creates the account under + sends notifications to. Use your preferred address (e.g. your GitHub/account email).

**Company Name** — leave blank (individual maintainer).

**Primary Discovery Channel** — your call: pick how you actually found SignPath
(e.g. "Google search", "GitHub"). Add specifics in the optional box if asked.

---

## Checkboxes

- ✅ **Required** — "I have read and agree to the SignPath Foundation Code of Conduct … certificates … may be revoked if terms are violated."
- ✅ **Required** — "I agree to allow SignPath to store and process my personal data."
- ⬜ Optional — "I agree to receive other communications from SignPath." (your preference)

---

## Note on the SignPath wording on the download page

The download page currently says *"free Windows code signing provided by the
SignPath Foundation's open-source program — rollout in progress."* This is
honest: **the installers are not signed yet.** If the SignPath reviewer asks for
unconditional present-tense ("is signed"), drop "rollout in progress" **only
after** the first signed release actually ships — don't claim signed before it is.

## After approval

Activate the Windows SignPath step that is already templated (commented) in
`.github/workflows/ci.yml`: SHA-pin the SignPath action, fill the
org/project/policy slugs from your approved account, add the `SIGNPATH_*` repo
secrets, and uncomment. Then tag a release and confirm the `.msi`/`.exe` show a
valid Authenticode signature.
