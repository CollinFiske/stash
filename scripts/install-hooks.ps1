# scripts/install-hooks.ps1
# Registers the .githooks/ directory as the git hooks path.
# Run once after cloning: .\scripts\install-hooks.ps1

$ErrorActionPreference = "Stop"

$root = Split-Path $PSScriptRoot -Parent

Push-Location $root

# Point git at our hooks directory (stored in the repo, not .git/)
git config core.hooksPath .githooks

# Make the hook executable on Unix (no-op on Windows but safe)
$hookFile = Join-Path $root ".githooks\pre-push"
if (Test-Path $hookFile) {
    # On Windows/Git Bash the file just needs to exist; chmod is advisory
    try { & bash -c "chmod +x .githooks/pre-push" } catch {}
}

Pop-Location

Write-Host ""
Write-Host "✅  Git hooks installed."
Write-Host "   The pre-push hook will run 'bun run validate' before every push."
Write-Host "   To skip it once (not recommended): git push --no-verify"
Write-Host ""
