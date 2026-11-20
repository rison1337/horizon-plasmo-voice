# Publishing

Publish the flat `mod/` SDK project as an immutable `.hmod` GitHub release asset.
`tools/package_sdk.py` checks the artifact and produces its SHA-256 file.

1. Run engine, DSP/null-output, adapter and independent OpenSSL checks.
2. Run a real launcher build against the exact signed SDK; record the prebuilt
   route, build/toolchain identities and actual game-test scope in VALIDATION.md.
3. Check documentation, notices, source archives, CI and package contents.
4. Commit the source, create a version tag and publish the `.hmod` and checksum.
   Never replace published bytes; fixes receive a new version.
5. Check the current submission rules. As of 20 November 2026, the website form
   explicitly cannot accept SDK `.hmod` files. The supported route is a PR in
   [horizon-mods](https://github.com/horizon-rust/horizon-mods), with the release
   download link, entry, icon and exact-SDK build evidence.
6. Stage/check using the catalog's exact `.github/horizon-rev`. The registry's
   `stage` workflow downloads linked release assets and verifies their name/hash.
   Maintainer review and merge make the mod appear in the site/launcher catalog.

Prepared entry/icon live in `publishing/horizon-mods/`. Update metadata only with
actual artifact hashes and dates. The closed legacy PR #305 remains historical;
the SDK migration needs a new submission. A release alone is not catalog approval.

The original project artwork is in `branding/`; `mod/icon.png` and
`publishing/horizon-mods/icons/plasmo_voice.png` use the same icon.
GitHub's social preview is `branding/github-social.png`.
