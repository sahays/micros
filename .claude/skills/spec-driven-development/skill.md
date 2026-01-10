---
name: spec-driven-development
description:
  Write specifications before code using epics, stories, and tasks tracked in git issues. Use when planning features,
  documenting requirements, or structuring work before implementation. Emphasizes spec-first workflow with iterative
  refinement.
---

- Core Principles
  - Spec before code: write detailed specifications before implementation
  - Hierarchy: epics contain stories, stories contain tasks
  - Track in git issues: use GitHub/GitLab with labels (epic, story, task)
  - Iterative refinement: start high-level, break into stories, then detailed tasks

- Issue Structure
  - Epic: issue with epic label
  - Story: issue with story label, linked to parent epic
  - Task: issue with task label, linked to parent story
  - Labels: epic, story, task for hierarchy; add domain labels (frontend, backend)
  - Linking: reference parent in description (Epic: #1) or use platform task lists

- Epic Template
  - Title: Epic Name
  - Labels: epic, priority, domain
  - Overview: business value and high-level description
  - Goals: primary and secondary objectives
  - Scope: in scope and out of scope
  - Success Metrics: measurable outcomes
  - Stories: checklist of story issues
  - Dependencies: reference other epics, systems, or prerequisites

- Story Template
  - Title: Story Name
  - Labels: story, priority, domain
  - Epic: reference parent epic (#X)
  - User Story: As a [user type], I want [capability] so that [benefit]
  - Acceptance Criteria: specific, testable outcomes
  - Technical Approach: high-level implementation strategy
  - Tasks: checklist of task issues
  - Dependencies: required stories or prerequisites

- Task Template
  - Title: Task Name
  - Labels: task, domain
  - Story: reference parent story (#X)
  - Description: what needs to be done
  - Implementation Details: files/components, functions/classes, integration points
  - Acceptance: completion criteria, test coverage, documentation
  - Effort: Small/Medium/Large

- Workflow
  - Create epic issue: define capability, goals, scope, add epic label
  - Create story issues: identify features, add story label, reference epic
  - Add story checklist to epic description
  - Refine story: add acceptance criteria and technical approach
  - Create task issues: decompose story, add task label, reference story
  - Add task checklist to story description
  - Implement tasks: complete work, reference in commits (Fixes #10, Relates to #11)
  - Close tasks when complete
  - Close story when all tasks complete and acceptance criteria met
  - Close epic when all stories complete

- Commit Strategy
  - Reference issues: Fixes #123, Closes #456, Relates to #789
  - Task commits: Add login form component (Fixes #10)
  - Partial progress: WIP: session management (Relates to #11)
  - Story completion: Complete login flow feature (Closes #5)
  - Use keywords (Fixes, Closes, Resolves) to auto-close when merged to main

- Progressive Disclosure
  - Start high-level: create epic with story placeholders
  - Elaborate just-in-time: create detailed task issues when ready to implement
  - Update as you learn: add comments for discoveries, update descriptions for changes

- Best Practices
  - Make acceptance criteria specific and testable
  - Keep tasks small (completable in one session)
  - Update parent task lists when children complete
  - Use milestones to group related epics or releases
  - Assign issues when work begins
  - Add comments for clarifications and progress updates
  - Close issues promptly when complete
  - Review epic/story issues before creating tasks to catch gaps
