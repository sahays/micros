---
name: spec-driven-development
description:
  Write specifications before code using epics, stories, and tasks tracked in git issues. Use when planning features,
  documenting requirements, or structuring work before implementation. Emphasizes spec-first workflow with iterative
  refinement.
---

# Spec-Driven Development

## Core Principles

**Spec before code**: Write detailed specifications before implementation. Clarify requirements, edge cases, and
acceptance criteria upfront to reduce rework.

**Hierarchy of work**: Epics contain multiple stories, stories contain multiple tasks. Each level provides appropriate
detail for its scope.

**Track in git issues**: Use GitHub/GitLab issues with labels for epics, stories, and tasks. Built-in status tracking,
discussions, and easy commit references.

**Iterative refinement**: Start with high-level epic, break into stories, then detailed tasks. Refine based on feedback
and discovery.

## Issue Structure

**Epic** = Issue with `epic` label **Story** = Issue with `story` label, linked to parent epic **Task** = Issue with
`task` label, linked to parent story

**Labels**: Use `epic`, `story`, `task` for hierarchy. Add domain labels (frontend, backend, etc.) for categorization.

**Linking**: Reference parent in description (`Epic: #1`) or use platform features (GitHub task lists, GitLab
parent/child).

**Example hierarchy**:

```
Epic #1: User Authentication [epic]
├── Story #5: Login Flow [story] → "Part of #1"
│   ├── Task #10: Login form component [task] → "Part of #5"
│   ├── Task #11: Session management [task] → "Part of #5"
│   └── Task #12: Error handling [task] → "Part of #5"
└── Story #6: Password Reset [story] → "Part of #1"
    └── Task #13: Reset email service [task] → "Part of #6"
```

## Epic Issue Template

**Title**: [Epic Name] **Labels**: `epic`, priority label, domain label

```markdown
## Overview

[Business value and high-level description]

## Goals

- Primary objective
- Secondary objectives

## Scope

**In scope**: [What's included] **Out of scope**: [What's explicitly excluded]

## Success Metrics

- Measurable outcome 1
- Measurable outcome 2

## Stories

- [ ] #X Story name
- [ ] #X Story name

## Dependencies

[Reference other epics: #Y, systems, or prerequisites]
```

## Story Issue Template

**Title**: [Story Name] **Labels**: `story`, priority label, domain label

```markdown
**Epic**: #X

## User Story

As a [user type], I want [capability] so that [benefit].

## Acceptance Criteria

- [ ] Criterion 1 with specific, testable outcome
- [ ] Criterion 2 with specific, testable outcome

## Technical Approach

[High-level implementation strategy]

## Tasks

- [ ] #X Task name
- [ ] #X Task name

## Dependencies

[Required stories or prerequisites]
```

## Task Issue Template

**Title**: [Task Name] **Labels**: `task`, domain label

```markdown
**Story**: #X

## Description

[What needs to be done]

## Implementation Details

- Files/components to create or modify
- Key functions or classes needed
- Integration points

## Acceptance

- [ ] Completion criterion
- [ ] Test coverage
- [ ] Documentation if needed

## Effort

[Small/Medium/Large]
```

## Workflow

1. **Create epic issue**: Define business capability, goals, scope, add `epic` label
2. **Create story issues**: Identify user-facing features, add `story` label, reference epic (#X)
3. **Add story checklist to epic**: Update epic description with story task list
4. **Refine story**: Choose next story, add acceptance criteria and technical approach
5. **Create task issues**: Decompose story into concrete work items, add `task` label, reference story
6. **Add task checklist to story**: Update story description with task list
7. **Implement tasks**: Complete work, reference in commits (`Fixes #10`, `Relates to #11`)
8. **Close tasks**: Close task issues when complete
9. **Close story**: Close story when all tasks complete and acceptance criteria met
10. **Close epic**: Close epic when all stories complete

## Commit Strategy

**Reference issues in commits**: Use `Fixes #123`, `Closes #456`, or `Relates to #789` to auto-link commits

**Implement task commits**: `"Add login form component (Fixes #10)"`

**Partial progress commits**: `"WIP: session management (Relates to #11)"`

**Story completion commits**: `"Complete login flow feature (Closes #5)"`

Use GitHub/GitLab keywords (`Fixes`, `Closes`, `Resolves`) to auto-close issues when merged to main.

## Progressive Disclosure

**Start high-level**: Create epic with story placeholders initially

**Elaborate just-in-time**: Create detailed task issues when ready to implement that story

**Update as you learn**: Add comments to issues for discoveries, update descriptions for requirement changes

## Best Practices

- Make acceptance criteria specific and testable
- Keep tasks small (completable in one session)
- Update parent task lists when children complete
- Use milestones to group related epics or releases
- Assign issues when work begins
- Add comments for clarifications and progress updates
- Close issues promptly when complete
- Review epic/story issues before creating tasks to catch gaps
