#!/usr/bin/env bash

set -euo pipefail

bump_type="${1:-patch}"

if [[ "$bump_type" != "patch" && "$bump_type" != "minor" && "$bump_type" != "major" ]]; then
  printf 'Usage: %s [patch|minor|major]\n' "$0" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  printf 'Working tree is not clean. Commit or stash changes before releasing.\n' >&2
  exit 1
fi

current_version="$(node -e "const fs=require('fs');const pkg=JSON.parse(fs.readFileSync('package.json','utf8'));process.stdout.write(pkg.version);")"

next_version="$(node -e "const [version, bump] = process.argv.slice(1); const parts = version.split('.').map(Number); if (parts.length !== 3 || parts.some(Number.isNaN)) { throw new Error('Invalid semantic version'); } if (bump === 'major') { parts[0] += 1; parts[1] = 0; parts[2] = 0; } else if (bump === 'minor') { parts[1] += 1; parts[2] = 0; } else { parts[2] += 1; } process.stdout.write(parts.join('.'));" "$current_version" "$bump_type")"

if git rev-parse "v$next_version" >/dev/null 2>&1; then
  printf 'Tag v%s already exists.\n' "$next_version" >&2
  exit 1
fi

node -e "const fs=require('fs'); const pkgPath='package.json'; const pkg=JSON.parse(fs.readFileSync(pkgPath,'utf8')); pkg.version=process.argv[1]; fs.writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');" "$next_version"

node -e "const fs=require('fs'); const tauriPath='src-tauri/tauri.conf.json'; const cfg=JSON.parse(fs.readFileSync(tauriPath,'utf8')); cfg.version=process.argv[1]; fs.writeFileSync(tauriPath, JSON.stringify(cfg, null, 2) + '\n');" "$next_version"

node -e "const fs=require('fs'); const cargoPath='src-tauri/Cargo.toml'; const source=fs.readFileSync(cargoPath,'utf8'); const next=source.replace(/^version\s*=\s*\"[^\"]+\"$/m, 'version = \"' + process.argv[1] + '\"'); if (source === next) { throw new Error('Could not update src-tauri/Cargo.toml version'); } fs.writeFileSync(cargoPath, next);" "$next_version"

yarn build
(cd src-tauri && cargo clippy -- -D warnings -A dead-code)

git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore(release): v$next_version"
git tag -a "v$next_version" -m "v$next_version"
git push --follow-tags

printf 'Released v%s\n' "$next_version"
