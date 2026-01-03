---
name: git-pre-commit
description: Create pre-commit hooks for automated code quality checks including formatting, linting, type checking, and tests. Use when setting up git hooks, CI/CD quality gates, or automated code validation.
---

# Git Pre-Commit Checks

## Core Principles

**Fail fast with helpful feedback**: Exit non-zero on violations with clear error messages showing exactly what failed and how to fix it.

**Run only on staged files**: Check only what's being committed using `git diff --cached --name-only --diff-filter=ACM`. Avoid checking entire codebase.

**Keep it fast**: Pre-commit hooks block the commit flow. Target under 10 seconds. Use parallel execution, incremental checks, and skip expensive operations (full test suites belong in CI).

**Auto-fix when possible**: Format code automatically (prettier, black, rustfmt) rather than just reporting errors. Stage fixes automatically or prompt user to review.

## Common Check Types

**Formatting**: Auto-format with prettier, black, rustfmt, gofmt. Stage formatted files or reject commit with fix instructions.

**Linting**: Run ESLint, pylint, clippy, golangci-lint on staged files only. Report violations with file and line numbers.

**Type checking**: Run tsc, mypy, or flow on changed files and their dependencies. Skip in pre-commit if slow (move to pre-push or CI).

**Unit tests**: Run tests for changed files only. Consider pre-push hook instead if test suite is large.

**Security checks**: Scan for secrets, credentials, API keys. Use tools like detect-secrets, git-secrets, or trufflehog.

**Conventional commits**: Validate commit message format (feat:, fix:, etc.) in commit-msg hook.

## Hook Setup Workflow

1. **Choose hook framework or manual**: Use pre-commit framework (Python), husky (Node), or write bash scripts in `.git/hooks/`
2. **Identify checks for your stack**: Select language-specific formatters, linters, type checkers
3. **Configure tools**: Create config files (.prettierrc, .eslintrc, pyproject.toml, etc.)
4. **Write hook script**: Get staged files, run checks in parallel where possible, report failures clearly
5. **Test hook thoroughly**: Try commits that should pass and fail, verify performance, ensure fixes work
6. **Document bypass method**: Add instructions for `git commit --no-verify` for emergency commits

## Multi-Language Projects

Run checks conditionally based on file extensions:

```bash
# Get staged files by type
js_files=$(git diff --cached --name-only --diff-filter=ACM | grep '\.js$\|\.ts$')
py_files=$(git diff --cached --name-only --diff-filter=ACM | grep '\.py$')

# Run appropriate checks
[[ -n "$js_files" ]] && npm run lint
[[ -n "$py_files" ]] && black --check $py_files
```

## Performance Patterns

**Parallel execution**: Run independent checks concurrently using background jobs or parallel command.

**Incremental checks**: Use linter caching (ESLint --cache, mypy --incremental) to skip unchanged files.

**Staged-only scope**: Never check entire repo, only files being committed.

**Defer expensive checks**: Move slow operations (full test suite, end-to-end tests) to pre-push or CI.

## Hook Types by Stage

**pre-commit**: Formatting, linting, quick tests, secret scanning

**commit-msg**: Commit message validation (conventional commits, issue references)

**pre-push**: Full test suite, type checking (if slow), build verification

## Best Practices

- Exit with non-zero status on any failure to block commit
- Print file paths and line numbers for violations
- Suggest fix commands in error messages
- Use colors for readability (red for errors, green for success)
- Allow bypass with --no-verify but log/track usage
- Version control hook configs (.pre-commit-config.yaml, package.json scripts) not .git/hooks scripts
- Test hooks work for all team members across different environments

## Common Tools

**Framework-based**: pre-commit (Python), husky + lint-staged (Node), lefthook (Go)

**Formatters**: prettier, black, rustfmt, gofmt, clang-format

**Linters**: ESLint, pylint, flake8, clippy, golangci-lint, rubocop

**Secret scanning**: detect-secrets, git-secrets, trufflehog, gitleaks
