# .githooks

Pre-commit gates installed by Orellius' `bootstrap-project` script.
Tracked in git so every clone gets the same gates.

## Activate after cloning

```
git config core.hooksPath .githooks
```

(Run `bootstrap-project` to do this automatically.)

## Override (emergencies only)

```
PRE_COMMIT_SKIP=1 git commit ...
```

Do not use `--no-verify`. The global Claude Code hook blocks that flag.

## What runs

See `pre-commit` for the exact gates per detected stack.
