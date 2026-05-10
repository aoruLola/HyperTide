# HyperTide CLI Packaging

This directory contains local CLI packaging entrypoints for `ht`. Release maintainers can use the generated archives as GitHub Release assets.

## Windows package

```powershell
powershell -ExecutionPolicy Bypass -File .\deploy\cli\package.ps1
```

Output:

- `deploy/cli/dist/hypertide-cli-<version>-windows-x86_64.zip`

## Linux package

```bash
bash ./deploy/cli/package.sh
```

Output:

- `deploy/cli/dist/hypertide-cli-<version>-linux-x86_64.tar.gz`

## What gets packaged

- `ht` only
- no server binary
- no Docker image

## Notes

- Prefer the signed or checksum-published artifacts on [GitHub Releases](https://github.com/openLYURA/HyperTide/releases) for normal installation.
- These scripts are for local packaging and release artifact preparation.
- They build from the workspace root but package only the CLI artifact.
- The output directory is ignored by git.
