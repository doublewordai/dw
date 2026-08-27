# Windows package manifests

Manifests for distributing `dw` through Windows package managers.

> **These carry placeholder versions and hashes.** None of them can be finished
> until a release exists that includes the `dw-windows-amd64.exe` asset. See
> "Filling in a release" below.

| | Location | Publishing |
|---|---|---|
| Scoop | [`bucket/dw.json`](../bucket/dw.json) | Self-service, merging to `main` publishes it |
| Chocolatey | [`chocolatey/`](./chocolatey) | `choco push` + first-package moderation review |
| WinGet | [`winget/`](./winget) | Manual PR to `microsoft/winget-pkgs` for the first version |

## Filling in a release

Every manifest needs the version and the SHA256 of `dw-windows-amd64.exe`, taken
from `checksums.txt` on the release:

```bash
VERSION=0.1.26
curl -fsSL "https://github.com/doublewordai/dw/releases/download/v${VERSION}/checksums.txt" \
  | grep 'dw-windows-amd64\.exe$'
```

Then update, in each file, the version string and the hash placeholder
(`0000…0000`, or `REPLACE_WITH_SHA256_FROM_RELEASE_CHECKSUMS` for Chocolatey).
The WinGet installer manifest also needs a real `ReleaseDate`.

## Scoop

Because `bucket/` sits in this repository, it *is* a Scoop bucket, no second
repo required:

```powershell
scoop bucket add dw https://github.com/doublewordai/dw
scoop install dw
```

Two things to keep if you edit the manifest:

- The download URL ends in `#/dw.exe`. Scoop reads that as a rename, so the file
  lands as `dw.exe` and the command is `dw`. Without it you get
  `dw-windows-amd64`.
- `autoupdate.hash.regex` matches the `dw-windows-amd64.exe` line specifically,
  because `checksums.txt` lists every platform.

With `checkver` and `autoupdate` set, Scoop's own tooling can refresh later
versions instead of doing it by hand.

## Chocolatey

The binary is **downloaded at install time** from the GitHub release and
checksum-verified, rather than embedded in the package. This keeps a ~9 MB
binary per version out of git. Chocolatey auto-shims any `.exe` in `tools/`, so
`dw.exe` becomes `dw` on PATH.

```powershell
cd packaging/chocolatey
choco pack
choco install dw --source .          # test locally first
choco push dw.<version>.nupkg --source https://push.chocolatey.org/
```

`choco push` needs a `CHOCO_API_KEY`. A **first-time package goes through human
moderation**, which can take days, so budget for that before promising a date.

## WinGet

Three manifests, laid out under `manifests/d/Doubleword/dw/<version>/` in the
community repo. `InstallerType: portable` with `Commands: [dw]`, since `dw` is a
bare binary rather than an installer.

Validate and submit:

```powershell
winget validate --manifest packaging/winget
winget install --manifest packaging/winget   # local install test
```

Two constraints to know before the first submission:

- **`PackageIdentifier` is permanent once accepted.** It is currently
  `Doubleword.dw`. Change it now if `DoublewordAI.dw` is preferred.
- **The first version must be submitted by hand** as a PR to
  `microsoft/winget-pkgs`. Both `wingetcreate` and the `winget-releaser` action
  require a version to already exist in the community repo, so CI automation is
  only possible from the second release onward.

## Automating subsequent releases

Once each package exists upstream, `release-please.yml` can gain a job after
`publish-release` that bumps the Scoop manifest in-repo, runs `choco push`, and
invokes `winget-releaser`. Deliberately not wired up yet, since there is nothing to
update until the first manual submissions land.
