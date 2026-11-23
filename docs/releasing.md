# Releasing

GitHub Actions builds, checks and publishes everything. Nothing is published until a person
publishes a draft release, and each channel stays off until it is switched on.

## Workflows

| Workflow | Runs on | Does |
| --- | --- | --- |
| `ci.yml` | pushes to `main`, pull requests | fmt, clippy, the MSRV (1.89), the full test suite on Linux and macOS; on Windows a build, the tests that need no shell or Python, and a check that the graph is identical to Linux's; the example checker; the extension and adapter tests |
| `release.yml` | a pushed tag `vX.Y.Z` | builds `sdc` for six targets, the npm packages and the VS Code extension, then creates a **draft** release with all of them. Run by hand, it builds the same and creates nothing. |
| `publish.yml` | publishing that draft | publishes to each channel whose variable is on |

Targets: macOS arm64 and x64, Linux arm64 and x64 (static musl builds, so they run on any
distribution), Windows arm64 and x64. The Linux arm64 build uses GitHub's `ubuntu-24.04-arm`
runner. Check that your plan provides it; if it doesn't, change that row of the matrix.

## Cutting a release

1. Set the version in `cli/Cargo.toml`: `workspace.package.version` and the `version` of
   each `sdlang-*` path dependency. The release workflow fails if they differ from the tag.
   The npm packages and the extension take their version from the tag.
2. `cargo check` in `cli/` to update `Cargo.lock`, then commit.
3. Tag and push: `git tag v0.1.0 && git push origin v0.1.0`.
4. Review the draft release: archives, `SHA256SUMS`, installers, `sdc.rb`, the `.vsix` and
   `npm-packages.tar.gz`. Versions starting with `0.` are marked as pre-releases.
5. Publish the draft. `publish.yml` takes it from there.

## Channels

Set these under Settings → Secrets and variables → Actions. The publish jobs use the
`release` environment, so you can require an approval there.

| Channel | Variable | Secret | One-time setup |
| --- | --- | --- | --- |
| npm: `@sdlang/cli`, its six platform packages, `@sdlang/nextjs` | `PUBLISH_NPM=true` | `NPM_TOKEN` (an automation token). Once the packages exist you can drop the token and use trusted publishing. | Create the `sdlang` organisation on npm |
| crates.io: the `sdlang-*` crates (`cargo install sdlang-cli`) | `PUBLISH_CRATES=true` | `CARGO_REGISTRY_TOKEN` | A crates.io account |
| VS Code Marketplace | `PUBLISH_VSCODE=true` | `VSCE_PAT` | Create the publisher `sdlang` |
| Open VSX (Cursor, Windsurf, VSCodium) | `PUBLISH_OPENVSX=true` | `OVSX_PAT` | Create the namespace `sdlang` and sign the publisher agreement |
| Homebrew tap | `HOMEBREW_TAP=owner/homebrew-tap` | `HOMEBREW_TAP_TOKEN`, a token that can push to that repository | Create the tap repository |

The install scripts (`sdc-installer.sh`, `sdc-installer.ps1`) and the archives are served
from the release itself, so they work as soon as it is published.

## macOS signing

The binaries are not signed with an Apple Developer ID and not notarized. npm, the install
scripts, Homebrew and cargo do not quarantine what they download, so those installs run
without warnings. An archive downloaded with a browser is quarantined, and Gatekeeper
blocks it until the user runs `xattr -d com.apple.quarantine sdc`. The release notes say
so. To sign later: add the certificate and an App Store Connect API key as secrets, then
run `codesign` and `notarytool` in the macOS build jobs.

## Before the first public release

- Choose the GitHub organisation and the domain. The schemas' `$id` URLs use
  `sdlang.dev`, which belongs to someone else.
- Add `repository` to `cli/Cargo.toml` once the GitHub location is settled. The npm
  packages and the extension get theirs from the workflow.
