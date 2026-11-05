# Contributing

Read README.md, PARITY.md and VALIDATION.md before changing compatibility claims.
Use the commands in docs/DEVELOPMENT.md. Engine tests and formatting run in CI;
the full adapter needs the matching Horizon source and separate validation.

Write runtime code in Rust. Keep audio/network work on the existing worker and
bound queues and caches. Keep new UI text in both translation files. Treat PTT
release, focus loss, lifecycle cleanup and independent player gain as behavior
that must survive refactors.

Use permissively licensed references and original code/artwork. Do not read or
copy the Plasmo Voice implementation or Minecraft textures into this project.
Credit every protocol reference and retained runtime dependency in
THIRD_PARTY_NOTICES.md. Do not commit proprietary game resources, recordings,
credentials, sessions, build outputs or test homes.

Changes should say what behavior changed, why, and which meaningful checks were
run. Avoid stating Windows device support based solely on cross-compilation.
