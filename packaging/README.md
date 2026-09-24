# Windows package manifests

Manifests for distributing `dw` through Windows package managers.

> **None of these can be finished until a release exists that includes the
> `dw-windows-amd64.exe` asset.** See "Filling in a release" below. The
> Chocolatey and WinGet manifests carry placeholder versions and hashes in the
> meantime — safe to leave, since publishing either one is a deliberate manual
> step (`choco push`, a PR to `microsoft/winget-pkgs`). `bucket/dw.json` is
> different: the moment it exists on `main`, `scoop bucket add` picks it up, so
> it is intentionally *not* checked in yet — create it fresh once real values
> exist, using the template under "Scoop" below.

| | Location | Publishing |
|---|---|---|
| Scoop | `bucket/dw.json` (create from the template below) | Self-service — merging to `main` publishes it immediately |
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

For Chocolatey and WinGet, update the version string and the hash placeholder
(`0000…0000`, or `REPLACE_WITH_SHA256_FROM_RELEASE_CHECKSUMS`) in the existing
files. The WinGet installer manifest also needs a real `ReleaseDate`. For
Scoop, create `bucket/dw.json` from the template below — do not add it before
real values are available.

## Scoop

Because `bucket/` sits in this repository, it *is* a Scoop bucket, no second
repo required, once `bucket/dw.json` exists:

```powershell
scoop bucket add dw https://github.com/doublewordai/dw
scoop install dw
```

Create `bucket/dw.json` with the real version and hash filled in:

```json
{
    "version": "0.1.26",
    "description": "Doubleword Batch Inference CLI. Upload JSONL files, run batches, stream results, and send real-time inference requests.",
    "homepage": "https://github.com/doublewordai/dw",
    "license": "MIT",
    "architecture": {
        "64bit": {
            "url": "https://github.com/doublewordai/dw/releases/download/v0.1.26/dw-windows-amd64.exe#/dw.exe",
            "hash": "<sha256 from checksums.txt>"
        }
    },
    "bin": "dw.exe",
    "checkver": {
        "github": "https://github.com/doublewordai/dw"
    },
    "autoupdate": {
        "architecture": {
            "64bit": {
                "url": "https://github.com/doublewordai/dw/releases/download/v$version/dw-windows-amd64.exe#/dw.exe"
            }
        },
        "hash": {
            "url": "https://github.com/doublewordai/dw/releases/download/v$version/checksums.txt",
            "regex": "$sha256\\s+dw-windows-amd64\\.exe"
        }
    }
}
```

Two things to keep if you edit the manifest:

- The download URL ends in `#/dw.exe`. Scoop reads that as a rename, so the file
  lands as `dw.exe` and the command is `dw`. Without it you get
  `dw-windows-amd64.exe`.
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
  `microsoft/winget-pkgs`. `wingetcreate new` can generate and submit it, but no
  automated CI (the `winget-releaser` action needs an existing upstream version) can, so CI
  automation is only possible from the second release onward.

## Automating subsequent releases

Once each package exists upstream, `release-please.yml` can gain a job after
`publish-release` that bumps the Scoop manifest in-repo, runs `choco push`, and
invokes `winget-releaser`. Deliberately not wired up yet, since there is nothing to
update until the first manual submissions land.
