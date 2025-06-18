# Git Hooks for Rossby

This directory contains Git hooks for the Rossby project that help enforce code quality before commits are made.

## Available Hooks

- **pre-commit**: Runs before a commit is created, verifying that all tests pass, the code compiles, and meets the project's formatting and linting standards.

## How to Install

Git hooks are not automatically transferred when a repository is cloned. You need to manually set them up:

### Option 1: Symlink the entire hooks directory

```bash
# From the project root directory
rm -rf .git/hooks
ln -s $(pwd)/hooks .git/hooks
```

### Option 2: Copy individual hooks

```bash
# From the project root directory
cp hooks/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit
```

### Option 3: Use core.hooksPath

You can tell Git to use the hooks in this directory by setting the `core.hooksPath` configuration:

```bash
git config core.hooksPath ./hooks
```

## What the pre-commit Hook Does

The pre-commit hook performs these checks:

1. `cargo check` - Verifies the code compiles without errors
2. `cargo test` - Runs all tests to ensure they pass
3. `cargo clippy` - Performs static analysis to catch common mistakes
4. `cargo fmt --check` - Ensures code adheres to formatting standards

If any of these checks fail, the commit will be aborted, giving you a chance to fix the issues before committing.

## Skipping Hooks

In rare cases when you need to bypass the pre-commit hook (not recommended), you can use:

```bash
git commit --no-verify
```

However, the CI checks will still run when you push, so it's generally better to fix the issues rather than bypass the hooks.
