# HyperTide CLI Packaging

This directory contains CLI-only packaging entrypoints for `ht`.

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

- These scripts are for internal distribution and deployment convenience.
- They build from the workspace root but package only the CLI artifact.
- The output directory is ignored by git.
