# Publishing KUI

Only the component library is published to crates.io. Its package name is
`kense-kui`, its Rust library name is `kui`, and its license is MIT. Story hosts,
the website, CLI, catalog, build tool, and template remain unpublished.

## Prepare a version

1. Update `workspace.package.version` and the `workspace.dependencies.kui.version`
   requirement in the root `Cargo.toml`. Keep the dependency's `package = "kense-kui"`
   and `path = "registry"` so previews compile the same source.
2. Update the version examples in `registry/README.md` and `docs/PLATFORMS.md`.
   Keep component API documentation in `docs/items/` and bilingual website copy in
   `website/site/docs/` current. Library examples use `kui::`; copied source uses
   `crate::ui::`.
3. Regenerate public Markdown, registry JSON, `llms.txt`, and preview hashes:

   ```sh
   cargo run -p kui-build -- build
   cargo run -p kui-build -- verify-preview
   ```

4. Run the local gates in [AGENTS.md](../AGENTS.md), plus:

   ```sh
   cargo test -p kense-kui
   cargo doc -p kense-kui --no-default-features --features web --no-deps
   ./scripts/build-site.sh
   python3 scripts/check-site.py website/dist
   cargo publish -p kense-kui --dry-run --locked
   ```

   Commit the release changes before the final dry run. For an initial working
   tree check only, `--allow-dirty` permits validation before committing.
   Inspect the packaged source, README, MIT license, and normalized manifest.

## Publish and verify

Authenticate Cargo with a crates.io API token that permits publishing `kense-kui`.
Use Cargo's configured credential provider; never store tokens in the repository.

```sh
cargo publish -p kense-kui --locked
cargo owner --add github:kense-labs:crates-owners kense-kui
cargo owner --list kense-kui
```

`github:kense-labs:crates-owners` is the organization team that manages Kense Rust
crates. crates.io uses GitHub team owners for organizations; the publisher must be
a team member and crates.io must have permission to read that membership.

Verify the version and owners on [crates.io](https://crates.io/crates/kense-kui),
then test `cargo add kense-kui --rename kui --no-default-features` in an application
outside this workspace. Tag the release commit as `kense-kui-v<VERSION>` and push
the commit and tag. Published versions cannot be overwritten.

Pushing `main` runs the website deployment described in [README.md](../README.md).
Check both languages for the published package name, installation command, API
links, and source-copy alternative.
