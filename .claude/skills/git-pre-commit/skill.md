---
name: git-pre-commit
description: Create pre-commit hooks for automated code quality checks including formatting, linting, type checking, and tests. Use when setting up git hooks, CI/CD quality gates, or automated code validation.
---

- Core Principles
  - Fail fast with helpful feedback: exit non-zero on violations with clear error messages
  - Run only on staged files: check only what's being committed using git diff --cached --name-only --diff-filter=ACM
  - Keep it fast: target under 10 seconds, use parallel execution, skip expensive operations
  - Auto-fix when possible: format code automatically (prettier, black, rustfmt), stage fixes or prompt review

- Common Check Types
  - Formatting: auto-format with prettier, black, rustfmt, gofmt, stage formatted files or reject commit
  - Linting: run ESLint, pylint, clippy, golangci-lint on staged files only
  - Type checking: run tsc, mypy, or flow on changed files and dependencies (skip if slow, move to pre-push or CI)
  - Unit tests: run tests for changed files only (consider pre-push if suite is large)
  - Security checks: scan for secrets, credentials, API keys (detect-secrets, git-secrets, trufflehog)
  - Conventional commits: validate commit message format (feat:, fix:) in commit-msg hook

- Hook Setup Workflow
  - Choose framework: pre-commit (Python), husky (Node), or bash scripts in .git/hooks/
  - Identify checks for your stack: select language-specific formatters, linters, type checkers
  - Configure tools: create config files (.prettierrc, .eslintrc, pyproject.toml)
  - Write hook script: get staged files, run checks in parallel, report failures clearly
  - Test hook thoroughly: try commits that should pass and fail, verify performance
  - Document bypass method: add instructions for git commit --no-verify for emergency commits

- Multi-Language Projects
  - Run checks conditionally based on file extensions
  - Get staged files by type (grep for extensions)
  - Run appropriate checks for each language

- Performance Patterns
  - Parallel execution: run independent checks concurrently
  - Incremental checks: use linter caching (ESLint --cache, mypy --incremental)
  - Staged-only scope: never check entire repo
  - Defer expensive checks: move slow operations to pre-push or CI

- Hook Types by Stage
  - pre-commit: formatting, linting, quick tests, secret scanning
  - commit-msg: commit message validation (conventional commits, issue references)
  - pre-push: full test suite, type checking (if slow), build verification

- Best Practices
  - Exit with non-zero status on any failure to block commit
  - Print file paths and line numbers for violations
  - Suggest fix commands in error messages
  - Use colors for readability (red for errors, green for success)
  - Allow bypass with --no-verify but log/track usage
  - Version control hook configs (not .git/hooks scripts)
  - Test hooks work for all team members across different environments

- Common Tools
  - Frameworks: pre-commit, husky + lint-staged, lefthook
  - Formatters: prettier, black, rustfmt, gofmt, clang-format
  - Linters: ESLint, pylint, flake8, clippy, golangci-lint, rubocop
  - Secret scanning: detect-secrets, git-secrets, trufflehog, gitleaks
